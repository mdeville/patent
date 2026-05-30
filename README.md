# patent

**A prior-art search for your code ideas.**

Give `patent` a plain-English dev-tool idea — *"an interactive CLI to kill
whatever's on a port"* — and it searches the open-source ecosystem (crates.io,
GitHub, npm, PyPI, Hacker News) for prior art, ranks the closest matches with
local semantic search, and gives you an honest, scoped verdict on whether it's
already been built.

> Like a patent search, but for code. A patent search *finds prior art* — it
> never certifies absence. Neither does this.

## The one rule

`patent` can prove something **exists**. It can **never** prove something
*doesn't* — it only searched some sources. Every result is scoped to *"what was
found in the sources checked,"* and a clean result is a prompt to keep looking,
not a green light to build.

## Status

Early development — building toward v0. See the milestones (M0–M6) in the plan.

```bash
# (target UX)
patent "interactive cli to kill whatever's on a port"
patent "..." --json | jq .
```

## How it works

1. Parse the idea into keywords.
2. Fan out concurrently to all sources (a failing source is skipped, not fatal).
3. Dedup matches.
4. Rank by semantic similarity (`fastembed`, local).
5. Write a scoped verdict (local Ollama model).
6. Render an interactive TUI (or `--json`).

## Requirements

- Rust (stable) — install via [rustup](https://rustup.rs).
- [Ollama](https://ollama.com) running locally for the verdict:
  `ollama pull qwen2.5 && ollama serve`.

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

Licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE) at your option.
