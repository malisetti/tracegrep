//! tracegrep CLI: read lines → parse → eval(query) → format → write.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use tracegrep::follow::{follow_tail, FollowParseSpec};
use tracegrep::group::bump_group_count;
use tracegrep::output::{
    ColorWriter, CountFormatter, Formatter, JsonFormatter, JsonLinesFormatter, TableFormatter,
};
use tracegrep::parser::resilience::{parse_cli_line, CliInputHint, ParsedCliLine};
use tracegrep::query::{parse, Expr};
use tracegrep::{DiagnosticsSink, ExitCode, Format, Record, Value};

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

    /// Tail one file (`tail -f`-style polling at EOF via async I/O).
    #[arg(long)]
    follow: bool,

    /// Abort when a physical line fails to parse (`--strict false` disables).
    #[arg(long, default_value_t = true)]
    strict: bool,

    #[arg(long = "no-strict")]
    no_strict: bool,

    /// Emit NDJSON with a flush after each line so downstream tools see lines immediately.
    #[arg(long)]
    jsonl: bool,

    #[arg(long, value_enum, default_value_t = InputHint::Auto)]
    input: InputHint,

    /// Comma-separated field projection for structured/table output (`_raw` only if listed).
    #[arg(long = "field")]
    field_projection: Option<String>,

    /// With `--format=count`, print `<bucket>\t<count>` lines sorted by bucket.
    #[arg(long = "group-by")]
    group_by: Option<String>,

    /// Disable ANSI coloring (currently used for highlighted `level` cells in `--format table`).
    #[arg(long = "no-color")]
    no_color: bool,
}

fn main() {
    match try_main() {
        Ok(any_match) => std::process::exit(ExitCode::from(any_match).as_i32()),
        Err(e) => {
            eprintln!("{e:#}");
            std::process::exit(ExitCode::Error.as_i32());
        }
    }
}

fn try_main() -> anyhow::Result<bool> {
    let cli = Cli::parse();
    let expr = parse(&cli.query).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let mut diag = DiagnosticsSink::new(&mut lock);

    let strict_parse = cli.strict && !cli.no_strict;

    let any_match = if cli.follow {
        run_follow(&cli, &expr, strict_parse, &mut diag)?
    } else {
        let body = read_sources(&cli.files)?;
        run_batch(&cli, &body, &expr, strict_parse, &mut diag)?
    };

    diag.summary()?;
    Ok(any_match)
}

fn run_follow<W: Write>(
    cli: &Cli,
    expr: &Expr,
    strict: bool,
    diag: &mut DiagnosticsSink<W>,
) -> anyhow::Result<bool> {
    if cli.files.len() != 1 {
        anyhow::bail!("--follow requires exactly one file path (not stdin)");
    }
    let path = &cli.files[0];
    if path.as_os_str() == OsStr::new("-") {
        anyhow::bail!("--follow does not support stdin; pass a file path");
    }

    let parse_spec = match cli.input {
        InputHint::Auto => FollowParseSpec::AutoPerLine,
        InputHint::Json => FollowParseSpec::Fixed(Format::JsonLines),
        InputHint::Logfmt => FollowParseSpec::Fixed(Format::Logfmt),
        InputHint::Plain => FollowParseSpec::Fixed(Format::Plain),
    };

    let runtime = tokio::runtime::Runtime::new()?;
    let path_ref = path.as_path();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let tty = out.is_terminal();
    let eff = resolve_output(cli.format, tty);

    let fields = parsed_fields(cli.field_projection.as_ref());

    let mut any_match = false;

    match eff {
        EffectiveOutput::Json => {
            let attach_footer = fields.is_none();
            if cli.jsonl {
                let mut f = if attach_footer {
                    JsonLinesFormatter::new(&mut out)
                } else {
                    JsonLinesFormatter::without_raw_footer(&mut out)
                };
                runtime.block_on(follow_tail(
                    path_ref,
                    expr,
                    &mut JsonProjectingFmt {
                        inner: &mut f,
                        fields: fields.as_deref(),
                    },
                    parse_spec,
                    strict,
                    Some(diag),
                    &mut any_match,
                ))?;
                f.flush()?;
            } else {
                let mut f = if attach_footer {
                    JsonFormatter::new(&mut out)
                } else {
                    JsonFormatter::without_raw_footer(&mut out)
                };
                runtime.block_on(follow_tail(
                    path_ref,
                    expr,
                    &mut JsonProjectingFmt {
                        inner: &mut f,
                        fields: fields.as_deref(),
                    },
                    parse_spec,
                    strict,
                    Some(diag),
                    &mut any_match,
                ))?;
                f.flush()?;
            }
        }
        EffectiveOutput::Count => {
            if cli.group_by.is_some() {
                anyhow::bail!("--group-by is not supported with --follow");
            }
            let mut f = CountFormatter::new(&mut out);
            runtime.block_on(follow_tail(
                path_ref,
                expr,
                &mut f,
                parse_spec,
                strict,
                Some(diag),
                &mut any_match,
            ))?;
            f.flush()?;
        }
        EffectiveOutput::Table => {
            let color_enabled = tty && !cli.no_color;
            let mut t = TableFormatter::with_default_max_buffer(&mut out);
            runtime.block_on(follow_tail(
                path_ref,
                expr,
                &mut TableProjectingFmt {
                    inner: &mut t,
                    fields: fields.as_deref(),
                    color_stdout: color_enabled,
                },
                parse_spec,
                strict,
                Some(diag),
                &mut any_match,
            ))?;
            t.flush()?;
        }
    }

    Ok(any_match)
}

/// [`JsonFormatter`] / [`JsonLinesFormatter`] adapter applying optional field projection.
struct JsonProjectingFmt<'a, J: Formatter> {
    inner: &'a mut J,
    fields: Option<&'a [String]>,
}

impl<J: Formatter> Formatter for JsonProjectingFmt<'_, J> {
    fn write(&mut self, r: &Record) -> io::Result<()> {
        match self.fields {
            None => self.inner.write(r),
            Some(cols) => {
                let pr = projected_record(r, cols);
                self.inner.write(&pr)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

struct TableProjectingFmt<'a, W: Write> {
    inner: &'a mut TableFormatter<W>,
    fields: Option<&'a [String]>,
    color_stdout: bool,
}

impl<W: Write> Formatter for TableProjectingFmt<'_, W> {
    fn write(&mut self, r: &Record) -> io::Result<()> {
        let row = record_to_row(r, self.color_stdout, self.fields);
        self.inner.push_row(row)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn run_batch<W: Write>(
    cli: &Cli,
    body: &str,
    expr: &Expr,
    strict: bool,
    diag: &mut DiagnosticsSink<W>,
) -> anyhow::Result<bool> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let tty = out.is_terminal();
    let eff = resolve_output(cli.format, tty);

    let hint = cli_input_hint(cli.input);
    let mut any_match = false;
    let fields = parsed_fields(cli.field_projection.as_ref());

    match eff {
        EffectiveOutput::Json => {
            let attach_footer = fields.is_none();
            if cli.jsonl {
                let mut f = if attach_footer {
                    JsonLinesFormatter::new(&mut out)
                } else {
                    JsonLinesFormatter::without_raw_footer(&mut out)
                };
                walk_batch_lines(
                    body,
                    expr,
                    hint,
                    strict,
                    diag,
                    &mut any_match,
                    |rec| match fields.as_deref() {
                        None => f.write(rec),
                        Some(cols) => {
                            let pr = projected_record(rec, cols);
                            f.write(&pr)
                        }
                    },
                )?;
                f.flush()?;
            } else {
                let mut f = if attach_footer {
                    JsonFormatter::new(&mut out)
                } else {
                    JsonFormatter::without_raw_footer(&mut out)
                };
                walk_batch_lines(
                    body,
                    expr,
                    hint,
                    strict,
                    diag,
                    &mut any_match,
                    |rec| match fields.as_deref() {
                        None => f.write(rec),
                        Some(cols) => {
                            let pr = projected_record(rec, cols);
                            f.write(&pr)
                        }
                    },
                )?;
                f.flush()?;
            }
        }
        EffectiveOutput::Count => {
            if let Some(gb) = cli.group_by.as_deref() {
                let mut tallies: BTreeMap<String, u64> = BTreeMap::new();
                walk_batch_lines(body, expr, hint, strict, diag, &mut any_match, |rec| {
                    bump_group_count(&mut tallies, rec, gb);
                    Ok(())
                })?;
                for (k, n) in &tallies {
                    writeln!(out, "{k}\t{n}")?;
                }
            } else {
                let mut f = CountFormatter::new(&mut out);
                walk_batch_lines(body, expr, hint, strict, diag, &mut any_match, |rec| {
                    f.write(rec)
                })?;
                f.flush()?;
            }
            out.flush()?;
        }
        EffectiveOutput::Table => {
            let color_enabled = tty && !cli.no_color;
            let mut t = TableFormatter::with_default_max_buffer(&mut out);
            walk_batch_lines(body, expr, hint, strict, diag, &mut any_match, |rec| {
                let row = record_to_row(rec, color_enabled, fields.as_deref());
                t.push_row(row)
            })?;
            t.flush()?;
        }
    }

    Ok(any_match)
}

fn cli_input_hint(h: InputHint) -> CliInputHint {
    match h {
        InputHint::Auto => CliInputHint::Auto,
        InputHint::Json => CliInputHint::Json,
        InputHint::Logfmt => CliInputHint::Logfmt,
        InputHint::Plain => CliInputHint::Plain,
    }
}

fn walk_batch_lines<W: Write>(
    body: &str,
    expr: &Expr,
    hint: CliInputHint,
    strict: bool,
    diag: &mut DiagnosticsSink<W>,
    any_match: &mut bool,
    mut on_match: impl FnMut(&Record) -> io::Result<()>,
) -> anyhow::Result<()> {
    let mut diag_slot: Option<&mut DiagnosticsSink<W>> = Some(diag);

    for (lineno0, raw) in body.lines().enumerate() {
        let lineno = (lineno0 + 1) as u64;

        match parse_cli_line(raw, lineno, hint, strict, &mut diag_slot)? {
            ParsedCliLine::Ignored => {}
            ParsedCliLine::Record(rec) => {
                if expr.eval(&rec) {
                    *any_match = true;
                    on_match(&rec)?;
                }
            }
        }
    }
    Ok(())
}

fn parsed_fields(sel: Option<&String>) -> Option<Vec<String>> {
    let inner = sel?;
    let v: Vec<String> = inner
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn projected_record(r: &Record, fields: &[String]) -> Record {
    let mut out = Record::new(r.raw());
    for name in fields {
        if let Some(v) = r.get(name) {
            out.insert(name.clone(), v.clone());
        }
    }
    out
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

fn record_to_row(
    r: &Record,
    color_stdout: bool,
    projection: Option<&[String]>,
) -> BTreeMap<String, String> {
    let styler = ColorWriter::with_enabled(io::sink(), color_stdout);

    fn insert_field(row: &mut BTreeMap<String, String>, name: &str, value_display: String) {
        row.insert(name.to_string(), value_display);
    }

    let mut row = BTreeMap::new();
    if let Some(proj) = projection {
        for name in proj {
            let val_disp = match r.get(name.as_str()) {
                None => String::new(),
                Some(value) => {
                    let scalar = scalar_display(value);
                    if name == "level" {
                        styler.colorize_level(&scalar)
                    } else {
                        scalar
                    }
                }
            };
            insert_field(&mut row, name, val_disp);
        }
    } else {
        for field in r.iter() {
            let val = scalar_display(field.value);
            let val_display = if field.name == "level" {
                styler.colorize_level(&val)
            } else {
                val
            };
            insert_field(&mut row, field.name, val_display);
        }
        row.insert("_raw".to_string(), r.raw().to_string());
    }

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
