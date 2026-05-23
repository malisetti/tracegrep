//! Streaming follow mode (`--follow`): tail a file line-by-line with polling at EOF.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom};

use crate::diagnostics::DiagnosticsSink;
use crate::output::Formatter;
use crate::parser::resilience::{
    cli_hint_from_format, parse_cli_line, CliInputHint, ParsedCliLine,
};
use crate::query::Expr;
use crate::record::Record;
use crate::{sniff, Format};

pub mod testing {
    //! Helpers for integration tests (`==> … <==` banners normally go straight to stdout).

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    static CAPTURE_ENABLED: AtomicBool = AtomicBool::new(false);
    static BANNERS: Mutex<Vec<String>> = Mutex::new(Vec::new());

    /// Start recording `follow` multiplex banners instead of printing them on stdout.
    pub fn reset_and_enable_banner_capture() {
        let mut guard = BANNERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.clear();
        CAPTURE_ENABLED.store(true, Ordering::SeqCst);
    }

    pub fn disable_banner_capture() {
        CAPTURE_ENABLED.store(false, Ordering::SeqCst);
    }

    pub(super) fn capture_enabled() -> bool {
        CAPTURE_ENABLED.load(Ordering::SeqCst)
    }

    pub(super) fn push_banner(line: impl Into<String>) {
        if !capture_enabled() {
            return;
        }
        let mut guard = BANNERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.push(line.into());
    }

    pub fn captured_banners() -> Vec<String> {
        BANNERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// How each tailed physical line should be parsed (CLI `--input` analogue).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FollowParseSpec {
    /// Per-line sniff + parse (`--input auto` batch semantics).
    AutoPerLine,
    Fixed(Format),
}

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

/// Emit a `tail -f`-style banner before the next formatter line when multiplexing outputs.
///
/// Callers **must not** pair this with [`Formatter`] wrappers that permanently hold [`std::io::Stdout`]’s
/// global outer lock (`io::stdout().lock()` held across awaited work); prefixes take a temporary lock
/// and would deadlock otherwise. Passing [`std::io::Stdout`]/`&Stdout`/`&mut Stdout`-like writers into
/// formatters avoids that.
#[inline]
pub fn emit_follow_source_banner(path: &Path) -> io::Result<()> {
    let line = format!("==> {} <==", path.display());
    if testing::capture_enabled() {
        testing::push_banner(line);
        return Ok(());
    }
    writeln!(io::stdout().lock(), "{line}")?;
    Ok(())
}

/// Follow **`paths`** concurrently from **EOF**, multiplexing readiness with Tokio scheduling.
///
/// With **exactly one** path, behaves like [`follow_path`] (`tail`-style banners are suppressed).
///
/// With **two or more** paths, emits `==> path <==` (via [`emit_follow_source_banner`]) before each
/// matched record is written — same convention as BSD `tail -f` switching between files when several
/// independent writers append concurrently, **ordering reflects whichever read wakes first**.
pub async fn follow_paths<F: Formatter>(
    paths: &[PathBuf],
    expr: &Expr,
    fmt: &mut F,
) -> anyhow::Result<()> {
    match paths.len() {
        0 => anyhow::bail!("follow_paths requires at least one path"),
        1 => follow_path(paths[0].as_path(), expr, fmt).await,
        _ => follow_paths_many(paths, expr, fmt).await,
    }
}

/// Follow `path` from **end of file**, polling every 100ms at EOF. Reopens when the file shrinks.
pub async fn follow_path<F: Formatter>(
    path: &Path,
    expr: &Expr,
    fmt: &mut F,
) -> anyhow::Result<()> {
    let line_fmt = sniff_line_format(path).await?;
    let mut silent_match = false;
    let mut sink = io::sink();
    let mut silent = DiagnosticsSink::new(&mut sink);
    follow_tail(
        path,
        expr,
        fmt,
        FollowParseSpec::Fixed(line_fmt),
        true,
        Some(&mut silent),
        &mut silent_match,
    )
    .await
}

/// Same as [`follow_path`], using an explicit detected or CLI-selected [`Format`] (skips sniff).
pub async fn follow_path_with_format<F: Formatter>(
    path: &Path,
    expr: &Expr,
    line_fmt: Format,
    fmt: &mut F,
) -> anyhow::Result<()> {
    let mut silent_match = false;
    let mut sink = io::sink();
    let mut silent = DiagnosticsSink::new(&mut sink);
    follow_tail(
        path,
        expr,
        fmt,
        FollowParseSpec::Fixed(line_fmt),
        true,
        Some(&mut silent),
        &mut silent_match,
    )
    .await
}

/// Tail-follow `path` with [`parse_cli_line`] semantics (--strict/--no-strict diagnostics).
pub async fn follow_tail<F: Formatter, W: io::Write>(
    path: &Path,
    expr: &Expr,
    fmt: &mut F,
    parse_spec: FollowParseSpec,
    strict: bool,
    mut diag: Option<&mut DiagnosticsSink<W>>,
    any_match: &mut bool,
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

    let cli_hint = match parse_spec {
        FollowParseSpec::AutoPerLine => CliInputHint::Auto,
        FollowParseSpec::Fixed(format) => cli_hint_from_format(format),
    };

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

        match parse_cli_line(line.trim_end(), lineno as u64, cli_hint, strict, &mut diag)? {
            ParsedCliLine::Ignored => {}
            ParsedCliLine::Record(ref record) if expr.eval(record) => {
                *any_match = true;
                fmt.write(record)
                    .map_err(|e| anyhow::anyhow!("format write: {e}"))?;
            }
            ParsedCliLine::Record(_) => {}
        }
    }
}

async fn follow_paths_many<F: Formatter>(
    paths: &[PathBuf],
    expr: &Expr,
    fmt: &mut F,
) -> anyhow::Result<()> {
    let (dispatch_tx, mut dispatch_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut join_set = tokio::task::JoinSet::new();

    for path in paths {
        let path = path.clone();
        let expr = expr.clone();
        let tx = dispatch_tx.clone();
        join_set.spawn(async move {
            let line_fmt = sniff_line_format(path.as_path()).await?;

            struct MatchForwarder {
                sender: tokio::sync::mpsc::UnboundedSender<(PathBuf, Record)>,
                path: PathBuf,
            }

            impl Formatter for MatchForwarder {
                fn write(&mut self, r: &Record) -> io::Result<()> {
                    let _ = self.sender.send((self.path.clone(), r.clone()));
                    Ok(())
                }

                fn flush(&mut self) -> io::Result<()> {
                    Ok(())
                }
            }

            let mut forwarder = MatchForwarder {
                sender: tx,
                path: path.clone(),
            };
            let mut silent_match = false;
            let mut sink = io::sink();
            let mut silent_diag = DiagnosticsSink::new(&mut sink);
            follow_tail(
                path.as_path(),
                &expr,
                &mut forwarder,
                FollowParseSpec::Fixed(line_fmt),
                true,
                Some(&mut silent_diag),
                &mut silent_match,
            )
            .await
        });
    }

    drop(dispatch_tx);

    loop {
        tokio::select! {
            msg = dispatch_rx.recv() => {
                match msg {
                    Some((path, record)) => {
                        emit_follow_source_banner(path.as_path())?;
                        fmt.write(&record)
                            .map_err(|e| anyhow::anyhow!("format write: {e}"))?;
                    }
                    None => {
                        anyhow::bail!("follow_paths multiplex dispatcher closed unexpectedly");
                    }
                }
            },
            joined = join_set.join_next(), if !join_set.is_empty() => {
                let Some(joined) = joined else {
                    anyhow::bail!("join_set unexpectedly empty while join awaited");
                };
                joined??;
            },
        }
    }
}
