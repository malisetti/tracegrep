//! Streaming follow mode (`--follow`): tail a file line-by-line with polling at EOF.

use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom};

use crate::output::Formatter;
use crate::query::Expr;
use crate::{parse_auto, sniff, Format};

/// Resolve line format using the **first non-empty line** without consuming the tail cursor.
pub async fn sniff_line_format(path: &Path) -> anyhow::Result<Format> {
    let mut f = File::open(path)
        .await
        .with_context(|| format!("open {} for sniff", path.display()))?;
    let mut reader = BufReader::new(&mut f);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .with_context(|| format!("read {} for sniff", path.display()))?;
        if n == 0 {
            return Ok(Format::Plain);
        }
        if line.trim().is_empty() {
            continue;
        }
        return Ok(sniff(&line));
    }
}

/// Follow `path` from **end of file**, polling every 100ms at EOF. Reopens when the file shrinks.
pub async fn follow_path<F: Formatter>(
    path: &Path,
    expr: &Expr,
    fmt: &mut F,
) -> anyhow::Result<()> {
    let line_fmt = sniff_line_format(path).await?;
    follow_path_with_format(path, expr, line_fmt, fmt).await
}

/// Same as [`follow_path`], using an explicit detected or CLI-selected [`Format`] (skips sniff).
pub async fn follow_path_with_format<F: Formatter>(
    path: &Path,
    expr: &Expr,
    line_fmt: Format,
    fmt: &mut F,
) -> anyhow::Result<()> {
    let mut file = File::open(path)
        .await
        .with_context(|| format!("open {} for follow", path.display()))?;
    let mut tracked_len = file.metadata().await?.len();
    file.seek(SeekFrom::End(0))
        .await
        .with_context(|| format!("seek end {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut lineno: usize = 0;

    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .await
            .with_context(|| format!("read_line {}", path.display()))?;

        if n == 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let meta = tokio::fs::metadata(path)
                .await
                .with_context(|| format!("stat {}", path.display()))?;
            if meta.len() < tracked_len {
                let mut new_file = File::open(path)
                    .await
                    .with_context(|| format!("reopen {} after truncation", path.display()))?;
                tracked_len = new_file.metadata().await?.len();
                new_file
                    .seek(SeekFrom::End(0))
                    .await
                    .with_context(|| format!("seek end after truncate {}", path.display()))?;
                reader = BufReader::new(new_file);
                continue;
            }
            tracked_len = meta.len();
            continue;
        }

        lineno += 1;
        if line.trim().is_empty() {
            continue;
        }

        let record = parse_auto(line.trim_end(), line_fmt)
            .map_err(|e| anyhow::anyhow!("line {}: {}", lineno, e))?;
        if expr.eval(&record) {
            fmt.write(&record)
                .map_err(|e| anyhow::anyhow!("format write: {e}"))?;
        }
    }
}
