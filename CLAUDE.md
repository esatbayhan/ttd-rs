# Claude Code Instructions

## Project Overview

`ttd` is a terminal-first task manager for the `todo.txt.d` format. It is a single Rust binary that exposes both a CLI (`add`, `list`, `done`, `search`) and a full TUI built on `ratatui` + `crossterm`. Tasks are plain `.txt` files in a `todo.txt.d/` directory, with completed items in `done.txt.d/` and smart-list definitions in `lists.d/`. The on-disk format is governed by the [`spec/`](spec/) submodule.

## Design Philosophy

- **Terminal-native first.** Prefer idiomatic `ratatui` widgets, `crossterm` events, and standard CLI patterns over hand-rolled equivalents. Use the framework's primitives (`Block`, `List`, `Paragraph`, layout splitters, key event matching) before building custom drawing code.
- **Plain text is the source of truth.** Never invent metadata that doesn't survive a round-trip through the file format. If a feature needs new state, extend the spec first.
- **Less code is better.** The simplest correct solution wins. Three similar lines beat a premature helper.
- **Spec compliance over convenience.** Behavior of the parser, store, and smart-list engine must match what is documented in [`spec/FORMAT.md`](spec/FORMAT.md) and [`spec/LISTS.md`](spec/LISTS.md). When in doubt, read the spec; if the spec is wrong, tell me. Dont work on the spec in this project. Keep spec as read only.
- **No over-engineering.** Don't abstract prematurely, don't add config knobs nobody asked for, don't build for hypothetical future features.

## Module Layout

- [src/parser.rs](src/parser.rs) — todo.txt line parsing
- [src/task.rs](src/task.rs) — `Task` model
- [src/store.rs](src/store.rs) — filesystem-backed task store, advisory locking, conflict detection
- [src/refresh.rs](src/refresh.rs) — filesystem polling and snapshot diffing
- [src/query.rs](src/query.rs) — filter / sort / group primitives shared by CLI and TUI
- [src/smartlist/](src/smartlist/) — `.list` file parser, AST, and evaluator
- [src/tui/](src/tui/) — TUI app, render loop, editor, mouse, session
- [src/cli.rs](src/cli.rs), [src/main.rs](src/main.rs) — CLI entry point (hand-rolled arg parser, no framework)
- [src/bootstrap.rs](src/bootstrap.rs), [src/config.rs](src/config.rs) — first-run flow and config file

Tests in [tests/](tests/) are integration tests; each module has a corresponding `*_*.rs` file. TUI behavior is exercised end-to-end via `expectrl` in [tests/tui_e2e.rs](tests/tui_e2e.rs) and friends.

## Branching & Git Workflow

**Branch naming (flat):**

- `feat/<short-name>` — new feature
- `fix/<short-name>` — bug fix
- `chore/<short-name>` — tooling, deps, non-code
- `refactor/<short-name>` — internal restructuring, no behavior change

No version prefix in branch names. Version tracking lives in `Cargo.toml`, git tags, and PR labels.

**Never work directly on `main`.**

- Every code change — features, fixes, chores, refactors — happens on its own branch. Switch to (or create) the branch *before* you start editing files, not after.
- The only thing that may land on `main` directly is an explicit, user-requested merge (e.g. "merge this into main"). Without that explicit instruction, do not commit, fast-forward, or fold work into `main`.
- If you find yourself with uncommitted changes on `main`, stop and move them to a branch (`git checkout -b <type>/<short-name>`) before continuing.

**Base branch:**

- Default: branch off `main`.
- Only branch off another feature branch if the new work genuinely depends on unmerged code from it. Stacking creates coupling — the child can't merge until the parent does.

**Worktrees for parallel work:**

When multiple Claude sessions might run in parallel, use a git worktree for new branches rather than switching branches in the primary checkout:

1. `git fetch origin`
2. `git worktree add ../ttd-rs-<short-name> -b <type>/<short-name> origin/main`
3. `cd ../ttd-rs-<short-name>` and start work there.

When done: `git worktree remove ../ttd-rs-<short-name>` after the branch is merged.

**Commits, merges, and pushes:**

- **Commits are automatic.** Once work on a branch is in a sensible state, commit it without asking. Don't wait for a "please commit" — the default is *commit freely*. Same for creating branches and annotated tags.
- **Merges require explicit approval every time.** Merging into `main` (or any other branch) only happens when the user says so in the current turn. Prior approval does not carry over.
- **Pushes require explicit approval every time.** `git push` for any ref — branches, tags, force-push — only happens when the user says so in the current turn. Prior approval does not carry over.

## Pre-Commit Checks

Before committing any code change, run these and make sure they pass:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

`clippy` warnings are errors here (`-D warnings`). Don't silence them with `#[allow(...)]` unless there is a real reason — fix the underlying issue.

## Dependency Policy

**Minimize transitive dependencies.** Every crate added to `Cargo.toml` must justify itself against a hand-rolled or stdlib alternative. The bar: could a motivated programmer recreate the needed subset in an afternoon without introducing bugs?

**Current dependencies and their rationale:**

| Dependency | Why it stays | Why it's the only option |
|---|---|---|
| `ratatui` | TUI widgets, layout, rendering | No lighter TUI framework exists that provides `Paragraph`, `List`, `Block`, `Layout`, and `TestBackend`. Alternatives (cursive, tui-rs) are either heavier or unmaintained. |
| `crossterm` | Terminal raw mode, events (key, mouse, resize), alternate screen | Required by `ratatui-crossterm` backend. Also used directly for event polling and terminal control. On Linux the `windows` default feature is dead code but we can't disable it (ratatui-crossterm enables defaults). |
| `time` | `today_date()` — local date formatting | Already pulled in transitively by `ratatui-widgets` (calendar). Using it directly is zero-cost — same crate, same version. Replaces the `libc` + `unsafe` alternative we briefly tried. |
| `expectrl` (dev) | End-to-end TUI tests via PTY | The only way to test the live TUI binary. Dev-only, doesn't affect release binary size or dep count. |

**Dependencies that were removed and why:**

| Removed | Replaced by | Reason |
|---|---|---|
| `clap` (10 unique transitive crates) | `std::env::args()` hand-rolled parser in `src/cli.rs` | CLI surface is 4 subcommands + `--task-dir`. clap's derive macros, help generation, and error formatting pulled in clap_builder, anstream, anstyle, clap_lex, strsim, and proc-macro infrastructure. Hand-rolling is ~60 lines. |

**Rules for adding a dependency:**

1. Exhaust the stdlib first. `std::env::args()`, `std::process::Command`, `std::fs`, `std::time` cover a lot.
2. If a crate is already transitively present, using it directly is free. Prefer it over adding a new one.
3. Before adding a crate for one function, ask: could this be done with what's already in the tree?
4. When adding a dependency, disable unused default features. Set `default-features = false` and enumerate only what's needed.
5. Prefer crates with few transitive dependencies. A crate with 0 deps is always better than one with 50, all else equal.

**Why not go further:**

- **ratatui can't be replaced.** The TUI is the product. Rewriting widgets, layout, and rendering from scratch would be hundreds of hours and produce a worse result.
- **crossterm can't be replaced.** It's pulled in by ratatui-crossterm regardless. termion isn't lighter *and* ratatui has no production-ready termion backend.
- **expectrl stays.** The e2e tests catch real regressions. It's dev-only.
- **time is already transitively present.** Using it directly costs nothing — same crate already compiled for ratatui-widgets.

## Version Bumping

The version lives in [Cargo.toml](Cargo.toml) under `[package].version`. Bumping it also updates `Cargo.lock` (run `cargo build` or `cargo update -p ttd` after editing). The CLI reads the version via `env!("CARGO_PKG_VERSION")`, so no other location needs updating.

Semver:
- new `feat` → minor bump
- only `fix` / `chore` / `refactor` → patch bump
- breaking spec migration or CLI/TUI behavior break → major bump (see `v2.0.0` for precedent)

## Spec Submodule

The on-disk format and smart-list grammar are owned by the [`spec/`](spec/) submodule (`https://github.com/esatbayhan/todo.txt.d`). When bumping the spec submodule pointer:

1. Verify `spec/FORMAT.md` and `spec/LISTS.md` `**Version:**` headers match.
2. Re-run the full test suite — many parser tests load fixtures from `spec/examples/`, so a spec bump can change test results.
3. If the spec adds new fields or syntax, update the parser, smart-list engine, and **add fixtures** under `spec/examples/` upstream first.

The release commit message mentions the spec version when relevant (e.g. `Release v2.0.0 — Spec v2.0.0 migration`).

## Release Tagging

Every release commit that bumps the `Cargo.toml` version must be tagged. Do this without being asked.

- Tag format: `v<X.Y.Z>` (e.g. `v2.1.0`).
- Use annotated tags: `git tag -a v<X.Y.Z> -m "<short release summary>"`.
- Tag points at the version-bump commit on `main`.
- Push the tag alongside the branch (with explicit approval): `git push origin main && git push origin v<X.Y.Z>`.

## Testing Conventions

- Integration tests live in [tests/](tests/) and run against a temp directory created per test. Use `tempfile` patterns already in the suite — don't write to the real `~/.config/ttd/` or to the repo's `todo.txt.d/`.
- Parser tests load fixtures from `spec/examples/`. When adding a new format edge case, add the fixture upstream in the spec repo first, then bump the submodule.
- TUI tests (`tui_e2e`, `tui_session`, `tui_render`, `tui_mouse`, `tui_editor`) drive the binary through `expectrl` PTY — they are slow and order-sensitive. Run them last when iterating; use `cargo test --test <name>` to scope to one file.
- Don't mock the filesystem in store/refresh tests — they need real `fs::rename`, `flock`, and mtime behavior to be meaningful.

## End-to-End Fixtures

The drop-in dataset at [fixtures/e2e/](fixtures/e2e/) is the manual / exploratory counterpart to the unit and integration suites. It is one realistic `todo.txt.d/` directory plus a full set of `lists.d/` smart lists, designed so a human (or a smoke-test script) can launch the binary against it and exercise every user-visible feature in one session.

**Maintenance rule:** every change that adds, alters, or removes a user-visible feature must update the e2e fixture set in the same commit (or in the same PR series). This is non-negotiable — the fixture is only useful if it reflects current behavior.

What counts as user-visible:

- A new field on `Task` (parser or model).
- A new filter, sort, or group field in the smart-list grammar.
- A new prefill / template-variable / lenient-parsing rule.
- A new TUI keybinding, sidebar item kind, picker, or editor affordance.
- A new CLI subcommand or flag.
- A breaking change to any of the above.

When updating the fixture:

1. Add (or edit) the smallest task `.txt` or `.list` file that demonstrates the new behavior.
2. Update [fixtures/e2e/README.md](fixtures/e2e/README.md) so the feature appears in the coverage matrix and the relevant table row exists.
3. If date-relative behavior is involved, anchor the fixture to the README's stated "today" — currently `2026-04-25`. Bumping that anchor is its own coordinated change across all date-bearing fixtures.
4. Smoke-test: `cargo run -- list --task-dir fixtures/e2e/todo.txt.d` (or launch the TUI) and confirm the new behavior is observable.

Pure refactors and internal-only changes that don't surface to the user are exempt.

## Adding a New Feature

When adding a feature that touches the task model, smart-list grammar, or sidebar:

1. **Spec first.** If the feature requires new on-disk syntax, land it in the `spec/` repo and bump the submodule pointer.
2. **Parser + model.** Extend `parser.rs` and `task.rs` with the new field, with round-trip tests in `tests/parser_spec.rs`.
3. **Query layer.** Add filter / sort / group support in `query.rs` so both CLI and TUI pick it up uniformly.
4. **Smart lists.** Extend the smart-list parser ([src/smartlist/parser.rs](src/smartlist/parser.rs)) and evaluator ([src/smartlist/eval.rs](src/smartlist/eval.rs)) if the new field should be filterable.
5. **TUI surface.** Wire the field into render and editor only after the underlying layers work.
6. **CLI surface.** Add the matching argument parser branch in [src/cli.rs](src/cli.rs) for scriptability.
7. **Docs.** Update the README keybinding table and feature list if user-visible.
8. **End-to-end fixtures.** Update [fixtures/e2e/](fixtures/e2e/) and its README so the new behavior is exercisable end-to-end (see § End-to-End Fixtures above).

Don't skip layers — adding a TUI shortcut for a field the parser doesn't understand is the kind of half-finished implementation we want to avoid.
