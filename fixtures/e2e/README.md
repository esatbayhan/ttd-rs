# End-to-End Fixture Set

Drop-in fixture data for exercising every user-visible feature of `ttd`
end-to-end. Each task and smart list demonstrates a specific aspect of
the on-disk format and smart-list grammar. Date metadata is anchored
around **2026-04-25** so date-relative smart lists (`Today`, `Upcoming`,
`Stale`) have meaningful content.

## How to use

Run the CLI or TUI against the fixture root. The `--task-dir` flag is a
one-shot override that does **not** modify your persisted config — handy
for exploring the fixtures without disturbing your real `~/.config/ttd/`.

```bash
cargo run -- --task-dir fixtures/e2e/todo.txt.d list   # CLI
cargo run -- --task-dir fixtures/e2e/todo.txt.d        # TUI
```

`TTD_TASK_DIR` works as an alternative to the flag.

To exercise destructive features (mark done, edit, delete) without
dirtying the repo, copy the fixture to a temp dir first:

```bash
cp -r fixtures/e2e/todo.txt.d /tmp/ttd-e2e
cargo run -- --task-dir /tmp/ttd-e2e
```

### Smart-list source viewer

With sidebar focus on a smart list (any of the rows in the sidebar except
auto-generated `+Project` / `@Context` items), press `e` to open a syntax-
highlighted view of the underlying `.list` source. Inside the viewer:

- `j` / `k` — scroll
- `e` — open the file in your `$EDITOR` (resolved as `editor=` config
  → `$VISUAL` → `$EDITOR` → `vi`/`notepad`); the viewer reloads when
  the editor exits
- `esc` / `q` — close

Most fixture lists have prefill / template-variable / lenient-parsing
content that's worth inspecting through the viewer:

- `7 Work Inbox.list` — multi-key prefill (`project`, `context`)
- `9 This Week.list` — all four prefill kinds plus a timed relative due offset
- `ttd/bugs.list` — `{{dir}}` template variable in both a filter and a
  prefill value
- `invalid/dir-out-of-range.list` — out-of-range template; the list is
  silently invalidated but the viewer still shows the (broken) source

If your binary's "today" doesn't match 2026-04-25, the `Today` /
`Upcoming` / `Stale` lists may show different counts than this README
predicts — that's expected.

## Layout

```
fixtures/e2e/
└── todo.txt.d/
    ├── *.txt                       open tasks (each file = one parser feature)
    ├── done.txt.d/
    │   └── *.txt                   completed tasks
    └── lists.d/
        ├── 1 Today.list … 13 No Time Due.list …  pinned (root) smart lists
        ├── A Group By Project.list  …
        ├── B Excludes Test.list
        ├── ttd/
        │   ├── bugs.list           grouped lists using {{dir}}
        │   └── features.list
        ├── work/projects/
        │   └── active.list         deep group using {{dir:1}}
        └── invalid/
            └── *.list              lenient-parsing fixtures
```

## Open tasks (`todo.txt.d/*.txt`)

| File                                  | Feature exercised                                                              |
|---------------------------------------|--------------------------------------------------------------------------------|
| `01-plain.txt`                        | Plain description, no metadata                                                 |
| `02-priority-creation.txt`            | Priority `(A)` + creation date                                                 |
| `03-project-context.txt`              | Single `+project` and `@context`                                               |
| `04-multi-projects-contexts.txt`      | Multiple projects and contexts in one task                                     |
| `05-due.txt`                          | `due:YYYY-MM-DD` tag                                                           |
| `06-scheduled.txt`                    | `scheduled:YYYY-MM-DD` tag                                                     |
| `07-starting.txt`                     | `starting:YYYY-MM-DD` tag (not yet actionable)                                 |
| `08-updated.txt`                      | `updated:YYYY-MM-DD` tag (spec v2.1.0)                                         |
| `09-custom-tag.txt`                   | Custom non-date tag (`progress:50`)                                            |
| `10-project-at-start.txt`             | `+project` at the very start of description                                    |
| `11-context-at-start.txt`             | `@context` at the very start of description                                    |
| `12-multi-tags.txt`                   | All four defined date tags on one task                                         |
| `13-non-date-tags.txt`                | Two custom non-date tags                                                       |
| `14-email-not-context.txt`            | `@` not preceded by space → not a context                                      |
| `15-math-not-project.txt`             | `+` not preceded by space → not a project                                      |
| `16-long-description.txt`             | Long line for TUI wrapping behavior                                            |
| `17-overdue.txt`                      | Overdue task (due in the past)                                                 |
| `18-future-starting.txt`              | Future `starting` tag — sorted to the bottom                                   |
| `19-ttd-bug.txt` `20-ttd-feature.txt` | Tasks for the `ttd` group lists (`+ttd @bug`, `+ttd @feature`)                 |
| `21-due-today.txt`                    | Hits `Today` smart list via `due:2026-04-25`                                   |
| `22-scheduled-today.txt`              | Hits `Today` smart list via `scheduled:2026-04-25`                             |
| `23-due-soon.txt`                     | Hits `Upcoming` smart list (due within 7 days)                                 |
| `24-no-priority.txt`                  | Open task with no priority — used by `Work Inbox` filter                       |
| `25-lowercase-priority.txt`           | **Lenient:** `(b)` lowercase — stays in description, not a priority            |
| `26-wrong-date-format.txt`            | **Lenient:** `04-20-2026` not `YYYY-MM-DD` — stays in description              |
| `27-duplicate-due.txt`                | **Duplicate keys:** first `due:` wins; second stays in description             |
| `28-malformed-due.txt`                | **Lenient:** `due:next-week` not a date — stays in description                 |
| `29-time-general-tag.txt`             | `time:09:00` general tag with colon in value (tag-value spec v3.0.0)           |
| `30-priority-z.txt`                   | Priority `Z` (boundary of A–Z range)                                           |
| `31-due-with-time.txt`                | `due:YYYY-MM-DDTHH:MM` tag with time (spec v3.0.0)                             |
| `32-scheduled-with-time.txt`          | `scheduled:YYYY-MM-DDTHH:MM` tag with time (spec v3.0.0)                       |
| `33-updated-with-time.txt`            | `updated:YYYY-MM-DDTHH:MM` tag with time (spec v3.0.0)                         |
| `34-due-this-evening.txt`             | `due:2026-04-25T19:00` — due today after 6 PM, hits Evening After Six list     |
| `35-creation-date-with-time.txt`      | `creation_date` with optional `THH:MM` time (spec v3.0.0)                      |
| `36-starting-with-time.txt`           | `starting:YYYY-MM-DDTHH:MM` tag with time (future, spec v3.0.0)                |

## Done tasks (`done.txt.d/*.txt`)

| File                          | Feature exercised                                          |
|-------------------------------|------------------------------------------------------------|
| `done-01-minimal.txt`         | `x YYYY-MM-DD description` (completion date only)          |
| `done-02-both-dates.txt`      | `x completion-date creation-date description`              |
| `done-03-with-metadata.txt`   | Done task carrying project, context, and `due` tag         |
| `done-04-with-updated.txt`    | Done task carrying `updated` tag                           |
| `done-05-completed-with-time.txt` | Done task with time on `completion_date` (spec v3.0.0)   |

## Smart lists (`lists.d/*.list`)

### Pinned (root)

| File                       | Filter / directive coverage                                                                |
|----------------------------|--------------------------------------------------------------------------------------------|
| `1 Today.list`             | `due = today` **OR** `scheduled = today`; `sort by priority asc`                           |
| `2 Inbox.list`             | `no due` AND `no scheduled` AND `no starting`; `sort by creation_date asc`                 |
| `3 Upcoming.list`          | `due <= today + 7`; `group by due asc`, `sort by due asc`                                  |
| `4 Stale.list`             | `updated < today - 30` **OR** `no updated` (covers `updated` field, spec v2.1.0)           |
| `5 Done.list`              | `done` (auto-includes done.txt.d/); `sort by description asc`                              |
| `6 Year End.list`          | **Absolute date anchor:** `due <= 2026-12-31`; `prefill due 2026-12-31-3` (offset on date) |
| `7 Work Inbox.list`        | `project includes` + `no priority`; **prefill** project + context                          |
| `8 High Priority.list`     | `priority above C`; multi-key sort (`priority asc`, then `creation_date asc`)              |
| `9 This Week.list`         | `due <= today + 7` + `not done`; **prefill** priority + timed relative due (`today + 3 T10:00`) + scheduled + starting |
| `A Group By Project.list`  | `has project`; `group by project asc`, `sort by priority asc` (group + sort)               |
| `B Excludes Test.list`     | `project excludes Test` + `description excludes Test` (`excludes` operator)                |
| `10 Today Until Noon.list` | **Time-aware anchor:** `due <= todayT12:00` OR `scheduled <= todayT12:00` (spec v3.0.0)   |
| `11 Evening Tasks.list`    | **Time-refinement:** `has time due` OR `has time scheduled`; group by due (spec v3.0.0)   |
| `12 Evening After Six.list` | **Time-aware comparison:** `due > todayT18:00` (spec v3.0.0)                              |
| `13 No Time Due.list`      | **Time-refinement:** `no time due` requires a date-only `due` field (spec v3.0.0)          |

### Grouped (template variables)

| File                                       | Coverage                                                                |
|--------------------------------------------|-------------------------------------------------------------------------|
| `ttd/bugs.list`                            | `{{dir}}` resolves to `ttd`; prefill uses `{{dir}}` + `@bug`            |
| `ttd/features.list`                        | Same group, demonstrates two lists sharing a directory                  |
| `work/projects/active.list`                | `{{dir:1}}` ancestor variable; deep nesting; `group by context asc`     |

### Invalid (lenient parsing)

| File                                    | Behavior                                                                              |
|-----------------------------------------|---------------------------------------------------------------------------------------|
| `invalid/empty-body.list`               | No conditions → matches no tasks; list still appears in sidebar                       |
| `invalid/dir-out-of-range.list`         | `{{dir:5}}` escapes `lists.d/` → list silently invalidated                            |
| `invalid/unknown-filter.list`           | `foobar baz quux` ignored; remaining valid filters still apply                        |
| `invalid/prefill-malformed-date.list`   | First malformed `prefill due` discarded; second valid one wins (slot not consumed)    |
| `invalid/prefill-unknown-field.list`    | `prefill foo bar` ignored; subsequent valid prefill still applies                     |
| `invalid/prefill-duplicate-scalar.list` | First valid scalar prefill wins; later duplicates ignored                             |
| `invalid/missing-name.list`             | No `name` in frontmatter → display name falls back to filename stem                   |

## Coverage matrix

The fixture aims to hit **every rule** in the spec at least once. When
adding a new spec feature or implementation behavior, add a fixture file
here as part of the same change — see `docs/projects/ttd-rs.md`
§ "End-to-End Fixtures" in the ttd-workbench.

| Feature | Behavior | Exercise via |
|---|---|---|
| Editor syntax highlighting | Colored tokens in add/edit editor input | `a` / `e` modal |
