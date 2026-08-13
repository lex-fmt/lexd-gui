# Lexed (Zed fork)

Lexed is a desktop editor for [Lex](https://lex.ing), a plain-text markup language
for ideas, aimed at **non-technical writers**.

This repository is a fork of the [Zed editor](https://github.com/zed-industries/zed)
by Zed Industries. It is **not** Zed, and it is not affiliated with or endorsed by
Zed Industries. All credit for the underlying editor — GPUI, the editor core,
tree-sitter integration, LSP support, and everything else this fork builds on —
belongs to the Zed project and its contributors.

## For agents and contributors: read this first

**The governing invariant of this fork: minimize the number of upstream files
touched. Drive it toward zero.**

Every line changed in a file Zed owns is a permanent rebase conflict. Every line
added in a file Zed has never seen (a new `lex_*` crate, a bundled config, a doc)
is free forever. Hide features via bundled default settings; don't delete them.
Never wrap upstream code in inline conditionals when a settings default or a
removed `init()` call will do.

The full strategy — the disable ladder, licensing analysis, work phases, and
rebase discipline — lives in [`docs/lexed/fork-strategy.md`](docs/lexed/fork-strategy.md).
Read it before changing anything.

The upstream commit this fork is based on is recorded in [`ZED_BASE`](ZED_BASE).
Every rebase updates it.

## Layout

- Lex-specific code lives in new crates under `crates/` named `lex_*`.
- Lexed docs live under `docs/lexed/`.
- Everything else is upstream Zed and should stay byte-identical wherever possible.

## Building

Same as upstream Zed: `cargo build` (see `docs/src/development/` for platform
setup). Use `./script/clippy` instead of `cargo clippy`.

## Licensing

Zed is licensed per component: `crates/gpui` under Apache-2.0, `crates/collab`
under AGPL-3.0-or-later, and the rest — including the editor — under
GPL-3.0-or-later. Per-crate `LICENSE-*` symlinks are authoritative.

**Lexed binaries are therefore distributed under GPL-3.0-or-later.** Lex crates
that are compiled in keep their MIT headers and remain MIT on crates.io;
`lex-lsp` runs as a separate MIT-licensed process. Release builds must publish
corresponding source alongside every binary.

"Zed", the Zed logo, and Zed app icons are trademarks of Zed Industries and are
not covered by the code licenses; they must not appear in Lexed distributions.
