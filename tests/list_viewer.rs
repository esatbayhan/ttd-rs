//! Tests for the smart-list source viewer feature.

use std::fs;
use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ttd::config::{AppConfig, ConfigPaths, resolve_editor};
use ttd::tui::app::FocusArea;
use ttd::tui::session::{SidebarItem, TuiSession};

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("ttd-list-viewer-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    path
}

fn write_inbox_list(root: &std::path::Path, body: &str) -> PathBuf {
    let lists_dir = root.join("lists.d");
    fs::create_dir_all(&lists_dir).unwrap();
    let path = lists_dir.join("1 inbox.list");
    let content = format!("---\nname: Inbox\nicon: 📥\n---\n{body}");
    fs::write(&path, content).unwrap();
    path
}

fn open_session(root: PathBuf) -> TuiSession {
    fs::create_dir_all(root.join("done.txt.d")).unwrap();
    TuiSession::open(root, "2026-04-25").unwrap()
}

// ─── Editor resolution ────────────────────────────────────────────────────────

#[test]
fn resolve_editor_prefers_config_over_env() {
    let cfg = AppConfig {
        task_dir: PathBuf::from("/tmp/x"),
        editor: Some("nvim".to_string()),
    };
    // SAFETY: env var mutation in tests; isolated to this single-threaded test.
    unsafe {
        std::env::set_var("VISUAL", "should-be-ignored");
        std::env::set_var("EDITOR", "should-be-ignored");
    }
    assert_eq!(resolve_editor(Some(&cfg)), "nvim");
    unsafe {
        std::env::remove_var("VISUAL");
        std::env::remove_var("EDITOR");
    }
}

#[test]
fn resolve_editor_falls_back_to_visual_then_editor() {
    let cfg = AppConfig {
        task_dir: PathBuf::from("/tmp/x"),
        editor: None,
    };
    unsafe {
        std::env::set_var("VISUAL", "vim-from-visual");
        std::env::remove_var("EDITOR");
    }
    assert_eq!(resolve_editor(Some(&cfg)), "vim-from-visual");

    unsafe {
        std::env::remove_var("VISUAL");
        std::env::set_var("EDITOR", "nano-from-editor");
    }
    assert_eq!(resolve_editor(Some(&cfg)), "nano-from-editor");
    unsafe {
        std::env::remove_var("EDITOR");
    }
}

#[test]
fn resolve_editor_uses_platform_default_when_nothing_set() {
    let cfg = AppConfig {
        task_dir: PathBuf::from("/tmp/x"),
        editor: None,
    };
    unsafe {
        std::env::remove_var("VISUAL");
        std::env::remove_var("EDITOR");
    }
    let resolved = resolve_editor(Some(&cfg));
    assert!(matches!(resolved.as_str(), "vi" | "notepad"));
}

// ─── Config persistence ───────────────────────────────────────────────────────

#[test]
fn config_round_trip_preserves_editor_field() {
    let root = temp_path("config-rt");
    fs::create_dir_all(&root).unwrap();
    let paths = ConfigPaths::from_root(root.clone());
    let original = AppConfig {
        task_dir: root.join("todo.txt.d"),
        editor: Some("code -w".to_string()),
    };
    original.save(&paths).unwrap();
    let loaded = AppConfig::load(&paths).unwrap();
    assert_eq!(loaded, original);
}

#[test]
fn legacy_single_line_config_loads_with_no_editor() {
    let root = temp_path("config-legacy");
    fs::create_dir_all(&root).unwrap();
    let paths = ConfigPaths::from_root(root.clone());
    fs::write(&paths.config_file, "/tmp/legacy-todo").unwrap();
    let loaded = AppConfig::load(&paths).unwrap();
    assert_eq!(loaded.task_dir, PathBuf::from("/tmp/legacy-todo"));
    assert_eq!(loaded.editor, None);
}

#[test]
fn config_with_comments_and_blank_lines_loads_correctly() {
    let root = temp_path("config-comments");
    fs::create_dir_all(&root).unwrap();
    let paths = ConfigPaths::from_root(root.clone());
    fs::write(
        &paths.config_file,
        "# pretty comment\n\n/tmp/with-comments\n\n# another comment\neditor=hx\n",
    )
    .unwrap();
    let loaded = AppConfig::load(&paths).unwrap();
    assert_eq!(loaded.task_dir, PathBuf::from("/tmp/with-comments"));
    assert_eq!(loaded.editor, Some("hx".to_string()));
}

// ─── Viewer state machine ────────────────────────────────────────────────────

#[test]
fn pressing_e_on_sidebar_smartlist_opens_viewer_with_source_content() {
    let root = temp_path("viewer-open");
    let body = "no due\nno scheduled\nno starting\n";
    let _list_path = write_inbox_list(&root, body);
    let mut session = open_session(root);
    assert_eq!(session.app().focus, FocusArea::Sidebar);
    assert_eq!(session.active_sidebar_item(), SidebarItem::SmartList(0));

    session.dispatch_key("e").unwrap();

    let viewer = session
        .app()
        .list_viewer
        .as_ref()
        .expect("viewer should be open after pressing e on a smart list");
    assert_eq!(viewer.list_name, "Inbox");
    assert!(viewer.content.contains("no due"));
    assert_eq!(viewer.scroll_offset, 0);
}

#[test]
fn pressing_e_in_viewer_signals_external_edit_and_keeps_viewer_open() {
    let root = temp_path("viewer-edit");
    write_inbox_list(&root, "no due\n");
    let mut session = open_session(root);

    session.dispatch_key("e").unwrap();
    assert!(session.app().list_viewer.is_some());

    session.dispatch_key("e").unwrap();
    let pending = session
        .take_pending_external_edit()
        .expect("pending external edit should be set");
    assert!(pending.ends_with("1 inbox.list"));
    assert!(
        session.app().list_viewer.is_some(),
        "viewer should remain open while editor child runs"
    );
}

#[test]
fn pressing_esc_in_viewer_closes_it() {
    let root = temp_path("viewer-close");
    write_inbox_list(&root, "no due\n");
    let mut session = open_session(root);

    session.dispatch_key("e").unwrap();
    assert!(session.app().list_viewer.is_some());

    session.dispatch_key("esc").unwrap();
    assert!(session.app().list_viewer.is_none());
}

#[test]
fn jk_scrolls_within_viewer() {
    let root = temp_path("viewer-scroll");
    let mut body = String::new();
    for i in 0..40 {
        body.push_str(&format!("# line {i}\n"));
    }
    body.push_str("no due\n");
    write_inbox_list(&root, &body);
    let mut session = open_session(root);

    session.dispatch_key("e").unwrap();
    assert_eq!(session.app().list_viewer.as_ref().unwrap().scroll_offset, 0);

    session.dispatch_key("j").unwrap();
    session.dispatch_key("j").unwrap();
    session.dispatch_key("j").unwrap();
    assert_eq!(session.app().list_viewer.as_ref().unwrap().scroll_offset, 3);

    session.dispatch_key("k").unwrap();
    assert_eq!(session.app().list_viewer.as_ref().unwrap().scroll_offset, 2);
}

#[test]
fn pressing_e_on_sidebar_for_non_smartlist_does_nothing() {
    let root = temp_path("viewer-non-smartlist");
    write_inbox_list(&root, "no due\n");
    fs::write(root.join("a.txt"), "Hello +Work\n").unwrap();
    let mut session = open_session(root);
    // Move sidebar selection to a Project item (auto-discovered, no source).
    session.select_sidebar_item(SidebarItem::Project("+Work".to_string()));

    session.dispatch_key("e").unwrap();
    assert!(
        session.app().list_viewer.is_none(),
        "Project sidebar item has no .list source; viewer must not open"
    );
}

#[test]
fn task_focus_e_still_edits_task_not_viewer() {
    let root = temp_path("viewer-task-focus");
    write_inbox_list(&root, "no due\n");
    fs::write(root.join("a.txt"), "Existing task\n").unwrap();
    let mut session = open_session(root);
    session.app_mut().focus = FocusArea::TaskList;

    session.dispatch_key("e").unwrap();
    assert!(
        session.app().list_viewer.is_none(),
        "TaskList focus must edit the task, never open the viewer"
    );
    assert!(
        session.app().editor.is_some(),
        "TaskList focus + e should open the task editor"
    );
}

// ─── Render ──────────────────────────────────────────────────────────────────

#[test]
fn rendered_viewer_shows_highlighted_source() {
    let root = temp_path("viewer-render");
    write_inbox_list(&root, "no due\nsort by priority asc\nprefill priority A\n");
    let mut session = open_session(root);
    session.dispatch_key("e").unwrap();

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| ttd::tui::render::render_session_frame(frame, &session))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let mut full = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            full.push_str(buffer.cell((x, y)).unwrap().symbol());
        }
        full.push('\n');
    }

    assert!(
        full.contains("Inbox") && full.contains("list source"),
        "viewer header should contain the list name"
    );
    assert!(full.contains("no due"), "body content should be visible");
    assert!(
        full.contains("sort by priority asc"),
        "directives should be visible"
    );
    assert!(full.contains("j/k scroll"), "footer hint should be visible");
}

// ─── Reload after external edit ──────────────────────────────────────────────

#[test]
fn reload_after_external_edit_picks_up_disk_changes() {
    let root = temp_path("viewer-reload");
    let path = write_inbox_list(&root, "no due\n");
    let mut session = open_session(root);
    session.dispatch_key("e").unwrap();
    assert!(
        session
            .app()
            .list_viewer
            .as_ref()
            .unwrap()
            .content
            .contains("no due")
    );

    // Simulate the external editor having written a new body to the file.
    fs::write(
        &path,
        "---\nname: Inbox\nicon: 📥\n---\nhas due\nsort by due asc\n",
    )
    .unwrap();
    session.reload_after_external_edit().unwrap();

    let viewer = session.app().list_viewer.as_ref().unwrap();
    assert!(
        viewer.content.contains("has due"),
        "viewer should reflect the new on-disk content; got {:?}",
        viewer.content
    );
    assert!(viewer.content.contains("sort by due asc"));
}
