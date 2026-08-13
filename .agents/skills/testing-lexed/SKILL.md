---
name: testing-lexed
description: >-
  How an agent tests Lexed (the Zed fork) by itself, from unit tests to
  full-GUI screenshot verification and real-app launches on macOS. Use when
  verifying any Lexed change: choosing the right test layer, running
  cargo nextest correctly, driving a workspace inside a gpui test, capturing
  real rendered screenshots headlessly, launching the app with a sandboxed
  profile, or asserting no outbound network traffic.
---

# Testing Lexed

Lexed inherits Zed's test infrastructure: ~8,000 tests, a deterministic
scheduler, and full platform simulation. Almost every behavior change can be
verified without a display. Pick the cheapest layer that proves your change.

## Layer 0: compile checks

- `cargo check -p <crate>` for fast iteration; `./script/clippy` before
  committing (never bare `cargo clippy` — the script adds `--release
  --all-targets --all-features -D warnings`).

## Layer 1: crate test suites (default choice)

Use `cargo nextest`, not `cargo test` (plain cargo test hits "Too many open
files" on macOS; CI uses nextest too):

```bash
cargo nextest run -p editor                          # one crate
cargo nextest run -p editor -E 'test(some_name)'     # filter by test name
cargo nextest run -p zed --no-capture                # see stdout/logs
cargo nextest run --workspace --no-fail-fast --no-tests=warn   # what CI runs (CI-scale, ~1h cold)
cargo test --workspace --doc --no-fail-fast          # doctests
```

Config in `.config/nextest.toml` (60s slow-timeout; `db` single-threaded).
No database, network, or display needed. Exception: `crates/collab`
integration tests need postgres (`compose.yml`, `script/seed-db`) — CI only
runs them on Linux; skip locally unless collab is the subject.

**Build-mode thrash warning:** `test-support` features change feature
unification, so alternating `cargo build -p zed` and `cargo nextest run -p X`
forces multi-minute rebuilds. In a work loop, stay in one mode, or use a
separate `CARGO_TARGET_DIR` for tests.

Reproduction env vars for `#[gpui::test]` (see also the `gpui-test` skill):
`SEED=<n>` fixes the scheduler seed, `ITERATIONS=<n>` re-runs with varied
seeds, `LEAK_BACKTRACE=1` traces leaked entities,
`GPUI_RUN_UNTIL_PARKED_LOG=1` shows what `run_until_parked` waits on.

## Layer 2: workspace-level integration tests (user flows, no pixels)

A `#[gpui::test]` can boot the whole app against fakes and drive real user
flows: open a project over `FakeFs`, open files, send keystrokes, dispatch
actions, and assert editor state. There is no display and no rasterization —
assert on **state**, not pixels.

Key constructors:
- `crates/zed/src/zed.rs` `init_test(...)` — full app boot with every startup
  `init()` wired to fakes; imitate its tests (e.g. `test_open_non_existing_file`,
  `test_navigation`).
- `AppState::test(cx)` (`crates/workspace/src/workspace.rs`),
  `Workspace::test_new`, `Project::test(fs, [paths], cx)`,
  `FakeFs::insert_tree(path, json!({...}))`, `SettingsStore::test(cx)`.
- `TestAppContext`: `simulate_keystrokes("cmd-shift-p")`, `simulate_input`,
  `dispatch_action`, prompt/path-picker simulation, `run_until_parked()`,
  `advance_clock()`. `VisualTestContext` adds mouse events and
  `debug_bounds(selector)`.
- Editor assertions as marked text: `EditorTestContext::set_state` /
  `assert_editor_state` with `ˇ` cursors
  (`crates/editor/src/test/editor_test_context.rs`).
- LSP without binaries: `FakeLanguageServer` + `FakeLspAdapter`
  (`crates/lsp/src/lsp.rs`, `crates/language/src/language.rs`), or
  `EditorLspTestContext`. Essential for testing lex-lsp integration patterns.
- LLM without network: `FakeLanguageModelProvider` / `FakeLanguageModel`
  (`crates/language_model/src/fake_provider.rs`).
- Cleanest user-flow template: `crates/file_finder/src/file_finder_tests.rs`
  `test_matching_paths` (type into picker → confirm → assert active editor).

## Layer 3: real-pixel screenshots, headless (visual test runner)

`crates/zed/src/visual_test_runner.rs` boots the real app stack (real
`RealFs`, Metal renderer, fonts, SVG icons) inside a deterministic dispatcher,
renders off-screen windows to a Metal texture, and diffs PNGs. **No visible
window, no display session, no Screen Recording permission needed.** macOS
only. Verified working in this tree (~40s, 23 screenshots).

```bash
cargo build -p zed --bin zed_visual_test_runner --features visual-tests
UPDATE_BASELINE=1 ./target/debug/zed_visual_test_runner        # write baselines
VISUAL_TEST_OUTPUT_DIR=/tmp/vt ./target/debug/zed_visual_test_runner  # compare; exit 1 on mismatch
```

Baselines land in `crates/zed/test_fixtures/visual_tests/` (gitignored — no
shared golden set; CI compiles but never runs this binary). Scenarios are
hardcoded in `run_visual_tests()` — adding one means editing that binary; the
breakpoint-hover scenario (~line 1015) is the best template for scripted UI
interaction with tooltips/time control via `advance_clock()`.

To capture screenshots from your own harness instead:
`VisualTestAppContext` (`crates/gpui/src/app/visual_test_context.rs`) —
`open_offscreen_window`, `capture_screenshot`, mouse simulation. The
cross-platform seam `HeadlessAppContext`
(`crates/gpui/src/app/headless_app_context.rs`) exists but has zero in-tree
users.

## Layer 4: launching the real app

For a smoke test of the actual binary on macOS:

```bash
cargo build -p zed
SANDBOX_DIR=$(mktemp -d)
ZED_STATELESS=1 ZED_WINDOW_SIZE=1200,800 \
  ./target/debug/zed --user-data-dir "$SANDBOX_DIR" /path/to/file.lex &
```

- `--user-data-dir <DIR>` redirects **everything** (config, db, extensions,
  logs) — never pollutes the real profile. There are no ZED_CONFIG_DIR env
  vars; the flag is the only override.
- `ZED_STATELESS=1` forces in-memory DBs and skips the single-instance check
  (dev-channel builds skip it anyway, so parallel instances are fine).
- Debug builds read `assets/` from disk — settings/theme/keymap edits apply on
  relaunch without a rebuild; Rust changes need `cargo build`.
- Verify liveness by checking the process is alive after a few seconds and the
  log under `$SANDBOX_DIR` is free of panics; kill it when done. A startup
  panic (e.g. bad default settings) happens within the first ~2s.
- To open more files in a running instance, the `cli` binary (`cargo build -p
  cli`) does IPC open only — it cannot dispatch actions or read state.
- `./target/debug/zed --dump-all-actions` and `--printenv` exit immediately
  and are useful introspection hooks.
- Screenshot of a really-running app: `screencapture -l <window-id>` needs a
  real display + Screen Recording TCC grant — use Layer 3 instead for
  assertions.

## Asserting "no outbound network traffic"

- In gpui tests, HTTP is already faked (`FakeHttpClient::with_404_response()`);
  `BlockedHttpClient` (`crates/http_client/src/http_client.rs`) errors on any
  request — the fail-closed option.
- For the real binary, run with an env proxy and observe:
  `http_proxy=http://127.0.0.1:<port> https_proxy=... ./target/debug/zed ...`
  — `crates/http_proxy` provides an allowlisting proxy with a
  `ProxyEvent::RequestAttempt {host, ...}` stream naming every attempted host.
  Note env proxies only catch the HTTP client; raw sockets need
  `sandbox-exec` with a Seatbelt profile from `crates/sandbox`
  (`NetworkAccess::None` blocks at kernel level) or packet capture.
- Lexed's bundled defaults point `server_url` at a dead loopback
  (`http://127.0.0.1:1`), so any regression shows up as instant local
  connection failures in the log, not real egress.

## CI

Workflows are **generated**: edit `tooling/xtask/src/tasks/workflows/*.rs`,
then `cargo xtask workflows`. Never hand-edit `.github/workflows/run_tests.yml`.
