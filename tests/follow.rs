//! Integration tests for `follow_path`: async tail with ordering under a capped runtime.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tracegrep::follow_path;
use tracegrep::output::Formatter;
use tracegrep::query::parse;
use tracegrep::record::{Record, Value};

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
    let mut opts = tokio::fs::OpenOptions::new();
    let mut file = opts.append(true).open(path).await.expect("append open");
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
