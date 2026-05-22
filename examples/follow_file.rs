//! Async tail loop like `tracegrep --follow --format json` (runs until Ctrl+C).
//!
//! ```bash
//! tmp="$(mktemp -t tracegrep-follow.XXXXXXXX)"
//! trap 'rm -f "$tmp"' EXIT
//! cargo run --example follow_file -- "$tmp" &
//! pid="$!"
//! sleep 0.2
//! printf '%s\n' '{"level":"error","msg":"late"}' >>"$tmp"
//! sleep 0.5
//! kill "$pid"
//! wait "$pid" 2>/dev/null || true
//! ```

use std::path::Path;

use tracegrep::follow_path;
use tracegrep::output::JsonFormatter;
use tracegrep::query::parse;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Some(path_arg) = std::env::args().nth(1) else {
        anyhow::bail!("usage: cargo run --example follow_file -- /path/to/log.jsonl");
    };

    let path = Path::new(&path_arg);
    let expr = parse(r#"level = "error""#)?;

    let mut out = std::io::stdout().lock();
    let mut fmt = JsonFormatter::new(&mut out);
    follow_path(path, &expr, &mut fmt).await
}
