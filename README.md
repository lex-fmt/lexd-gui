# Lexed

Lexed is the desktop editor for [Lex](https://lex.ing), a plain-text markup
language for ideas, aimed at **non-technical writers**.

It is built on a fork of the [Zed editor](https://github.com/zed-industries/zed)
by Zed Industries. It is **not** Zed, and it is not affiliated with or endorsed
by Zed Industries. All credit for the underlying editor — GPUI, the editor core,
tree-sitter integration, LSP support, and everything else this fork builds on —
belongs to the Zed project and its contributors.

This fork is the path forward for Lexed. The questions that made it an
evaluation — licensing, rebase cost, rebrand surface, depth of Lex
integration — have been worked through and answered; the current
Electron-based Lexed will be retired once the remaining polish lands.

## The governing invariant

**Minimize the number of upstream files touched. Drive it toward zero.**

Every line changed in a file Zed owns is a permanent rebase conflict. Every line
added in a file Zed has never seen (a new `lex_*` crate, a bundled config, a doc)
is free forever. Hide features via bundled default settings; don't delete them.
Never wrap upstream code in inline conditionals when a settings default or a
removed `init()` call will do.

The full playbook — the disable ladder, licensing analysis, work phases, and
rebase discipline — lives in
[`docs/lexed/fork-strategy.md`](docs/lexed/fork-strategy.md). Read it before
changing anything. `script/lexed-diff-guard` enforces the invariant in CI on
every push and pull request.

The upstream commit this fork is based on is recorded in [`ZED_BASE`](ZED_BASE).
Every rebase updates it.

## What Lexed adds

- **Bundled Lex support.** `crates/lex_bundled_extensions` pre-installs the
  [zed-lex](https://github.com/lex-fmt/zed-lex) extension (tree-sitter grammar,
  the `lex-lsp` language server) at first launch — no network access or user
  action required. The vendored artifacts are rebuilt with
  `script/lexed-package-lex-extension`.
- **Native preview.** `crates/lex_preview` renders lex-lsp's HTML export as
  native gpui elements in a live-updating pane (`lex: open preview`, the
  toolbar eye button, or open-to-the-side with alt-click).
- **LSP-backed structure.** Outline, breadcrumbs, and the symbol picker for
  Lex documents come from lex-lsp's `documentSymbol`, via bundled default
  settings.
- **No Zed-server surfaces.** Telemetry, collab, sign-in, auto-update, and
  extension-registry traffic are disabled through bundled defaults;
  `server_url` points at a dead loopback address, so regressions fail loudly
  and locally.
- **Lexed identity.** App name, bundle identifier, icons, and data
  directories.

## Layout

- Lex-specific code lives in new crates under `crates/` named `lex_*`.
- Lexed docs live under `docs/lexed/`; fork-specific tooling is `script/lexed-*`.
- Everything else is upstream Zed and stays byte-identical wherever possible.

## Building and testing

Same as upstream Zed: `cargo build -p zed` (see `docs/src/development/` for
platform setup). Use `./script/clippy` instead of `cargo clippy`, and
`script/lexed-test` rather than `cargo test` or a bare `cargo nextest run` —
it is a thin wrapper that applies the fork's test policy, so it answers the
same way CI does. It takes nextest's arguments: `script/lexed-test` for the
workspace, `script/lexed-test -p lex_preview` for one crate.

That policy — which upstream tests this fork excludes and which ones get a
longer per-test budget, each with the reason beside it — lives in
[`.config/lexed-nextest.toml`](.config/lexed-nextest.toml). It is a layer
*under* Zed's own `.config/nextest.toml`, which the fork does not edit; see
§3 of [`docs/lexed/fork-strategy.md`](docs/lexed/fork-strategy.md) for how
that layering works and when to prefer a budget raise to an exclusion.

The project's testing layers — from crate tests through workspace-level gpui
tests to sandboxed real-app launches with `--user-data-dir` — are documented
in the `testing-lexed` skill under `.claude/skills/`.

Local tooling and the pre-commit/pre-push gate are described in
[`docs/lexed/dev-setup.md`](docs/lexed/dev-setup.md); run `lefthook install`
once per clone.

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
