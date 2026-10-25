# opencode-nanogpt

A small Rust CLI that configures [OpenCode](https://opencode.ai) to use
[NanoGPT](https://nano-gpt.com) as a model provider and, optionally, enables
the NanoGPT MCP server. It edits `opencode.json` **surgically**: your
comments, formatting, and every other config section stay byte-identical.

## Install

Homebrew (once the tap is published):

```bash
brew trust --formula lunaticmuch/tap/opencode-nanogpt
brew install lunaticmuch/tap/opencode-nanogpt
```

Or download a pre-built binary from the
[releases page](https://github.com/LunaticMuch/opencode-nanogpt/releases).

## Quick start

```bash
export NANOGPT_API_KEY=sk-...

opencode-nanogpt init                      # provider added, MCP disabled
opencode-nanogpt update-models             # keeps latest 2 per family
opencode-nanogpt update-models --interactive  # pick models from a searchable TUI
opencode-nanogpt update-models --subscription-only
opencode-nanogpt update-models --no-subscription --token-multiplier 2
opencode-nanogpt update-models --per-family 1 --include 'deepseek*'
opencode-nanogpt validate                  # confirms the section is well-formed
```

Every command supports:

| Flag | Effect |
| :--- | :--- |
| `--config <PATH>` | Use a specific `opencode.json` instead of the XDG default |
| `--dry-run` | Print the resulting file to stdout; write nothing |

## How it edits your config

`opencode-nanogpt` never rewrites the whole file. It scans the JSONC,
locates the `provider.nanogpt` (or `mcp.nanogpt`) value, and replaces only
that byte span. Everything outside it, including:

- `//` line comments and `/* */` block comments
- trailing commas
- CRLF line endings and a leading BOM
- your other providers, themes, and keys

is preserved exactly. This is enforced by unit tests in
`src/config/jsonc.rs`.

## MCP modes

| `init` flag | MCP block written |
| :--- | :--- |
| *(default)* | `"enabled": false` — present but inactive |
| `--mcp` | `"enabled": true` — active on next OpenCode start |
| `--no-mcp` | Omitted entirely |

The NanoGPT MCP tools (web search, scraping, media generation) **bill your
NanoGPT balance** on every call and consume model context. The default is
`enabled: false` precisely so that turning them on is always an explicit
decision. When MCP is written, the CLI prints a cost/context reminder.

**MCP prerequisite:** the injected command is `npx -y @nanogpt/mcp`, which
requires Node.js 22+ on the machine running OpenCode. The Rust binary itself
has no runtime dependencies.

## Interactive model selection

For catalogs with hundreds of models, use the searchable TUI:

```bash
opencode-nanogpt update-models --interactive
```

- Type to filter the full list by model ID or display name.
- Arrow keys move the cursor; Space toggles a model; Enter confirms.
- Esc or Ctrl-C cancels without writing anything.
- Ctrl-S cycles the subscription filter (all / subscription / pay-as-you-go).
- Ctrl-T cycles the token-multiplier filter (all / 1x / 2x).
- Ctrl-O cycles the open-weights filter (all / open / closed).
- The highlighted model shows a detail pane with context, cost, open weights, providers, and capabilities.
- Non-interactive equivalents: `--subscription-only`, `--no-subscription`, `--token-multiplier 1|2`.
- Existing models are pre-selected, but the final config is replaced by your selection.

## Model filtering

NanoGPT exposes several hundred models. `update-models` groups them by
**family** and keeps the newest `--per-family N` (default **2**) per family.

Family = the model ID with the provider prefix and any trailing date suffix
removed:

| IDs | Family |
| :--- | :--- |
| `deepseek/deepseek-v4-flash`, `deepseek/deepseek-v4-flash-0731` | `deepseek-v4-flash` |
| `openai/gpt-4o-20240731`, `gpt-4o` | `gpt-4o` |

Distinct variants do **not** collapse: `deepseek-v4` and
`deepseek-v4-flash` are different families.

Ranking within a family prefers the API's `created` timestamp, falls back
to parsed version numbers, and finally to ID length (a dated ID outranks an
undated base). A model whose rank cannot be determined is **never dropped**.

Manual curation:

```bash
opencode-nanogpt update-models --include 'deepseek*'
opencode-nanogpt update-models --exclude '*-free'
opencode-nanogpt update-models --prune
```

By default `update-models` **preserves** any model already in your config
that the API no longer lists, so a temporary API omission cannot silently
remove a model you rely on. Pass `--prune` to opt into strict sync.

## Backups and rollback

Every non-dry-run write first copies the current file to
`opencode.json.bak.<timestamp>` (the 5 most recent are kept). To undo:

```bash
opencode-nanogpt rollback            # restore newest backup
opencode-nanogpt rollback --dry-run  # show which file would be used
```

Rollback is itself safe: it backs up the current file before restoring, so
a mistaken rollback can be undone the same way.

## Commands

| Command | Purpose |
| :--- | :--- |
| `init` | Add or update the `nanogpt` provider (+ optional MCP block) |
| `update-models` | Fetch models, filter, write `provider.nanogpt.models` |
| `validate` | Read-only check that the section is well-formed |
| `rollback` | Restore the most recent backup |
| `format` | Reserved (no-op in v0.1) |

Run `opencode-nanogpt <command> --help` for the full flag list.

## Development

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Release builds are single binaries produced by
`.github/workflows/release.yml` on any `v*` tag. The Homebrew formula lives
at `packaging/homebrew/opencode-nanogpt.rb`.
