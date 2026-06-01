# CLAUDE.md

Guidance for working in this repository.

## What this is

`patent` — a prior-art search for code ideas. Takes a plain-English dev-tool
idea and searches the open-source ecosystem (11 sources: crates.io, npm, PyPI,
GitHub, Go, Maven, NuGet, RubyGems, Docker Hub, the VS Code Marketplace, and
Hacker News) for prior art, ranks matches with local semantic search
(`fastembed`), and writes a scoped verdict via a local Ollama model. The exact
sources searched are chosen per query (`sources::detect_sources`); GitHub and
Hacker News are always included. Output is an interactive ratatui TUI, or
`--json`.

## ⚠️ Verdict-integrity rules (non-negotiable)

This is the product's whole point — do not soften it:

- The tool can prove something **exists**; it can **never** prove something
  *doesn't*. It only searched some sources.
- All verdict copy is scoped to *"found in the sources checked"* — never
  *"this doesn't exist."*
- The sources-checked list is **always** displayed (transparency), and sources
  that were selected but failed are surfaced as "not reached" (in the TUI and
  in `--json` via `Verdict::sources_failed`), so a thin result is never mistaken
  for "nothing out there."
- The Ollama prompt is told **only** the sources that actually responded — never
  a hardcoded list — and explicitly forbids asserting absence. A clean result
  means *"keep looking before committing,"* not a green light.
- Defence in depth (the prompt is necessary but not trusted): `verdict.rs`
  *floors* the level against the similarity data and *scrubs* any headline that
  asserts non-existence (`guard_headline` / `floor_level`).
- The fixed humble caveat (`verdict::CAVEAT`) appears on every verdict path
  (Ollama success, Ollama unreachable, model-not-pulled, and low-relevance).

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
- `sources/` — `trait SourceAdapter` + one impl per ecosystem; `detect_sources`
  picks the relevant subset per query, `search_all` fans out concurrently
  (`join_all`) and returns a `SearchOutcome { matches, reached, failed }`. A
  failing source is retried once, then skipped and reported "not reached,"
  never fatal.
- `rank.rs` — `fastembed` embeddings + cosine similarity + dedup + top-N.
- `verdict.rs` — builds the integrity-scoped prompt, calls Ollama, parses the
  `Verdict`.
- `ollama.rs` — minimal `localhost:11434/api/generate` client; clear error when
  unreachable.
- `src/bin/{main,cli,tui}.rs` — clap parsing, pipeline wiring, ratatui UI.

## Conventions

- Errors: `thiserror` in the library (`patent::Error`), `anyhow` in the binary.
- Sources are best-effort and independent; never let one source fail the run.
- New sources: implement `SourceAdapter`, add the `Source` variant in
  `model.rs`, register it in `sources::build_source` **and** make it selectable
  in `sources::detect_sources` (a built-but-never-selected source is a bug —
  the `every_built_source_is_reachable_from_some_idea` test guards this), then
  add a `wiremock` integration test in `tests/sources.rs`.
- Each milestone ends fmt-clean, clippy-clean, tests green.

## Milestones

M0 scaffold · M1 first source (crates.io) · M2 all sources + dedup · M3 ranking
· M4 verdict + JSON · M5 TUI · M6 review & polish. See the plan for detail.
