# Scripts

Tools for generating demo data and capturing TUI screenshots.

## Prerequisites

- [vhs](https://github.com/charmbracelet/vhs) — terminal GIF/PNG generator
  - Arch: `pacman -S vhs`
  - Go: `go install github.com/charmbracelet/vhs@latest`
- ImageMagick (`magick`) — for extracting PNG frames from GIFs
  - Arch: `pacman -S imagemagick`

## Workflow

### 1. Generate demo data

```bash
bash scripts/generate-demo-data.sh
```

Creates `demo-data/todo.txt.d/` with ~25 open tasks, 4 completed tasks, and 11
smart lists. All dates are computed relative to the current date so that
smart lists like Today, Tomorrow, Next Week, and Overdue always produce
meaningful results. The directory is `.gitignore`d — it is regenerated every
time you need it.

### 2. Build the release binary

```bash
cargo build --release
```

The tapes expect the binary at `target/release/ttd`. A release build is
necessary because vhs captures at a fixed framerate and debug builds are too
slow (the TUI may not finish rendering within the sleep window).

### 3. Capture screenshots

```bash
bash scripts/capture-screenshots.sh
```

Runs all three tapes in `scripts/tapes/` and extracts one PNG frame from each
GIF. Output ends up in `docs/screenshots/`.

Or run tapes individually:

```bash
vhs scripts/tapes/main.tape        # TUI main view with sidebar
vhs scripts/tapes/navigation.tape  # Browsing sidebar and task list
vhs scripts/tapes/editor.tape      # Inline task editor
```

### 4. Commit the PNGs and GIFs

Screenshots in `docs/screenshots/` are committed to the repo so they appear
in the README. If you update the TUI rendering, regenerate and commit.

## Tapes

Each `.tape` file is a vhs script that drives the terminal and captures
output. The general pattern:

1. `Hide` / `Show` — hide setup steps (building, clearing) from the recording.
2. Launch `ttd` with `--task-dir demo-data/todo.txt.d`.
3. Simulate keypresses to navigate the TUI.
4. Quit with `q`.

Key notes:
- All paths in tapes are relative to the **project root** (where `vhs` is
  invoked from the orchestration script).
- `Type "q"` sends the literal character `q` to the running TUI process.
- `Ctrl+d` sends Ctrl+D (shortcut for inserting today's date in the editor).
- `Escape` sends ESC (cancel editor).
- `Sleep` durations give the TUI time to render between keystrokes.

### Available tapes

| Tape | Output | What it captures |
|---|---|---|
| `main.tape` | `tui-main.gif` / `.png` | Default view with Today smart list and sidebar |
| `navigation.tape` | `tui-navigate.gif` / `.png` | Browsing smart lists (`j`/`k`), switching panes (`h`/`l`) |
| `editor.tape` | `tui-editor.gif` / `.png` | Opening the editor (`a`), typing a task, Ctrl+D date shortcut |

### PNG extraction

The `capture-screenshots.sh` script uses ImageMagick to extract a
representative frame from each GIF (~85% through the animation, well into
the TUI session). This works because vhs GIFs store only incremental frame
differences, so `magick "$gif[0]"` gives an empty background. Instead we
coalesce frames up to the target and keep the last composited frame.

## Updating screenshots after TUI changes

1. Change something in the TUI rendering or task model.
2. Run `bash scripts/capture-screenshots.sh`.
3. Inspect `docs/screenshots/` to verify the output looks right.
4. Commit the new screenshots alongside the code changes.
