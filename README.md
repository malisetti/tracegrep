# tracegrep — jq-ish queries for JSON-lines & logfmt

**tracegrep** is a small Rust CLI + library that filters streamed logs: compile a predicate like `level = "error" and msg ~ "timeout"`, evaluate it against each parsed line (JSON-lines, logfmt, or plain), and emit **JSON-lines**, **ASCII tables**, or a **matching-line count**.

---

## Install

From [crates.io](https://crates.io/crates/tracegrep) (recommended once published):

```bash
cargo install tracegrep
cargo install tracegrep --version "^0.2"
tracegrep --version
```

From a git checkout (see [CHANGELOG.md](CHANGELOG.md) for releases):

```bash
git clone https://github.com/malisetti/tracegrep.git && cd tracegrep
cargo install --locked --path .
tracegrep --version
```

Rust **1.70+** matches this crate's `rust-version` in `Cargo.toml`.

---

## v0.2.0 — CLI polish

Structured exit codes (**0** = matches, **1** = no matches, **2** = error) behave like **`grep`**; malformed lines integrate with **`--strict` / `--no-strict`**; **`--input=auto`** sniffs format **per physical line**.

**Recover from bad lines (`--no-strict`)**

```bash
printf '%s\n' '{"broken' '{"level":"error","msg":"late"}' \
  | tracegrep --input json --no-strict 'level = "error"' --format count
```

**Stream NDJSON one record at a time (`--jsonl`)**

```bash
printf '%s\n' '{"msg":"timeout"}' '{"msg":"steady"}' \
  | tracegrep --input json --format json --jsonl 'msg ~ "timeout"'
```

**Project columns (`--field`)**

```bash
printf '%s\n' '{"level":"error","svc":"pay","msg":"timeout"}' \
  | tracegrep --input json --format json --field level,svc 'svc = "pay"'
```

**Bucket match counts (`--group-by`)**

```bash
printf '%s\n' \
  '{"level":"error","host":"a"}' \
  '{"level":"error","host":"b"}' \
  '{"level":"error","host":"a"}' \
  '{"level":"info","host":"z"}' \
  | tracegrep --input json --format count --group-by host 'level = "error"'
```

Expect tab-separated **`host<count>` lines** sorted by bucket.

**Several files & tail-follow**

Multiple positional paths concatenate inputs in batch mode:

```bash
tracegrep --input json 'level = "error"' ./one.jsonl ./two.jsonl --format count
```

`--follow` tails **one** path at a time; run **`tracegrep --follow`** once per append-only stream (shell background jobs or supervisors) when you want live multi-file coverage.

Diagnostics for skipped lines emit on **stderr**; use **`--no-color`** when piping table output through tools that choke on ANSI.

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
| `--input auto` \| `json` \| `logfmt` \| `plain` | **`auto`** re-sniffs **each line**: `{…}` ⇒ JSON-lines, `word=value…` heuristic ⇒ logfmt; otherwise plain `_raw`/optional `msg`. |
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
