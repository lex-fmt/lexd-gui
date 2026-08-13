# Lexed on Zed — Fork Strategy and Operations

The operating manual for this fork. The original version of this document was
the evaluation brief for the port; the evaluation is over and the questions it
posed — licensing, rebase cost, rebrand surface, depth of Lex integration —
are answered. What remains here is the invariant that governs every change,
the mechanisms that have proven out, the current state, and the discipline
for staying current with upstream.

## 1. Context and decision

Lexed is the desktop editor for [Lex](https://lex.ing), a plain-text markup
language for ideas. Its audience is **non-technical writers** — people for
whom VS Code or Neovim is a barrier, not a tool.

The previous Lexed is an Electron app wrapping Monaco. It works, but it is
feature-poor as an editor, and the incremental cost of closing that gap (file
tree, spelling, outline, robust LSP integration) is high relative to the value
delivered.

**Decision, validated: Lexed is a fork of the Zed editor.** The rationale
held up in practice:

- LSP, tree-sitter syntax, file tree, outline, and multibuffer are solid.
- Input latency is materially better than Electron or VS Code — this matters
  for a format whose pitch is responsiveness of thought.
- Zed has no sprawling extension marketplace, so the out-of-box experience is
  far less confusing than VS Code's for a non-technical user.
- Distribution size is comparable to the Electron build and roughly half of
  VS Code.
- The whole stack is Rust, matching the rest of the Lex codebase.

The Electron Lexed will be retired once the remaining work in §5 lands.

## 2. The governing invariant

> **Minimize the number of upstream files touched. Drive it toward zero.**

Every line changed in a file Zed owns is a permanent rebase conflict. Every
line added in a file Zed has never seen is free forever.

This is not a stylistic preference. It is the single variable that determines
whether staying current with upstream costs an afternoon per quarter or
becomes impossible.

Two corollaries that are easy to get wrong:

- **Do not delete features to simplify the UX.** Deletion is the most
  expensive possible form of change. Hide, don't remove.
- **Inline `if` gating is not the answer.** A conditional wrapped around code
  inside an upstream file is still a diff in that file. Ten gates in
  `editor.rs` is ten permanent conflict points — strictly worse than one
  clean deletion.

### The disable ladder

When switching something off, work down this list and stop at the first rung
that works.

| Rung | Mechanism                          | Diff cost                         |
| ---- | ---------------------------------- | --------------------------------- |
| 1    | Bundled default settings JSON      | **Near zero.** Config-only edit.  |
| 2    | Don't call its `init()` at startup | One deleted line, one file.       |
| 3    | Drop the crate from `Cargo.toml`   | Manifest-only. Trivial conflicts. |
| 4    | Cargo feature flag                 | Manifest plus attribute lines.    |
| 5    | Inline conditional                 | **Last resort.**                  |

If you reach rung 5, the upstream edit must be a single call into a function
defined in a Lex-owned crate — one line, not a block. The canonical example
is the quick action bar's preview button: its `return None` fallback became
one call into `lex_preview::toolbar`, and every future Lex preview type
registers there without another upstream edit.

### Enforcement

`script/lexed-diff-guard` counts upstream files touched (modified, deleted,
or renamed — additions are free) relative to the base commit in `ZED_BASE`,
and fails above a ceiling (currently 60). It runs in CI on every push and
pull request (`.github/workflows/lexed_diff_guard.yml`) and is runnable
locally. Raise the ceiling only deliberately, in the workflow file, with a
commit that says why.

Current shape of the diff: ~58 touched files, the large majority one-time
branding swaps (icons, `crates/zed/resources/*`, packaging metadata) that
will conflict trivially or not at all on rebase. The live source seams are a
short list: `crates/zed/src/main.rs` (two `init()` calls), the two root
manifests, `assets/settings/default.json`, the quick-action-bar fallback
call, and the identity/paths edits from the rebrand. The diff remains
readable in one sitting; keep it that way.

## 3. Established mechanisms

These patterns are proven in-tree. Reach for them, in this order, before
inventing new ones.

**Bundled default settings** (rung 1). `assets/settings/default.json` carries
the fork's behavior changes: every Zed-server surface is disabled
(`server_url` points at a dead loopback, so any regression fails loudly and
locally rather than reaching Zed Industries), telemetry/auto-update/extension
auto-install are off, and per-language defaults configure Lex —
`document_symbols: "on"` feeds outline, breadcrumbs, and the symbol picker
from lex-lsp instead of tree-sitter.

**Bundled extensions** (zero upstream). The WASM extension system is
load-bearing and stays; Lexed pre-installs what it needs.
`crates/lex_bundled_extensions` embeds the packaged
[zed-lex](https://github.com/lex-fmt/zed-lex) extension (compiled
`extension.wasm`, grammar wasm, language configs) and syncs it into
`<data>/extensions/installed/` at startup through the extension store's
normal on-disk layout — staged-and-renamed, version-compared, never
clobbering a dev-extension symlink. `script/lexed-package-lex-extension`
rebuilds the vendored artifacts from a zed-lex checkout using the in-tree
`extension_cli`.

**Additive `lex_*` crates** (manifest + one `init()` line each).
`crates/lex_preview` is the template: the preview pane gets HTML from
lex-lsp's `lex.export` workspace command and renders it as native gpui
elements — following the `svg_preview` structure, registered with one call in
`main.rs`. Note the licensing consequence of this design in §4: no Lex crates
are compiled into the editor.

**Lex-owned registries behind single-call seams** (rung 5 done right). Where
upstream hardcodes an enumeration (which file types get a preview button),
replace the fallback branch with one call into a Lex-owned registry, then
extend by registration forever after.

**Generated CI.** Zed's workflows are generated: edit
`tooling/xtask/src/tasks/workflows/*.rs`, run `cargo xtask workflows`, never
hand-edit the YAML. Upstream's heavy workflows are gated to
`zed-industries` repository owners and self-hosted runner pools, so they are
inert in this fork — the active CI today is `lexed_diff_guard.yml`. Fork CI
grows by adding new `lexed_*` generators, not by editing Zed's.

**Testing layers.** The `testing-lexed` skill (`.claude/skills/testing-lexed`)
documents the ladder: `cargo nextest` crate suites, workspace-level gpui
tests against fakes (including `FakeLspAdapter` for lex-lsp flows), the
headless visual test runner, and sandboxed real-app launches with
`--user-data-dir`. Pick the cheapest layer that proves the change.

## 4. Licensing

Settled. **Zed is triple-licensed by component**, not dual, and not
user-selectable:

- `crates/gpui/` — Apache-2.0
- `crates/collab/` and server-side — AGPL-3.0-or-later
- everything else, including the editor — **GPL-3.0-or-later**

Per-crate `LICENSE-*` symlinks are authoritative. Check them, don't assume.
Fork-added `lex_*` crates carry GPL-3.0-or-later like their neighbors.

### What this means for Lex code

- **Lexed binaries ship under GPL-3.0-or-later.** Accepted.
- **`lex-lsp` runs as a separate process** speaking JSON-RPC over stdio. That
  is an arm's-length boundary: a separate program, not a derivative work. It
  stays MIT, even when bundled in the same installer (mere aggregation).
  **Preserve this boundary** — it is a licensing asset, and it is currently
  doing extra work: the preview pane consumes lex-lsp's HTML export over the
  wire, so **no Lex crates are compiled into the editor at all**. If that
  ever changes (e.g. compiling lex-core in for offline rendering), the
  compiled-in crates keep their MIT headers, remain MIT on crates.io, and
  only the combined binary is GPL.
- **No upstream contamination.** Canonical `lex-fmt` repos stay MIT. GPL has
  no reach-back mechanism, and Arthur holds copyright regardless.

### Obligations

- **Trademark.** "Zed", its logo, and app icons are not covered by the code
  licenses. The rebrand removed them from binaries, assets, and user-visible
  strings; keep it that way (the copyright inventory in this directory tracks
  what was swept).
- **Corresponding source.** GPL-3.0 requires offering complete source to
  binary recipients. Release CI must publish the source of the exact tag
  built alongside every binary — this is a launch blocker for §5, not a
  retrofit.
- **Attribution.** Prominent acknowledgement of the Zed codebase in README
  (done), the About dialog, and the project site. No ambiguity about
  provenance.

## 5. Status: shipped and remaining

Shipped, in the order the original phases prescribed:

- **Fork setup** — `lex-fmt` fork, `ZED_BASE`, `lex_*` crate layout, diff
  guard in CI.
- **Network and account surface** — disabled via bundled defaults (§3);
  collab, AI relay, telemetry, auto-update, registry calls, and sign-in are
  all off. Verified by sandboxed launches and log inspection; a packet
  capture on a clean launch is still owed before the first public release.
- **Rebrand** — app identity, bundle id, icons (Lexed tree glyph), data
  directories, packaging resources, Zed legal docs removed.
- **Lex language support** — bundled zed-lex extension: tree-sitter
  highlighting, lex-lsp diagnostics/formatting/actions, zero configuration.
- **Preview pane (v1)** — native HTML preview with live updates, toolbar
  button, open-to-the-side.
- **LSP-backed structure** — outline, breadcrumbs, symbols via
  `documentSymbol`.

Remaining before retiring the Electron Lexed:

- **Spellcheck** — link `harper-core` into `lex-lsp` (one process, zero
  configuration). This is lex-repo work, not fork work.
- **Preview scroll-sync** with the editor.
- **Commands and menus** — a `lex_menus` crate surfacing preview, export,
  and the other context-rich operations through the application menu and
  toolbar; the command palette is a developer affordance, not the product
  surface.
- **Document-oriented UX** — open a file, Save As, Recent Documents. Still
  the largest design risk, still scoped separately, still must not drive
  deletions in core.
- **Release pipeline** — `lexed_*` release workflow with Lexed signing
  identities and corresponding-source publication.
- **Windows validation** — hands-on, not from the spec sheet.

Known upstream issues affecting Lex features:
[tree-sitter-lex#117](https://github.com/lex-fmt/tree-sitter-lex/issues/117)
(grammar mis-parses session/document titles in common shapes; outline is
LSP-backed as the durable mitigation, but highlighting and textobjects still
degrade) and [zed-lex#49](https://github.com/lex-fmt/zed-lex/issues/49)
(brackets/textobjects queries not valid for Zed).

## 6. Rebase discipline

**Do not attempt to track `main`.** Also do not freeze indefinitely.

**Pin and rebase quarterly.** The current base (`ZED_BASE`) is an upstream
snapshot commit; move the pin to upstream stable release tags at the next
rebase and stay on tags thereafter.

Reasoning: continuous rebase costs roughly linearly. Selective cherry-picking
into a tree that has diverged for a year costs superlinearly, because the
surrounding code has moved out from under the patch. A quarterly cadence
keeps each rebase small enough to stay mechanical and never enters the
superlinear regime.

The mechanical conflict resolution is not the bottleneck — an agent handles
that. **Identification is the bottleneck.** Zed has no LTS branch, no
security branch, and no CVE advisory stream. "Just merge the critical fixes"
presumes a labelled set of commits that does not exist. With the network
surface disabled, the residual exposure is dependency surface: the crate
tree, tree-sitter grammars, LSP subprocess handling, the WASM runtime. Cover
it directly:

- `cargo audit` and `cargo deny` in CI, on the fork, with no upstream
  tracking required.
- Review the upstream release notes at each quarterly rebase — cheap, and
  catches the rest.

For a local document editor, this is a defensible posture.

### Expect churn where it doesn't matter

Zed's `editor` crate is among the most heavily churned parts of the tree;
multibuffer and rendering are refactored regularly. **This costs nothing if
there is no diff there.** Conversely, a stable subsystem patched in fifteen
places will still bite. The invariant is not "upstream settles down." It is
"count the upstream files touched, and drive that number to zero."

`git rebase --onto` against the next upstream tag completing without manual
intervention in the `lex_*` and config layers is the standing test of whether
§2 is being followed; each rebase updates `ZED_BASE`.

## 7. Windows

Zed reached 1.0 in April 2026 with Windows support, but it landed later than
macOS and Linux. Zed Industries has a stated commitment, so this is a matter
of maturity rather than direction. Validate hands-on before shipping Lexed
for Windows. Accepted as a known risk.
