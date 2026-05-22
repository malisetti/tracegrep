//! tracegrep CLI: read lines → parse → eval(query) → format → write.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use tracegrep::output::{CountFormatter, Formatter, JsonFormatter, TableFormatter};
use tracegrep::query::{parse, Expr};
use tracegrep::{parse_auto, sniff, Format, Record, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Auto,
    Json,
    Table,
    Count,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum InputHint {
    Auto,
    Json,
    Logfmt,
    Plain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EffectiveOutput {
    Json,
    Table,
    Count,
}

#[derive(Parser)]
#[command(name = "tracegrep", version, about)]
struct Cli {
    /// Query expression (e.g. `level = "error" and msg ~ "timeout"`).
    query: String,

    /// Files to read; `-` or omitted means stdin.
    #[arg(required = false)]
    files: Vec<PathBuf>,

    #[arg(long, value_enum, default_value_t = OutputFormat::Auto)]
    format: OutputFormat,

    /// Reserved for follow mode (not implemented yet).
    #[arg(long)]
    follow: bool,

    #[arg(long, value_enum, default_value_t = InputHint::Auto)]
    input: InputHint,
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e:#}");
        std::process::exit(1);
    }
}

fn try_main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let _ = cli.follow;

    let body = read_sources(&cli.files)?;
    let line_fmt = line_format(cli.input, &body);
    let expr = parse(&cli.query).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let tty = out.is_terminal();
    let eff = resolve_output(cli.format, tty);

    match eff {
        EffectiveOutput::Json => {
            let mut f = JsonFormatter::new(&mut out);
            process_lines(&body, line_fmt, &expr, |rec| {
                f.write(rec)?;
                Ok(())
            })?;
            f.flush()?;
        }
        EffectiveOutput::Count => {
            let mut f = CountFormatter::new(&mut out);
            process_lines(&body, line_fmt, &expr, |rec| {
                f.write(rec)?;
                Ok(())
            })?;
            f.flush()?;
        }
        EffectiveOutput::Table => {
            let mut t = TableFormatter::with_default_max_buffer(&mut out);
            process_lines(&body, line_fmt, &expr, |rec| {
                t.push_row(record_to_row(rec))?;
                Ok(())
            })?;
            t.flush()?;
            out.flush()?;
        }
    }

    Ok(())
}

fn resolve_output(sel: OutputFormat, stdout_tty: bool) -> EffectiveOutput {
    match sel {
        OutputFormat::Json => EffectiveOutput::Json,
        OutputFormat::Table => EffectiveOutput::Table,
        OutputFormat::Count => EffectiveOutput::Count,
        OutputFormat::Auto => {
            if stdout_tty {
                EffectiveOutput::Table
            } else {
                EffectiveOutput::Json
            }
        }
    }
}

fn line_format(hint: InputHint, full_text: &str) -> Format {
    match hint {
        InputHint::Auto => sniff(full_text),
        InputHint::Json => Format::JsonLines,
        InputHint::Logfmt => Format::Logfmt,
        InputHint::Plain => Format::Plain,
    }
}

fn read_sources(paths: &[PathBuf]) -> anyhow::Result<String> {
    let mut chunks = Vec::new();

    let read_stdin = || -> anyhow::Result<String> {
        let mut buf = String::new();
        io::stdin().lock().read_to_string(&mut buf)?;
        Ok(buf)
    };

    if paths.is_empty() {
        chunks.push(read_stdin()?);
    } else {
        for p in paths {
            if p.as_os_str() == OsStr::new("-") {
                chunks.push(read_stdin()?);
            } else {
                let mut buf = String::new();
                File::open(p)?.read_to_string(&mut buf)?;
                chunks.push(buf);
            }
        }
    }

    Ok(chunks.join("\n"))
}

fn record_to_row(r: &Record) -> BTreeMap<String, String> {
    let mut row = BTreeMap::new();
    for field in r.iter() {
        row.insert(field.name.to_string(), scalar_display(field.value));
    }
    row.insert("_raw".to_string(), r.raw().to_string());
    row
}

fn scalar_display(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(x) => format!("{x}"),
        Value::Null => String::new(),
    }
}

fn process_lines(
    body: &str,
    fmt: Format,
    expr: &Expr,
    mut emit: impl FnMut(&Record) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    for (lineno, raw) in body.lines().enumerate() {
        let lineno = lineno + 1;
        if raw.trim().is_empty() {
            continue;
        }
        let record = parse_auto(raw, fmt).map_err(|e| anyhow::anyhow!("line {}: {}", lineno, e))?;
        if expr.eval(&record) {
            emit(&record)?;
        }
    }
    Ok(())
}
