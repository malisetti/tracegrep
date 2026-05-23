//! Integration tests for `follow_path`: async tail with ordering under a capped runtime.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tracegrep::follow::testing as follow_testing;
use tracegrep::output::Formatter;
use tracegrep::query::parse;
use tracegrep::record::{Record, Value};
use tracegrep::{follow_path, follow_paths};

#[derive(Clone)]
struct CollectSeq(Arc<Mutex<Vec<i64>>>);

impl Formatter for CollectSeq {
    fn write(&mut self, r: &Record) -> std::io::Result<()> {
        let Some(seq) = r.get("seq") else {
            return Ok(());
        };
        let Value::Int(n) = seq else {
            return Ok(());
        };
        self.0.lock().unwrap().push(*n);
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

async fn append_json_lines(path: &std::path::Path, payloads: &[&str]) {
    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .await
        .expect("append open");
    for line in payloads {
        file.write_all(line.as_bytes()).await.expect("write");
        file.flush().await.expect("flush");
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn follow_emits_matches_in_sequence() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path: PathBuf = dir.path().join("trail.jsonl");

    // Seed format sniff + EOF position for follower.
    {
        let mut f = tokio::fs::File::create(&path).await.unwrap();
        f.write_all(b"{}\n").await.unwrap();
        f.flush().await.unwrap();
    }

    let collected = Arc::new(Mutex::new(Vec::<i64>::new()));
    let follower_collected = Arc::clone(&collected);
    let path_follow = path.clone();

    let follow_fut = async move {
        let expr =
            parse("seq = 1 or seq = 2 or seq = 3").map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let mut fmt = CollectSeq(follower_collected);
        follow_path(&path_follow, &expr, &mut fmt).await
    };

    let write_fut = async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        append_json_lines(&path, &["{\"seq\":1}\n", "{\"seq\":2}\n", "{\"seq\":3}\n"]).await;
    };

    let (follow_outcome, ()) = tokio::join!(
        tokio::time::timeout(Duration::from_millis(500), follow_fut),
        write_fut,
    );

    assert!(
        follow_outcome.is_err(),
        "follow should run until cancelled by timeout",
    );

    let got = collected.lock().unwrap().clone();
    assert_eq!(got, vec![1, 2, 3], "expected ordered matches, got {got:?}");
}

struct DisableFollowBannerCapture;

impl Drop for DisableFollowBannerCapture {
    fn drop(&mut self) {
        follow_testing::disable_banner_capture();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn follow_paths_orders_multiplex_reads_and_emit_tail_banners() {
    let _cleanup = DisableFollowBannerCapture;

    let dir = tempfile::tempdir().expect("tmpdir");
    let path_a: PathBuf = dir.path().join("alpha.jsonl");
    let path_b: PathBuf = dir.path().join("beta.jsonl");

    for path in &[path_a.clone(), path_b.clone()] {
        let mut f = tokio::fs::File::create(path).await.unwrap();
        f.write_all(b"{}\n").await.unwrap();
        f.flush().await.unwrap();
    }

    follow_testing::reset_and_enable_banner_capture();

    let collected = Arc::new(Mutex::new(Vec::<i64>::new()));
    let paths = vec![path_a.clone(), path_b.clone()];
    let follower_collected = Arc::clone(&collected);

    let follow_fut = async move {
        let expr = parse("seq = 100 or seq = 101 or seq = 102")
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let mut fmt = CollectSeq(follower_collected);
        follow_paths(&paths, &expr, &mut fmt).await
    };

    let wa = path_a.clone();
    let wb = path_b.clone();
    let write_fut = async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let l100 = "{\"seq\":100}\n";
        let l101 = "{\"seq\":101}\n";
        let l102 = "{\"seq\":102}\n";

        tokio::join!(
            async move {
                tokio::time::sleep(Duration::from_millis(5)).await;
                append_json_lines(&wa, &[l100]).await;
                tokio::time::sleep(Duration::from_millis(250)).await;
                append_json_lines(&wa, &[l102]).await;
            },
            async move {
                tokio::time::sleep(Duration::from_millis(80)).await;
                append_json_lines(&wb, &[l101]).await;
            },
        );
    };

    let follow_task = tokio::spawn(follow_fut);

    write_fut.await;

    tokio::time::sleep(Duration::from_millis(150)).await;
    follow_task.abort();
    match follow_task.await {
        Err(e) if e.is_cancelled() => {}
        Ok(Ok(())) => panic!("follow_paths exited early unexpectedly with Ok"),
        Ok(Err(e)) => panic!("follow_paths exited early unexpectedly with Err: {e:#}"),
        Err(e) => panic!("follow task aborted with unexpected error: {e}"),
    }

    let banners = follow_testing::captured_banners();
    assert!(
        banners.iter().any(|b| b.contains("alpha.jsonl")),
        "expected banner for alpha.jsonl in {banners:?}",
    );
    assert!(
        banners.iter().any(|b| b.contains("beta.jsonl")),
        "expected banner for beta.jsonl in {banners:?}",
    );

    let got = collected.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![100, 101, 102],
        "expected multiplex order to mirror write timings, got {got:?}",
    );
}
