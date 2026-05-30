# CLAUDE.md

Guidance for working in this repository.

## What this is

`patent` — a prior-art search for code ideas. Takes a plain-English dev-tool
idea and searches the open-source ecosystem (crates.io, GitHub, npm, PyPI,
Hacker News) for prior art, ranks matches with local semantic search
(`fastembed`), and writes a scoped verdict via a local Ollama model. Output is
an interactive ratatui TUI, or `--json`.

## ⚠️ Verdict-integrity rules (non-negotiable)

This is the product's whole point — do not soften it:

- The tool can prove something **exists**; it can **never** prove something
  *doesn't*. It only searched some sources.
- All verdict copy is scoped to *"found in the sources checked"* — never
  *"this doesn't exist."*
- The sources-checked list is **always** displayed (transparency).
- The Ollama prompt explicitly forbids asserting absence; a clean result means
  *"keep looking before committing,"* not a green light.
- The fixed humble caveat (`verdict::CAVEAT`) appears on every verdict.

## Commands

```bash
cargo fmt --all --check                       # formatting (CI-enforced)
cargo clippy --all-targets -- -D warnings     # lint, warnings denied (CI-enforced)
cargo test                                    # unit + wiremock integration tests
cargo build                                   # build
cargo run -- "your idea here"                 # run the CLI/TUI
cargo run -- "your idea" --json | jq .        # structured output
```

## Architecture

lib/bin split: the testable core is the library (`src/lib.rs`); the binary
(`src/bin/`) is a thin CLI/TUI shell.

- `model.rs` — core types (`Query`, `Source`, `Match`, `Saturation`, `Verdict`).
- `sources/` — `trait Source` + one impl per ecosystem; `search_all` fans out
  concurrently (`join_all`). A failing source is skipped and reported "not
  reached," never fatal.
- `rank.rs` — `fastembed` embeddings + cosine similarity + dedup + top-N.
- `verdict.rs` — builds the integrity-scoped prompt, calls Ollama, parses the
  `Verdict`.
- `ollama.rs` — minimal `localhost:11434/api/generate` client; clear error when
  unreachable.
- `src/bin/{main,cli,tui}.rs` — clap parsing, pipeline wiring, ratatui UI.

## Conventions

- Errors: `thiserror` in the library (`patent::Error`), `anyhow` in the binary.
- Sources are best-effort and independent; never let one source fail the run.
- New sources: implement `Source`, register in `sources::search_all`, add a
  `wiremock` integration test in `tests/sources.rs`.
- Each milestone ends fmt-clean, clippy-clean, tests green.

## Milestones

M0 scaffold · M1 first source (crates.io) · M2 all sources + dedup · M3 ranking
· M4 verdict + JSON · M5 TUI · M6 review & polish. See the plan for detail.
