# Agent Guidelines

This repository uses **Conventional Commits** for all commit messages.

## Commit message format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.

## Why

Releases are automated with [release-please](https://github.com/googleapis/release-please), which derives the next version and changelog from conventional commit messages. Non-conventional commits are ignored by the release process, so always follow this format.

## Progressive commits

Make small, focused commits as work progresses rather than one large commit at the end.
Each commit must use a conventional commit message and should represent a coherent,
reviewable change (for example: parsing, CLI flags, UI, tests, docs). Prefer multiple
progressive commits over a single squashed change.

## Examples

- `feat: add NanoGPT provider init`
- `fix: preserve trailing commas in JSONC`
- `ci: add release-please action`
- `docs: clarify MCP cost warning`