# Lexed dev setup

What to install to work in this repo, and what the local quality gate checks.

## Tools

Beyond the Rust toolchain (`rust-toolchain.toml` pins it) and
[`cargo-nextest`](https://nexte.st):

| Tool         | Install                   | Used by                   |
| ------------ | ------------------------- | ------------------------- |
| `lefthook`   | `brew install lefthook`   | runs the git hooks        |
| `typos`      | `brew install typos-cli`  | pre-commit spelling check |
| `shellcheck` | `brew install shellcheck` | pre-commit shell lint     |

Then install the hooks once per clone:

```sh
lefthook install
```

`script/lexed-install-hooks` does this for you and is idempotent — agent
sessions run it automatically (see [Agent sessions](#agent-sessions)). It never
fails a session: a missing tool prints the install command and exits 0. The
hook that needs the tool is where the failure becomes loud.

## The gate

Three tiers, budgeted so that no tier is slow enough to be worth evading.
`lefthook.yml` is the source of truth.

| Hook           | Checks                                                                                                                  | Budget    |
| -------------- | ----------------------------------------------------------------------------------------------------------------------- | --------- |
| **pre-commit** | `rustfmt --check` on staged `.rs`; `typos` on staged files; `script/shellcheck-scripts error` when `script/*` is staged | seconds   |
| **pre-push**   | `script/lexed-clippy-changed`; `script/lexed-test-changed`                                                              | minutes   |
| **CI**         | whole-workspace clippy, full style pass, tests, licenses                                                                | unbounded |

### Why pre-commit checks staged files, not the tree

On a workspace this size `cargo fmt --all -- --check` takes about a minute, and
whole-tree `typos` reports 76 findings in pristine upstream code that the fork's
governing invariant says not to edit. Whole-tree checks at this tier would make
every commit either crawl or fail outright. Both run whole-tree in CI instead,
where the budget is unbounded.

`rustfmt` reads the root `rustfmt.toml` for `edition` and `style_edition`, so
checking files individually agrees with the `cargo fmt --all` that CI runs.

The shellcheck leg is the exception: it is _triggered_ by staged `script/*`
files but runs whole-directory, because the shell scripts here carry no
extension and shebang detection — which `script/shellcheck-scripts` already
does — is the only correct way to pick them out.

Prettier is deliberately not in the gate. `script/prettier` shells out to
`pnpm dlx prettier@3.5.0`, which reaches the network on every run; a pre-commit
hook that needs the network is the wrong tier for it. CI covers it.

### Changed-crate scoping

Pre-push runs are scoped to what you actually changed:

- `script/lexed-changed-crates [prefix]` prints the package names of crates
  changed against `origin/main`, reading each name from its `Cargo.toml` rather
  than assuming it matches the directory.
- `script/lexed-clippy-changed` clippies those crates (dev profile, not
  `script/clippy`'s `--release --all-features`, which is a CI-tier cost). Above
  15 changed crates it steps aside and lets CI's workspace run cover the push.
- `script/lexed-test-changed` runs `cargo nextest` for the changed `lex_*`
  crates only — the upstream suite is CI-sized.

Each takes `--list` to print what it detected without running anything.

## Rebase canary

A weekly workflow (`lexed_rebase_check.yml`) rehearses
`git rebase --onto <upstream main> $(cat ZED_BASE)` and goes red when the fork
stops replaying cleanly. It is early conflict radar, not the rebase itself —
that stays quarterly and onto upstream stable tags (fork-strategy §6).

Run the same check locally against any ref:

```sh
git fetch https://github.com/zed-industries/zed main
script/lexed-rebase-check FETCH_HEAD
```

Run those two together. `FETCH_HEAD` is whatever the last fetch of _any_ remote
left behind, and the pre-push hooks fetch `origin` — so a stale `FETCH_HEAD`
will happily rebase the fork onto the fork's own `main` and report a pile of
meaningless conflicts. The script prints the target commit's sha, date, and
subject before it starts; if that line does not look like upstream Zed, refetch.

The replay happens in a throwaway detached worktree, so your branch, working
tree, and git config are untouched either way. Exit 1 means conflict, and the
conflicting files are printed.

That bare run is textual only and takes about two seconds. Upstream can also
break the fork without touching a line the fork edited — renaming a function
the `lex_*` crates call conflicts with nothing and replays silently — so
`--semantic` adds a `cargo check --workspace` on the rebased tree and exits 3
when it fails. That build is workspace-sized, which is why it is opt-in. It is
what CI runs weekly:

```sh
script/lexed-rebase-check --semantic FETCH_HEAD
```

By default the check builds into a throwaway target directory that is deleted
afterward, so nothing lands in your checkout. Set `CARGO_TARGET_DIR` to borrow
a warm one and turn a cold workspace build into a short one — the script never
deletes a target directory you handed it.

## Agent sessions

`.claude/settings.json` and `.codex/hooks.json` each register a `SessionStart`
hook that runs `script/lexed-install-hooks`, so an agent session in a fresh
clone gets working hooks without anyone remembering to ask. The two files use
the same schema.

Codex gates repo-provided hooks behind a trust prompt, so the first session in
a clone runs without them. The root `AGENTS.md` therefore also tells agents to
run the script, which covers that first session and any agent with no hook
mechanism at all.

## Bypassing

`git commit --no-verify` and `git push --no-verify` skip the hooks. Reach for
them when the gate is wrong, not when it is inconvenient — CI runs a superset
and will find it anyway.
