use std::collections::BTreeMap;
use std::io::{self, Write};

use super::Formatter;
use crate::{Record, Value};

pub const DEFAULT_MAX_BUFFER: usize = 1000;

/// Tabular formatter: buffers heterogeneous rows (`BTreeMap<String, String>`) until [`Self::flush`]
/// or [`Self::max_buffer`] triggers the first emission. Column widths derive from buffered rows seen
/// at that point; afterward rows stream with locked widths derived from [`Self::cols`].
pub struct TableFormatter<W: Write> {
    pub cols: Vec<String>,
    pub buffered_rows: Vec<BTreeMap<String, String>>,
    pub max_buffer: usize,
    writer: W,
    col_widths: Vec<usize>,
    streaming: bool,
    header_written: bool,
}

impl<W: Write> TableFormatter<W> {
    pub fn new(writer: W, max_buffer: usize) -> Self {
        Self {
            cols: Vec::new(),
            buffered_rows: Vec::new(),
            max_buffer: max_buffer.max(1),
            writer,
            col_widths: Vec::new(),
            streaming: false,
            header_written: false,
        }
    }

    pub fn with_default_max_buffer(writer: W) -> Self {
        Self::new(writer, DEFAULT_MAX_BUFFER)
    }

    #[must_use]
    pub fn streaming(&self) -> bool {
        self.streaming
    }

    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    pub fn push_row(&mut self, row: BTreeMap<String, String>) -> io::Result<()> {
        if self.streaming {
            Self::write_streaming_row_internal(
                &mut self.writer,
                &self.cols,
                &self.col_widths,
                &row,
            )?;
            return Ok(());
        }

        merge_cols_from_row(&mut self.cols, &row);
        self.buffered_rows.push(row);

        if self.buffered_rows.len() >= self.max_buffer {
            self.emit_locked_width_flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if !self.streaming && !self.buffered_rows.is_empty() {
            self.emit_locked_width_flush()?;
        }
        self.writer.flush()
    }

    fn emit_locked_width_flush(&mut self) -> io::Result<()> {
        if self.buffered_rows.is_empty() {
            return Ok(());
        }

        for row in &self.buffered_rows {
            merge_cols_from_row(&mut self.cols, row);
        }

        self.col_widths = self
            .cols
            .iter()
            .map(|col| {
                let mut width = display_width(col);
                for row in &self.buffered_rows {
                    if let Some(v) = row.get(col) {
                        width = width.max(display_width(v));
                    }
                }
                width
            })
            .collect();

        if !self.header_written {
            write_header_lines(&mut self.writer, &self.cols, &self.col_widths)?;
            self.header_written = true;
        }

        let rows = std::mem::take(&mut self.buffered_rows);
        for row in rows {
            Self::write_streaming_row_internal(
                &mut self.writer,
                &self.cols,
                &self.col_widths,
                &row,
            )?;
        }

        self.streaming = true;
        Ok(())
    }

    fn write_streaming_row_internal(
        writer: &mut W,
        cols: &[String],
        col_widths: &[usize],
        row: &BTreeMap<String, String>,
    ) -> io::Result<()> {
        debug_assert_eq!(cols.len(), col_widths.len());
        let cells: Vec<&str> = cols
            .iter()
            .map(|c| row.get(c).map(|s| s.as_str()).unwrap_or(""))
            .collect();
        write_padded_row(writer, cells.into_iter(), col_widths)?;
        writeln!(writer)?;
        Ok(())
    }
}

#[inline]
fn display_width(text: &str) -> usize {
    text.chars().count()
}

fn merge_cols_from_row(cols: &mut Vec<String>, row: &BTreeMap<String, String>) {
    for key in row.keys() {
        match cols.binary_search_by(|probe| probe.as_str().cmp(key.as_str())) {
            Ok(_) => {}
            Err(idx) => cols.insert(idx, key.clone()),
        }
    }
}

fn write_header_lines<W: Write>(
    writer: &mut W,
    cols: &[String],
    col_widths: &[usize],
) -> io::Result<()> {
    debug_assert_eq!(cols.len(), col_widths.len());

    write_padded_row(writer, cols.iter().map(String::as_str), col_widths)?;
    writeln!(writer)?;

    for (i, &w) in col_widths.iter().enumerate() {
        if i > 0 {
            write!(writer, "-+-")?;
        }
        for _ in 0..w {
            write!(writer, "-")?;
        }
    }
    writeln!(writer)?;
    Ok(())
}

fn write_padded_row<'a, W: Write, I>(
    writer: &mut W,
    cells: I,
    col_widths: &[usize],
) -> io::Result<()>
where
    I: Iterator<Item = &'a str>,
{
    for (i, cell) in cells.enumerate() {
        if i > 0 {
            write!(writer, " | ")?;
        }
        pad_field(writer, cell, col_widths[i])?;
    }
    Ok(())
}

fn pad_field<W: Write>(writer: &mut W, cell: &str, width_chars: usize) -> io::Result<()> {
    let count = display_width(cell);
    if count > width_chars {
        let visible = width_chars.saturating_sub(1);
        let truncated: String = cell.chars().take(visible).collect();
        write!(writer, "{}", truncated)?;
        return write!(writer, "\u{2026}");
    }

    write!(writer, "{cell}")?;
    let pad = width_chars.saturating_sub(count);
    for _ in 0..pad {
        write!(writer, " ")?;
    }
    Ok(())
}

fn record_scalar_display(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(x) => format!("{x}"),
        Value::Null => String::new(),
    }
}

impl<W: Write> Formatter for TableFormatter<W> {
    fn write(&mut self, r: &Record) -> io::Result<()> {
        let mut row = BTreeMap::new();
        for field in r.iter() {
            row.insert(field.name.to_string(), record_scalar_display(field.value));
        }
        row.insert("_raw".to_string(), r.raw().to_string());
        TableFormatter::push_row(self, row)
    }

    fn flush(&mut self) -> io::Result<()> {
        TableFormatter::flush(self)
    }
}
