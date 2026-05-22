//! Filter hard-coded JSON-lines with [`tracegrep::query::parse`] + [`tracegrep::parse_auto`].
//!
//! ```bash
//! cargo run --example query_in_memory
//! ```

use tracegrep::query::parse;
use tracegrep::{parse_auto, Format};

fn main() {
    let lines: Vec<String> = vec![
        r#"{"level":"info","service":"api","msg":"started"}"#.to_string(),
        r#"{"level":"error","service":"db","msg":"timeout connecting to upstream"}"#.to_string(),
        r#"{"level":"warn","service":"cache","msg":"miss for key=user:42"}"#.to_string(),
    ];

    let expr = parse(r#"level = "error" and msg ~ "timeout""#).expect("compile query");
    let line_fmt = Format::JsonLines;

    for raw in lines {
        let record = parse_auto(&raw, line_fmt).expect("parse line");
        if expr.eval(&record) {
            println!("{}", record.raw());
        }
    }
}
