# Lexed on Zed — Fork Strategy and Initial Work

Working brief for an agent doing the initial port. Read the whole document before
starting; the invariant in §2 governs every decision below it.

## 1. Context

Lexed is a desktop editor for [Lex](https://lex.ing), a plain-text markup language
for ideas. Its audience is **non-technical writers** — people for whom VS Code or
Neovim is a barrier, not a tool.

The current Lexed is an Electron app wrapping Monaco. It works, but it is
feature-poor as an editor, and the incremental cost of closing that gap (file tree,
spelling, outline, robust LSP integration) is high relative to the value delivered.

**Decision: rebase Lexed on a fork of the Zed editor.**

Rationale:

- LSP, tree-sitter syntax, file tree, outline, and multibuffer are already solid.
- Input latency is materially better than Electron or VS Code — this matters for a
  format whose pitch is responsiveness of thought.
- Zed has no sprawling extension marketplace, so the out-of-box experience is far
  less confusing than VS Code's for a non-technical user.
- Distribution size is comparable to today's Electron build and roughly half of VS Code.
- The whole stack becomes Rust, matching the rest of the Lex codebase.

## 2. The governing invariant

> **Minimize the number of upstream files touched. Drive it toward zero.**

Every line changed in a file Zed owns is a permanent rebase conflict. Every line
added in a file Zed has never seen is free forever.

This is not a stylistic preference. It is the single variable that determines
whether staying current with upstream costs an afternoon per quarter or becomes
impossible.

Two corollaries that are easy to get wrong:

- **Do not delete features to simplify the UX.** Deletion is the most expensive
  possible form of change. Hide, don't remove.
- **Inline `if` gating is not the answer.** A conditional wrapped around code inside
  an upstream file is still a diff in that file. Ten gates in `editor.rs` is ten
  permanent conflict points — strictly worse than one clean deletion.

### The disable ladder

When switching something off, work down this list and stop at the first rung that works.

| Rung | Mechanism | Diff cost |
|---|---|---|
| 1 | Bundled default settings JSON | **Zero.** No upstream source touched. |
| 2 | Don't call its `init()` at startup | One deleted line, one file. |
| 3 | Drop the crate from `Cargo.toml` | Manifest-only. Trivial conflicts. |
| 4 | Cargo feature flag | Manifest plus attribute lines. |
| 5 | Inline conditional | **Last resort.** |

If you reach rung 5, the condition must be a call to a function defined in a
Lex-owned crate, so the upstream line is a single call rather than a block.

### Success metric

The complete diff against upstream should be readable in one sitting, and consist
mostly of files upstream has never seen. Target for the initial port: **under ~20
changed lines in Zed-owned files.**

Track this. Add a CI job that reports `git diff --stat upstream/main -- <zed-owned paths>`
and fails if the touched-file count exceeds an agreed ceiling.

## 3. Licensing

Settled — no open questions, but get it right mechanically.

**Zed is triple-licensed by component**, not dual, and not user-selectable:

- `crates/gpui/` — Apache-2.0
- `crates/collab/` and server-side — AGPL-3.0-or-later
- everything else, including the editor — **GPL-3.0-or-later**

Per-crate `LICENSE-*` symlinks are authoritative. Check them, don't assume.

### What this means for Lex code

- **The Lexed fork ships under GPL-3.0-or-later.** Accepted.
- **`lex-lsp` runs as a separate process** speaking JSON-RPC over stdio. That is an
  arm's-length boundary: a separate program, not a derivative work. It stays MIT,
  even when bundled in the same installer (mere aggregation). **Preserve this
  boundary** — it is a licensing asset, not just an architectural one.
- **Lex crates compiled into the editor** (parser and renderer, needed by the preview
  pane) make *that binary* GPL. The crates themselves on crates.io remain MIT.
  Keep their MIT headers intact in-tree; downstream recipients may extract them
  under MIT.
- **No upstream contamination.** Canonical `lex-fmt` repos stay MIT. GPL has no
  reach-back mechanism, and Arthur holds copyright regardless.

### Obligations to discharge

- **Trademark.** "Zed", its logo, and app icons are *not* covered by the code
  licenses. Full rebrand required before any distribution.
- **Corresponding source.** GPL-3.0 requires offering complete source to binary
  recipients. Wire this into release CI from day one — the tag built is the tag
  published. Retrofitting is painful.
- **Attribution.** Prominent, public acknowledgement of the Zed codebase in README,
  About dialog, and the project site. No ambiguity about provenance.

## 4. Work phases

### Phase 0 — Fork setup

- Fork `zed-industries/zed` into `lex-fmt`. Add `upstream` remote.
- **Pin to a stable release tag**, not `main`.
- Establish the workspace layout: Lex additions live in new peer crates under
  `crates/`, named `lex_*`.
- Add the diff-size CI job described in §2.
- Record the base tag in a `ZED_BASE` file at repo root. Every rebase updates it.

### Phase 1 — Strip network and account surface

Remove or neutralize, using the disable ladder:

- Collab / multiplayer (`crates/collab` and client-side callers)
- AI agent panel and the hosted model relay
- Telemetry
- Auto-update (Lexed ships its own update channel)
- Extension registry network calls — see the warning below
- Zed sign-in and account UI

Leaving any of these pointed at Zed Industries' infrastructure is both a support
liability and discourteous.

> **Do not delete the WASM extension system.** It is large and load-bearing, and
> ripping it out generates permanent conflicts. Disable the extensions *UI* via
> default settings, cut the registry network calls, and pre-install the extensions
> Lexed needs. The install path comes free.

Note that much of Zed's own network-facing code disappears here, which
substantially reduces the security-patch surface discussed in §5.

### Phase 2 — Rebrand

- App name, bundle identifier, window title, About dialog.
- App icon: use the existing Lexed marketing icon.
- Icon theme: source a liberally-licensed VS Code icon pack. **Zed icon themes use a
  different schema** — expect a conversion pass, not a drop-in. Ships as an icon
  theme extension, so **zero core changes**.
- Purge Zed marks from all user-visible strings and assets.

### Phase 3 — Lex language support

Ships as a **bundled extension**, the same shape as the Harper extension.
**Zero core changes** for any of:

- Lex tree-sitter grammar
- `lex-lsp` registration (separate MIT binary, spawned as subprocess)
- Spellcheck

For spellcheck, two options — **prefer the second**:

1. Bundle the `harper-ls` binary and pre-seed settings.
2. Link `harper-core` directly into `lex-lsp`. One process, zero configuration,
   nothing for a non-technical user to discover or set up.

### Phase 4 — Lex preview pane

The only substantial core work, and it is additive.

Zed has three in-tree precedents: `markdown_preview`, `svg_preview`, and CSV
preview, each its own crate. The extension API cannot add custom file-type preview
panes, so a crate is the correct shape.

- New `crates/lex_preview/`, mirroring the `svg_preview` structure. Budget ~500 LOC
  plus tests.
- **Read both `markdown_preview` and the `markdown` crate first.** There are two
  parallel implementations (the latter backs the agent panel and other UI chrome),
  and upstream appears to be consolidating them. Choose the base that survives.
- Upstream contact: workspace member entry in root `Cargo.toml`, plus one `init()`
  call site in `crates/zed/src/main.rs`. **Nothing else.**

### Phase 5 — Commands and menus

Zed's action system plus command palette is straightforward Rust. But **the command
palette is a developer affordance** — Lexed's users need menu items and toolbar
buttons.

- New `crates/lex_menus/` registering Lex actions: generate preview, export, and the
  other context-rich operations.
- Surface them through the application menu and a toolbar, not solely the palette.
- Consider Zed tasks for anything that shells out — no core changes needed.

### Phase 6 — Document-oriented UX

**Flagged as the largest design risk, and larger than Phases 4 and 5 combined.**

Zed is project-folder-oriented. Non-technical writers expect document-oriented:
open a file, Save As, Recent Documents. Reconciling this is real design work, not a
settings change, and it should be scoped separately rather than folded into the
initial port.

Do not start this until Phases 0–5 are stable. Do not let it drive deletions in
core.

## 5. Rebase discipline

**Do not attempt to track `main`.** Also do not freeze indefinitely.

**Pin to stable tags and rebase quarterly.**

Reasoning: continuous rebase costs roughly linearly. Selective cherry-picking into a
tree that has diverged for a year costs superlinearly, because the surrounding code
has moved out from under the patch. A quarterly cadence keeps each rebase small
enough to stay mechanical and never enters the superlinear regime.

The mechanical conflict resolution is not the bottleneck — an agent handles that.
**Identification is the bottleneck.** Zed has no LTS branch, no security branch, and
no CVE advisory stream. "Just merge the critical fixes" presumes a labelled set of
commits that does not exist.

Given that Phase 1 removes most of Zed's network-facing code, the residual exposure
is largely dependency surface: the crate tree, tree-sitter grammars, LSP subprocess
handling, the WASM runtime. Cover it directly:

- `cargo audit` and `cargo deny` in CI, on the fork, with no upstream tracking required.
- Review the upstream release notes at each quarterly rebase — cheap, and catches
  the rest.

For a local document editor, this is a defensible posture.

### Expect churn where it doesn't matter

Zed's `editor` crate is among the most heavily churned parts of the tree; multibuffer
and rendering are refactored regularly. **This costs nothing if there is no diff
there.** Conversely, a stable subsystem patched in fifteen places will still bite.

The invariant is not "upstream settles down." It is "count the upstream files
touched, and drive that number to zero."

## 6. Acceptance criteria for the initial port

- [ ] Fork builds from a pinned stable tag on macOS, Linux, and Windows.
- [ ] Diff against upstream: under ~20 changed lines in Zed-owned files; all other
      changes in `lex_*` crates or bundled config.
- [ ] No outbound network traffic to Zed Industries infrastructure. Verified by
      packet capture on a clean launch, not by code reading.
- [ ] No Zed trademarks in binary, assets, or user-visible strings.
- [ ] `.lex` files open with syntax highlighting, LSP diagnostics, and spellcheck,
      with no user configuration.
- [ ] Preview pane renders Lex and scroll-syncs with the editor.
- [ ] Lex commands reachable from the application menu, not only the palette.
- [ ] Release CI publishes corresponding source alongside every binary.
- [ ] `git rebase --onto` against the next upstream stable tag completes without
      manual intervention. **This is the real test of whether §2 was followed.**

## 7. Windows

Zed reached 1.0 in April 2026 with Windows support, but it landed later than macOS
and Linux. Zed Industries has a stated commitment, so this is a matter of maturity
rather than direction.

Validate hands-on rather than from the spec sheet. Accepted as a known risk.
