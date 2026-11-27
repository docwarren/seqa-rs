# seqa_agent

A local, private LLM assistant for seqa. It runs an [Ollama](https://ollama.com) model through [rig](https://github.com/0xPlaygrounds/rig) and answers genomics questions by calling seqa's own functions as tools: range searches, file browsing, reference data and ontology search. A CLI, the Tauri desktop app and the axum server can each embed the same library.

## Library

```rust
use seqa_agent::{AgentConfig, AgentSession};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Optional: load an ontology for the search_ontology tool.
    seqa_obos::load("s3://my-bucket/hp.obo").await?;

    let mut config = AgentConfig::new("llama3.1");
    config.allow_local_paths = false;
    let (mut session, mut events) = AgentSession::new(config)?;

    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            // In Tauri: app.emit("tool-result", &event). ToolResultEvent implements Serialize.
            println!("[{} {}] {} items", event.tool, event.result_id, event.items.len());
        }
    });

    let reply = session.chat("What cytobands are on chr17 in hg38?").await?;
    println!("{reply}");
    let follow_up = session.chat("Which of those is 17q21.31?").await?;
    println!("{follow_up}");
    session.reset();
    Ok(())
}
```

- `AgentConfig::new(model)` sets the defaults: Ollama at `http://localhost:11434` (`ollama_url: None`), `summary_limit` 20, `allow_local_paths` true, the built-in `preamble`, and no `extra_tools`. Change any field before creating the session.
- `AgentSession::new(config)` returns the session and a `tokio::sync::mpsc::UnboundedReceiver<ToolResultEvent>`. Each successful tool call sends one event `{ result_id, tool, items }` holding the full result. The host must drain the receiver or drop it. Sends to a dropped receiver are ignored, and results are never stored.
- `chat(message)` returns the model's final reply. The model may call several tools first (up to 10 model calls per message). The message and reply are added to the history only when the turn succeeds. A failed turn, such as an unreachable Ollama, returns `AgentError::Chat` and leaves the history unchanged. Tool result events already sent during a failed turn are not taken back, so the host can receive events for a turn that ends in an error.
- `reset()` clears the history and keeps the tools.
- `extra_tools` takes `seqa_agent::rig_agent::tool::DynamicTool`s, offered to the model alongside the built-in tools. `seqa_agent` re-exports `rig_agent` so hosts build them with the same rig version.
- The library never prints and never reads the terminal. Outside tool calls it does no file, network or process I/O; its only other I/O is chat requests to Ollama. The model is offered only the seven built-in tools and the host's extra tools: no rig built-in tools, MCP clients or Ollama-side tools.
- The model must support tool calling in Ollama, for example `llama3.1`, `qwen2.5` or `mistral-nemo`. Small models call tools less reliably.

## REPL

`seqa-agent` is a terminal chat built only on the public library API. It is behind the `cli` feature, so library users don't pull in `clap`:

```sh
cargo build --release -p seqa_agent --features cli      # target/release/seqa-agent
cargo install --path seqa_agent --features cli          # or install it
```

Usage:

```
seqa-agent [OPTIONS] --model <MODEL>

Options:
      --model <MODEL>                  Ollama model name. The model must support tool calling, e.g. llama3.1
      --ollama-url <OLLAMA_URL>        Ollama base URL [default: http://localhost:11434]
      --ontology <ONTOLOGY>            Ontology (e.g. HPO) to load at startup for search_ontology: a path or s3://, gs://, az://, http(s):// URL
      --summary-limit <SUMMARY_LIMIT>  Maximum number of items a tool returns to the model [default: 20]
  -h, --help                           Print help
```

Example:

```sh
ollama pull llama3.1
seqa-agent --model llama3.1 --ontology s3://my-bucket/hp.obo
> What cytobands are on chr17 in hg38?
[cytobands r1] 24 items
...
> /reset
History cleared.
> /quit
```

- Type one message per line. `/reset` clears the conversation. `/quit` or end of input (Ctrl-D) exits with status 0.
- Each tool result prints one line, `[<tool> <result_id>] <total> items`. The full result is not printed.
- An error in one turn, such as an unreachable Ollama, is printed and the prompt returns.
- If `--ontology` fails to load, the error is printed and `seqa-agent` exits with a non-zero status. An ontology path must not contain `..`.

## Tools

The tools run in process. Each successful call sends its full result to the host as a `ToolResultEvent` and returns only a short summary to the model, so large results never fill the model's context. The event's `result_id` matches the one in the model's summary.

### Reference data and formats

These tools return counts or a short list to the model. The full result goes to the host.

| Tool | Arguments | Full-result item | Model receives |
|------|-----------|------------------|----------------|
| `supported_formats` | none | `{ "format": "BAM", "extensions": [".bam"], "indexExtension": ".bai" }` (`indexExtension` is `null` for BigWig and BigBed) | `{ result_id, tool, total, items, omitted }` |
| `gene_coordinates` | `genome` (`hg38` or `hg19`), `source` (`refseq` or `gencode`) | `{ "chromosome": "chr17", "begin": 43044295, "end": 43170327, "symbol": "BRCA1" }` | `{ result_id, tool, record_count }` |
| `cytobands` | `genome` (`hg38` or `hg19`), `chromosome` (with the `chr` prefix, e.g. `chr17`) | `{ "chromosome": "chr17", "begin": 42800000, "end": 46800000, "name": "q21.31", "stain": "gneg" }` | `{ result_id, tool, record_count }` |

Example arguments:

```json
{ "genome": "hg38", "source": "refseq" }
{ "genome": "hg38", "chromosome": "chr17" }
```

`genome` and `source` are read exactly as `seqa_core::reference_data` reads them: `hg19`, `grch37` and `ch37` select hg19 and anything else selects hg38; any source other than `gencode` selects RefSeq. A chromosome without the `chr` prefix (`17`) matches no bands and returns a `record_count` of 0.

### Storage

| Tool | Arguments | Full-result item | Model receives |
|------|-----------|------------------|----------------|
| `search_file` | `path` (file URI), `coordinates` (list of one-based regions) | header lines first, `{ "kind": "header", "line": "##fileformat=VCFv4.2" }`, then records, `{ "kind": "record", "coordinate": "chr1:30000-50000", "line": "chr1\t34000\t..." }` | `{ result_id, tool, header_line_count, record_count }` |
| `list_directory` | `uri` (folder URI) | `{ "uri": "s3://bucket/project/a.bam", "name": "a.bam", "isDirectory": false, "size": 1024, "lastModified": "2026-01-01T00:00:00+00:00" }` | `{ result_id, tool, total, items, omitted }` |
| `list_buckets` | `backend` (`s3`, `gcs` or `azure`) | bucket URI, e.g. `"s3://my-bucket"` or `"az://account/container"` | `{ result_id, tool, total, items, omitted }` |

Example arguments:

```json
{ "path": "s3://bucket/sample.vcf.gz", "coordinates": ["chr1:1000000-2000000", "chr2"] }
{ "uri": "s3://bucket/project/" }
{ "backend": "s3" }
```

- `search_file` works like the rest of seqa: the index path and file format come from the file's extension, and the index sits next to the file. A remote index is cached in the working directory (see the `seqa_core` README). `record_count` is the total across all coordinates. The model sees no header or record text.
- `list_directory` lists one level, folders first. A plain local path such as `/data/project/`, or a Windows drive path such as `C:\data`, is turned into a `file://` URI.
- `items` holds at most `summary_limit` entries (default 20). `omitted` is the number left out.
- Credentials come from the same environment variables as the rest of seqa (`AWS_*`, `AZURE_*`, `GOOGLE_*`).

### Local paths

`allow_local_paths` is a host setting on `AgentConfig`, not a tool argument, so the model cannot change it. It defaults to `true`. When it is `false`, `search_file` and `list_directory` reject local paths, Windows drive paths and `file:` URIs before touching storage, using the same check as `seqa_axum` (`seqa_api::local_uri::is_local_uri`) plus a check for single-letter (drive) schemes. Cloud and HTTP URIs are unaffected. A server host should set it to `false`.

### Ontology

| Tool | Arguments | Full-result item | Model receives |
|------|-----------|------------------|----------------|
| `search_ontology` | `query`, optional `limit` (at least 1, default 100) | a full `seqa_obos::Term`, e.g. `{ "id": "HP:0000252", "name": "Microcephaly", "def": ..., "synonyms": [...] }` | `{ result_id, tool, total, items, omitted }` |

Example arguments:

```json
{ "query": "microceph" }
{ "query": "HP:00002", "limit": 5 }
```

The tool only searches; it never loads. **The host must load the ontology first** with `seqa_obos::load(location).await`. Until then, the tool returns "no ontology has been loaded" to the model. `seqa_obos` keeps **one ontology per process**, so every agent in a process searches the same ontology, and loading a different one replaces it for all of them.
