# patent

**A prior-art search for your code ideas.**

Give `patent` a plain-English dev-tool idea and it searches 11 open-source
registries for existing implementations, ranks them with local semantic search,
and writes an honest verdict — all without leaving the terminal.

```
patent "interactive cli to kill whatever's on a port"
```

> Like a patent search, but for code. A patent search *finds prior art* — it
> never certifies absence. Neither does this.

## Features

- **11 sources** — crates.io, npm, PyPI, GitHub, Go, Maven, NuGet, RubyGems,
  Docker Hub, VS Code Marketplace, Hacker News
- **Smart source selection** — mentions "rust" and it searches crates.io;
  mentions "docker" and it hits Docker Hub. GitHub and Hacker News (both
  language-agnostic) are always searched; with no language signal it falls back
  to a broad sweep across the largest registries.
- **Semantic ranking** — local embeddings (AllMiniLM-L6-V2 via
  [fastembed](https://crates.io/crates/fastembed)) rank matches by cosine
  similarity to your idea
- **AI verdict** — a local [Ollama](https://ollama.com) model classifies the
  space as Open / Crowded / Saturated and identifies gaps
- **Interactive TUI** — scrollable matches, filtering, one-key URL opening,
  help overlay
- **JSON output** — `--json` for scripting and CI pipelines
- **Fully local** — no data leaves your machine; embeddings and LLM run locally
- **Graceful degradation** — Ollama down, or the model not pulled? Results
  still render, ranked by similarity, without an AI verdict. A source fails?
  It's skipped (and shown as "not reached"), never fatal.

## Install

### From crates.io

```bash
cargo install patent
```

### From source

```bash
git clone https://github.com/r14dd/patent.git
cd patent
cargo install --path .
```

### Prerequisites

**Rust** (stable 1.80+) via [rustup](https://rustup.rs).

**Ollama** (optional, recommended) — powers the AI verdict:

```bash
# macOS
brew install ollama

# Linux
curl -fsSL https://ollama.com/install.sh | sh

# Then:
ollama pull qwen2.5
ollama serve
```

Without Ollama, `patent` still searches and ranks — you just won't get the
AI-generated verdict.

**GitHub token** (optional) — the unauthenticated GitHub search API is limited
to 10 requests/minute. Set a token to raise that to 30 requests/minute (3×):

```bash
export GITHUB_TOKEN=ghp_your_token_here
```

**First run** — `patent` downloads a small (~80 MB) embedding model the first
time it ranks results. It's cached under your OS cache directory (e.g.
`~/Library/Caches/patent` on macOS, `~/.cache/patent` on Linux), so it's a
one-time download shared across every directory you run from.

## Usage

```bash
# Basic search — opens the interactive TUI
patent "CLI tool that kills whatever's on a port"

# Structured JSON output for scripting
patent "react component for infinite scroll" --json | jq .

# Use a smaller/faster Ollama model
patent "kubernetes log viewer" --model qwen2.5:3b

# Keep more matches after ranking
patent "async runtime for rust" --limit 100
```

### Options

| Flag | Description | Default |
|---|---|---|
| `--json` | Print JSON to stdout instead of the TUI | — |
| `--model <MODEL>` | Ollama model for the verdict | `qwen2.5` |
| `--limit <N>` | Max matches to keep after ranking | `50` |
| `--completions <SHELL>` | Generate shell completions and exit | — |

### Shell completions

```bash
# Bash
patent --completions bash >> ~/.bashrc

# Zsh
patent --completions zsh >> ~/.zshrc

# Fish
patent --completions fish > ~/.config/fish/completions/patent.fish

# PowerShell
patent --completions powershell >> $PROFILE
```

## TUI keybindings

| Key | Action |
|---|---|
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `/` | Filter matches |
| `m` | Show more / show less |
| `Enter` | Open selected match in browser |
| `?` | Help overlay |
| `q` | Quit |

Press `?` inside the TUI for the full keybinding reference.

## How it works

```
idea ──► parse keywords
              │
              ├──► fan out to sources (concurrent, with retry)
              │         │
              │         ▼
              │    dedup matches
              │         │
              ▼         ▼
         load model ──► rank by cosine similarity
                              │
                              ▼
                        verdict via Ollama
                              │
                              ▼
                        TUI or JSON
```

1. **Parse** — extracts keywords, selects relevant sources based on the idea
2. **Search** — fans out to selected sources concurrently; a failing source is
   skipped and retried once, never fatal
3. **Rank** — embeds the idea and each match description with AllMiniLM-L6-V2,
   sorts by cosine similarity, keeps the top N
4. **Verdict** — sends the ranked matches to a local Ollama model that
   classifies the space and identifies gaps
5. **Output** — interactive TUI (default) or structured JSON (`--json`)

The embedding model loads concurrently with source searches, so the model-load
latency is hidden behind network I/O.

## The integrity rule

`patent` can prove something **exists**. It can **never** prove something
*doesn't* — it only searched some sources. Every verdict is scoped to "what was
found in the sources checked," and a clean result means *keep looking*, not
*start building*.

The sources-checked list is always displayed for transparency.

## Architecture

```
src/
├── lib.rs              # library root
├── model.rs            # Query, Match, Source, Verdict, Saturation
├── sources/
│   ├── mod.rs          # trait Source, search_all fan-out, dedup, retry
│   ├── crates_io.rs    # crates.io API
│   ├── github.rs       # GitHub search API
│   ├── npm.rs          # npm registry API
│   ├── pypi.rs         # PyPI search (HTML scraping)
│   ├── hacker_news.rs  # HN Algolia API
│   ├── go.rs           # Go package search
│   ├── maven.rs        # Maven Central API
│   ├── nuget.rs        # NuGet API
│   ├── rubygems.rs     # RubyGems API
│   ├── docker_hub.rs   # Docker Hub API
│   └── vscode.rs       # VS Code Marketplace API
├── rank.rs             # fastembed embeddings + cosine similarity
├── verdict.rs          # Ollama prompt + response parsing
├── ollama.rs           # minimal Ollama client
└── tui.rs              # TUI state machine
src/bin/patent/
├── main.rs             # CLI entry point, pipeline wiring
├── cli.rs              # clap argument parsing
└── tui.rs              # ratatui rendering + event loop
```

Lib/bin split: the testable core is the library; the binary is a thin CLI/TUI
shell.

## Development

```bash
cargo fmt --all --check       # formatting (CI-enforced)
cargo clippy --all-targets -- -D warnings  # lint (CI-enforced)
cargo test                    # unit + wiremock integration tests
cargo build --release         # optimized build
```

### Adding a new source

1. Create `src/sources/your_source.rs` implementing the `SourceAdapter` trait
2. Add the variant to `Source` in `src/model.rs`
3. Register it in `sources::build_source` and `sources::detect_sources`
4. Add wiremock integration tests in `tests/sources.rs`

## License

Licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE) at your option.
