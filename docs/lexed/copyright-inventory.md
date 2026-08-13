# Copyright and trademark inventory

Audit of everything in-tree that is copyrighted or trademarked by Zed
Industries or third parties, with verdicts and replacement plans. Verified
against in-tree license files, not assumptions. Status as of the initial port
(base `a8fafdd7ee`).

## The legal frame

Almost everything in the repo is *licensed* to us — GPL-3.0-or-later,
Apache-2.0, MIT, ISC, or OFL — and Lexed complies by shipping GPL with
corresponding source. What is **not** licensed is the trademarks: the "Zed"
name, the Z logo, and Zed's app icons. Those must not appear in anything
Lexed distributes. Copyright is mostly a non-problem; trademarks are the
whole problem, and the surface is small.

## Verdicts

### Already replaced (initial port)

App icon PNGs/ICOs, `assets/images/zed_logo.svg`, bundle identifiers,
display names, application menu, About window, single-instance dialog.

### Must go — trademark (no compliance path exists)

| Path | What | Fix |
|---|---|---|
| `assets/icons/ai_zed.svg`, `zed_agent.svg`, `zed_predict{,_up,_down,_error,_disabled}.svg` (7) | Contained the literal Zed "Z" logo path | **Done** — all seven replaced with a simplified tree glyph (trunk, forked branches, leaf dots) derived from the Lexed tree mark, drawn on the same 16×16 / stroke-1.2 grid. In-place content swaps; filenames and `IconName` mapping unchanged. `zed_predict_disabled` overlays the slash rather than slicing gaps around it — acceptable at 16 px, refine later if a designer wants. |
| `assets/badge/v0.json` | shields.io badge with the full Z logo inline | Deleted |
| `assets/images/zed_x_copilot.svg` | Zed wordmark × Copilot lockup | Deleted (unreferenced; `assets/icons/copilot.svg` covers the need) |
| `"Zed (Default)"` icon theme name | `assets/settings/default.json` + `DEFAULT_ICON_THEME_NAME` in `crates/theme/src/icon_theme.rs` | Rename to "Lexed (Default)" — small code+settings change, do together with the icon redraws |
| `crates/zed/resources/flatpak/zed.metainfo.xml.in`, `zed.desktop.in`, `windows/zed.iss`, `snap/snapcraft.yaml.in`, `Permissions.plist`, `DocumentTypes.plist` | Zed Industries naming/attribution throughout packaging resources | **Done** (branding sweep) — all display strings, publisher fields, ids, and URLs now Lexed/lex.ing. Still needed before actual Linux/Windows distribution: a *functional* pass over `script/bundle-linux`, `script/bundle-windows.ps1`, `script/flatpak/bundle-flatpak`, `script/install.sh`/`uninstall.sh`, and the release workflows, which still use the old `zed` binary/artifact names and `dev.zed.Zed*` ids; snap/flatpak templates and those scripts must be reconciled in that pass. Screenshot URLs in the metainfo point at `https://lex.ing/img/flatpak/` and need real assets before any flatpak submission. |

### Must go — misrepresentation (not copyright)

`legal/{terms,privacy-policy,subprocessors,third-party-terms}.md` were Zed
Industries' actual contracts, privacy claims, and vendor lists —
unreferenced by any build code except the installer license-screen chain
(`script/generate-terms-rtf` → `script/terms/`). All deleted; the Windows
installer script will fail loudly at its `LicenseFile` reference until real
Lexed terms exist, which is the correct failure mode.

### Fine to keep (verified licenses)

| Path group | License evidence | Notes |
|---|---|---|
| `assets/fonts/ibm-plex-sans/` | OFL 1.1 (`license.txt` in-tree), genuine unmodified IBM Plex Sans | The OFL "Plex" reserved-name problem was already solved upstream: the old "Zed Plex" derived names were deprecated (`crates/gpui/src/text_system.rs` ~1180) and `.ZedSans`/`"Zed Plex Sans"` are now pure aliases. Only alias-token renames remain (cosmetic). |
| `assets/fonts/lilex/` | OFL 1.1 (`OFL.txt` in-tree), genuine Lilex | Same. |
| `assets/icons/*.svg` — 286 of 293 | `assets/icons/LICENSES`: Lucide ISC + Feather MIT; the rest is Zed's own GPL work | Generic glyphs, no marks. **Keeping these is the recommendation** — see below. |
| `assets/icons/file_icons/` (96) | Same LICENSES file; Zed's own redraws of language marks | Nominative use of third-party marks, same posture as upstream and VS Code. |
| `assets/themes/{one,ayu,gruvbox}/` | MIT per-directory LICENSE files (GitHub Inc., Ike Ku, Gruvbox) | `"author": "Zed Industries"` in the JSONs is accurate historical attribution of the port — keep. |
| `assets/sounds/` (8) | Zed's own work per git history → GPL | |
| `assets/icons/ai_*.svg`, `copilot*.svg`, forge icons (~27) | Zed's GPL drawings of third-party marks | Nominative use — keep. |
| Keymaps, prompts, settings templates | Zed's own → GPL | |

**GPL-compliance warning:** `assets/icons/LICENSES`, `assets/themes/LICENSES`,
`assets/themes/*/LICENSE`, and `assets/fonts/*/{license.txt,OFL.txt}` are
embedded in the binary by RustEmbed and MUST be retained — ISC/MIT/OFL all
require notices to travel with the work. Deleting them during any cleanup
would create the only actual license violation available here.

### Optional / polish (no legal exposure)

- The 4 `zed_*` icons with no Z mark (`zed_assistant.svg`, `zed_agent_two.svg`,
  `zed_src_custom.svg`, `zed_src_extension.svg`) — rename someday; art is fine.
- `zed://` URL scheme (~20 call sites), `.zed/` project dir (~15 call sites) —
  trademark-adjacent at most. `.zed/` renaming would break existing projects
  for zero legal benefit; leave it.
- `"$schema": "https://zed.dev/schema/..."` lines in theme JSONs — dead URLs,
  cosmetic.
- Zed-plan stamp images (`assets/images/*_stamp.svg`, `grid.svg`) — Zed's own
  GPL art, no marks; they decorate the (now-dead) Zed subscription UI and can
  be deleted with that UI later.

## Icon replacement: recommendation

**Do not do a wholesale icon-set swap.** The audit removes the reason for
one: the in-tree set is ISC/GPL-clean, and it is stylistically better than
any candidate replacement — Zed's icons are natively 16×16 at
stroke-width 1.2, while Lucide/Phosphor/Tabler/Material Symbols are 24×24 at
stroke 2, so downscaling produces off-grid hairlines. The total required work
is **7 SVG edits**, five of which substitute one shared Z path.

The TOML-mapping conversion pipeline the fork brief hoped for is therefore
designed but deliberately not built. If a future full visual rebrand wants
it, the shape is:

- `tooling/icon-remap/mapping.toml`: `[meta]` (source set, vendored checkout
  path, `target_grid = 16`, `target_stroke = 1.2`) plus one `[icons]` row per
  `IconName` stem — the stem **is** the target filename
  (`crates/icons/src/icons.rs` `path()` formats `icons/{stem}.svg`, and two
  tests enforce the enum↔file bijection in both directions).
- A normalizer script that rewrites viewBox to `0 0 16 16`, rescales
  stroke-width to 1.2 *after* the transform, and **preserves
  `stroke="black"`** — GPUI renders icons as monochrome masks keyed off
  black; converting to `currentColor` yields invisible icons (only 1 of 293
  in-tree icons uses it).
- Mapping + script + vendored source set are all new files (zero upstream
  diff); only the final SVG content-swaps touch upstream files, one
  self-contained file each.
- Fully mechanical: normalization, placement, bijection checks. Needs eyes:
  per-icon source choice, fill-based/multi-path/knockout art.

File-type icons: an icon-theme *extension* can restyle file icons (Material
Icon Theme and vscode-icons are MIT and convertible — Zed's icon-theme JSON
schema differs from VS Code's, so budget a converter), but note UI icons are
unreachable by extensions: `IconTheme` only carries file/directory/chevron
icons, so `IconName` glyphs can only change by editing `assets/icons/`.
