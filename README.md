# tracegrep — jq-ish queries for JSON-lines & logfmt

**tracegrep** is a small Rust CLI + library that filters streamed logs: compile a predicate like `level = "error" and msg ~ "timeout"`, evaluate it against each parsed line (JSON-lines, logfmt, or plain), and emit **JSON-lines**, **ASCII tables**, or a **matching-line count**.

---

## Install

Published (once crates.io publishes this crate):

```bash
cargo install tracegrep
```

From this checkout:

```bash
git clone https://github.com/malisetti/tracegrep.git && cd tracegrep
cargo install --path .
tracegrep --version
```

---

## Quickstart — three runnable examples

**1. JSON-lines over stdin**

```bash
printf '%s\n' \
  '{"level":"info","msg":"ok"}' \
  '{"level":"error","msg":"timeout connecting to upstream"}' \
  | tracegrep 'level = "error"' --format json
```

Expected stdout:

```text
{"_raw":"{\"level\":\"error\",\"msg\":\"timeout connecting to upstream\"}","level":"error","msg":"timeout connecting to upstream"}
```

**2. Explicit logfmt (`--input logfmt`)**

```bash
printf '%s\n' 'level=info msg=ok cache=warm' 'level=error msg=timeout service=payments' \
  | tracegrep --input logfmt 'level = "error"' --format json
```

Expected stdout:

```text
{"_raw":"level=error msg=timeout service=payments","level":"error","msg":"timeout","service":"payments"}
```

**3. Follow an append-only file (`--follow`)**

Processes new lines forever until **Ctrl+C**. Use **`--input json`** when bootstrapping an empty file:

```bash
tmp="$(mktemp -t tracegrep-demo.XXXXXXXX)"
trap 'rm -f "$tmp"' EXIT
tracegrep --follow --format json --input json 'level = "error"' "$tmp" &
pid="$!"
sleep 0.25
printf '%s\n' '{"level":"warn","msg":"warming up"}' >>"$tmp"
printf '%s\n' '{"level":"error","msg":"late reconnect"}' >>"$tmp"
sleep 0.35
kill "$pid"
wait "$pid" 2>/dev/null || true
```

You should see the NDJSON row whose `msg` is `late reconnect`.

---

## Query language reference

Combinators (**`and`**, **`or`**, **`not`**) and comparisons are parsed in `src/query/parser.rs`.

| Operator | Meaning | Example RHS |
|---------|---------|-------------|
| `=` / `!=` | Equality | `"error"`, `42`, `true` |
| `<` `<=` `>` `>=` | Ordered compare | `98.6`, `-3` |
| `~` | Regex (`regex`, compiled while parsing the query) | `"timeout"` |

Double-quoted strings, booleans, integers, floats.

---

## Input / output / follow flags

| Flag | Choices | Behaviour |
|------|---------|-----------|
| `--input auto` \| `json` \| `logfmt` \| `plain` | Sniff-first-line auto-detect: `{…}` ⇒ JSON-lines, `word=value…` heuristic ⇒ logfmt; else `_raw/+msg`. |
| `--format auto` \| `json` \| `table` \| `count` | **`auto`** → **table on TTY**, **JSON lines** on pipes. **`count`** → integer then newline. |
| `--follow` | **Exactly one file path**: tail EOF with **Tokio**, **100 ms** sleeps at EOF; reopens if truncated; `--input json` skips fragile sniff-on-empty-tail. |

Emissions expose `_raw` for every structured line.

**Library sketches:** `cargo run --example query_in_memory`; `cargo run --example follow_file -- /tmp/stream.jsonl`.

---

## JSON, table & count (`sample stdout`)

```bash
payload="$(printf '%s\n' '{"level":"info","msg":"ok"}' '{"level":"error","msg":"timeout"}')"
printf '%s' "$payload" | tracegrep 'level = "error"' --format json
```

```text
{"_raw":"{\"level\":\"error\",\"msg\":\"timeout\"}","level":"error","msg":"timeout"}
```

```bash
printf '%s' "$payload" | tracegrep 'level = "error"' --format count
```

```text
1
```

```bash
printf '%s' "$payload" | tracegrep 'level = "error"' --format table
```

```text
_raw                              | level | msg     
----------------------------------+-------+--------
{"level":"error","msg":"timeout"} | error | timeout
```

```bash
printf '%s\n' \
  'level=info svc=api msg=steady' \
  'level=error svc=payments msg="upstream deadline"' \
  | tracegrep --input logfmt 'level = "error"' --format count
```

```text
1
```

---

## Performance note

Per record: **`serde_json`/`logfmt` parse + evaluator** dominates; **`~`** amortizes **`Regex`** construction query-wide. **`--follow`** sleeps on idle EOF. Estimate with **`criterion`**:

```bash
cargo bench --features bench --bench parse_bench --no-run
cargo bench --features bench --bench parse_bench
```

The harness parses **1 000** synthetic records per JSON/logfmt iteration.
