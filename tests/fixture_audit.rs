//! End-to-end audit of `fixtures/e2e/`.
//!
//! Asserts that every smart list in the fixture evaluates to the contents
//! the README claims, and that every documented spec rule has at least
//! one corresponding task or list.
//!
//! Anchored at today = 2026-04-25 (matches `fixtures/e2e/README.md`).

use std::path::Path;
use ttd::smartlist::{
    Field, Prefill, SmartList, evaluate, group_by_directives, has_done_filter, load_all,
    resolve_date_value,
};
use ttd::store::{Snapshot, StoredTask, TaskStore};
use ttd::tui::session::{SidebarItem, TuiSession};

const FIXTURE_ROOT: &str = "fixtures/e2e/todo.txt.d";
const TODAY: &str = "2026-04-25";

fn load_fixture() -> (Snapshot, Vec<SmartList>) {
    let store = TaskStore::open(Path::new(FIXTURE_ROOT).to_path_buf()).unwrap();
    let snapshot = store.load_all().unwrap();
    let lists = load_all(&store.lists_dir());
    (snapshot, lists)
}

fn find_list<'a>(lists: &'a [SmartList], name: &str) -> &'a SmartList {
    lists
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("smart list {name:?} not found in fixture"))
}

fn evaluate_list(list: &SmartList, snapshot: &Snapshot) -> Vec<StoredTask> {
    let mut all: Vec<StoredTask> = snapshot.open_tasks.clone();
    if has_done_filter(list) {
        all.extend(snapshot.done_tasks.iter().cloned());
    }
    evaluate(list, &all, TODAY)
}

fn descriptions(tasks: &[StoredTask]) -> Vec<String> {
    tasks.iter().map(|t| t.task.description.clone()).collect()
}

fn assert_contains(tasks: &[StoredTask], needle: &str) {
    assert!(
        descriptions(tasks).iter().any(|d| d.contains(needle)),
        "expected task containing {needle:?} in result; got {:?}",
        descriptions(tasks)
    );
}

fn assert_excludes(tasks: &[StoredTask], needle: &str) {
    assert!(
        !descriptions(tasks).iter().any(|d| d.contains(needle)),
        "did not expect task containing {needle:?} in result; got {:?}",
        descriptions(tasks)
    );
}

// ─── Snapshot sanity ──────────────────────────────────────────────────────────

#[test]
fn fixture_has_30_open_tasks_and_4_done() {
    let (snapshot, _) = load_fixture();
    assert_eq!(snapshot.open_tasks.len(), 30, "open task count");
    assert_eq!(snapshot.done_tasks.len(), 4, "done task count");
}

#[test]
fn fixture_loads_21_smart_lists() {
    let (_, lists) = load_fixture();
    assert_eq!(lists.len(), 21, "smart list count");
}

// ─── Pinned (root) lists ──────────────────────────────────────────────────────

#[test]
fn today_list_returns_due_or_scheduled_today() {
    let (snapshot, lists) = load_fixture();
    let result = evaluate_list(find_list(&lists, "Today"), &snapshot);
    assert_eq!(result.len(), 2, "Today list expected 2 matches at {TODAY}");
    assert_contains(&result, "Submit weekly report");
    assert_contains(&result, "Lunch with mentor");
    // Sort by priority asc → priority B (Submit weekly report) before no-priority
    assert!(result[0].task.description.contains("Submit weekly report"));
}

#[test]
fn inbox_list_returns_tasks_with_no_date_metadata() {
    let (snapshot, lists) = load_fixture();
    let result = evaluate_list(find_list(&lists, "Inbox"), &snapshot);
    // Excluded (have at least one of due/scheduled/starting):
    // 05, 06, 07, 12, 16, 17, 18, 21, 22, 23, 27 → 11 tasks.
    // 30 - 11 = 19 tasks.
    assert_eq!(result.len(), 19);
    assert_contains(&result, "Buy bread");
    assert_excludes(&result, "Submit conference proposal"); // 05 has due
    assert_excludes(&result, "Coffee with Alex"); // 06 has scheduled
    assert_excludes(&result, "Annual review prep"); // 07 has starting+due
}

#[test]
fn upcoming_list_groups_by_due_date() {
    let (snapshot, lists) = load_fixture();
    let list = find_list(&lists, "Upcoming");
    let matched = evaluate_list(list, &snapshot);
    // due ≤ today + 7 = 2026-05-02
    // 05 (May 1), 17 (Apr 1), 21 (Apr 25), 23 (Apr 29), 27 (Apr 30)
    assert_eq!(matched.len(), 5);
    assert_contains(&matched, "Submit conference proposal");
    assert_contains(&matched, "Submit overdue paperwork");
    assert_contains(&matched, "Submit weekly report");
    assert_contains(&matched, "Renew library card");
    assert_contains(&matched, "Plan retro");

    let groups = group_by_directives(&list.group_directives, &matched);
    // 5 unique due dates → 5 groups
    assert_eq!(groups.len(), 5);
    // Sorted asc by due — first group should be 2026-04-01
    assert!(groups[0].label.contains("2026-04-01"));
}

#[test]
fn stale_list_returns_tasks_without_recent_review() {
    let (snapshot, lists) = load_fixture();
    let result = evaluate_list(find_list(&lists, "Stale"), &snapshot);
    // updated < 2026-03-26 OR no updated
    // Open tasks WITH updated: 08 (2026-04-22), 12 (2026-04-23) — both fresh, excluded
    // So stale = open tasks without updated = 30 - 2 = 28
    assert_eq!(result.len(), 28);
    assert_contains(&result, "Buy bread");
    assert_excludes(&result, "Review goals"); // 08 has fresh updated
    assert_excludes(&result, "Plan team offsite"); // 12 has fresh updated
}

#[test]
fn done_list_includes_all_done_tasks() {
    let (snapshot, lists) = load_fixture();
    let result = evaluate_list(find_list(&lists, "Done"), &snapshot);
    assert_eq!(result.len(), 4);
    let descs = descriptions(&result);
    // Sort by description asc
    assert!(descs[0].starts_with("Finish onboarding"));
    assert!(descs[1].starts_with("Refactor parser"));
    assert!(descs[2].starts_with("Review architecture docs"));
    assert!(descs[3].starts_with("Ship release"));
}

#[test]
fn year_end_list_uses_absolute_date_anchor_and_prefill() {
    let (snapshot, lists) = load_fixture();
    let list = find_list(&lists, "Year End");
    let result = evaluate_list(list, &snapshot);
    // due ≤ 2026-12-31 — every task with a due tag
    assert_eq!(result.len(), 8);
    // Prefill due 2026-12-31-3 → resolves to 2026-12-28
    let prefill_due = list.prefill.due.as_ref().expect("prefill due present");
    assert_eq!(resolve_date_value(prefill_due, TODAY), "2026-12-28");
}

#[test]
fn work_inbox_returns_unpriotized_work_tasks_with_prefill() {
    let (snapshot, lists) = load_fixture();
    let list = find_list(&lists, "Work Inbox");
    let result = evaluate_list(list, &snapshot);
    // +Work AND no priority → 14, 16, 24, 27, 29 = 5 tasks
    assert_eq!(result.len(), 5);
    assert_contains(&result, "Email john@example.com about renewal");
    assert_contains(&result, "Stretch goal idea");
    assert_contains(&result, "Plan retro"); // 27, no priority, has due
    // Prefill carries project Work + context office
    assert_eq!(list.prefill.projects, vec!["Work"]);
    assert_eq!(list.prefill.contexts, vec!["office"]);
    assert_eq!(list.prefill.priority, None);
}

#[test]
fn high_priority_list_returns_a_or_b_only_with_multikey_sort() {
    let (snapshot, lists) = load_fixture();
    let result = evaluate_list(find_list(&lists, "High Priority"), &snapshot);
    // Priority A or B: 02, 04, 05, 12, 17, 21, 26 = 7 tasks
    assert_eq!(result.len(), 7);
    // Sort by priority asc, then creation_date asc.
    // A first (4 tasks), then B (3 tasks). Within A, oldest creation first
    // (17 = 2026-03-01), then 02 and 05 (both 2026-04-20, file order), then 26 (no creation).
    let priorities: Vec<char> = result
        .iter()
        .map(|t| t.task.priority.expect("priority present"))
        .collect();
    assert_eq!(priorities, vec!['A', 'A', 'A', 'A', 'B', 'B', 'B']);
    assert!(
        result[0]
            .task
            .description
            .contains("Submit overdue paperwork")
    );
}

#[test]
fn this_week_list_carries_full_prefill() {
    let (snapshot, lists) = load_fixture();
    let list = find_list(&lists, "This Week");
    let result = evaluate_list(list, &snapshot);
    assert_eq!(result.len(), 5);
    // Prefill: priority A, due today+7 (= 2026-05-02), scheduled today (= 2026-04-25)
    assert_eq!(list.prefill.priority, Some('A'));
    assert_eq!(
        resolve_date_value(list.prefill.due.as_ref().unwrap(), TODAY),
        "2026-05-02"
    );
    assert_eq!(
        resolve_date_value(list.prefill.scheduled.as_ref().unwrap(), TODAY),
        "2026-04-25"
    );
}

#[test]
fn group_by_project_groups_into_9_buckets() {
    let (snapshot, lists) = load_fixture();
    let list = find_list(&lists, "By Project");
    let matched = evaluate_list(list, &snapshot);
    // has project: 30 - 4 (01, 02, 06, 22) = 26
    assert_eq!(matched.len(), 26);
    let groups = group_by_directives(&list.group_directives, &matched);
    // Expect 9 distinct first-projects: Admin, Errands, Family, Health,
    // Personal, Reading, Test, Work, ttd
    assert_eq!(groups.len(), 9);
}

#[test]
fn excludes_test_list_filters_out_test_tasks() {
    let (snapshot, lists) = load_fixture();
    let result = evaluate_list(find_list(&lists, "Not Test"), &snapshot);
    // Excludes anything with project Test (25, 26, 28) AND anything containing
    // the word "test" case-insensitively in its description (16 — long
    // description happens to contain "to test"). 30 - 4 = 26.
    assert_eq!(result.len(), 26);
    assert_excludes(&result, "Lowercase priority stays");
    assert_excludes(&result, "Wrong creation date format");
    assert_excludes(&result, "Try thing");
    assert_excludes(&result, "Detailed task description"); // 16, "to test"
}

// ─── Grouped (template-variable) lists ────────────────────────────────────────

#[test]
fn ttd_bugs_uses_dir_template_for_filter_and_prefill() {
    let (snapshot, lists) = load_fixture();
    let list = find_list(&lists, "Bugs");
    assert_eq!(list.group_path, vec!["ttd"]);
    let result = evaluate_list(list, &snapshot);
    // +ttd AND @bug → only 19 (Fix sidebar flicker)
    assert_eq!(result.len(), 1);
    assert_contains(&result, "Fix sidebar flicker");
    // Prefill resolves {{dir}} to "ttd"
    assert_eq!(list.prefill.projects, vec!["ttd"]);
    assert_eq!(list.prefill.contexts, vec!["bug"]);
}

#[test]
fn ttd_features_uses_dir_template() {
    let (snapshot, lists) = load_fixture();
    let result = evaluate_list(find_list(&lists, "Features"), &snapshot);
    assert_eq!(result.len(), 1);
    assert_contains(&result, "Add multi-select");
}

#[test]
fn deep_nested_list_uses_dir_n_ancestor_template() {
    let (snapshot, lists) = load_fixture();
    let list = find_list(&lists, "Active in Work");
    assert_eq!(list.group_path, vec!["work", "projects"]);
    let result = evaluate_list(list, &snapshot);
    // {{dir:1}} = "work" matches +Work case-insensitively → 9 tasks
    assert_eq!(result.len(), 9);
    // Prefill {{dir:1}} resolves at parse time to "work"
    assert_eq!(list.prefill.projects, vec!["work"]);
    // Group by context — 3 of those tasks have @meeting (12, 16, 29)
    let groups = group_by_directives(&list.group_directives, &result);
    let meeting = groups
        .iter()
        .find(|g| g.label == "Context: meeting")
        .expect("meeting group present");
    assert_eq!(meeting.tasks.len(), 3);
}

// ─── Invalid (lenient parsing) ────────────────────────────────────────────────

#[test]
fn empty_body_list_loads_with_zero_blocks() {
    let (snapshot, lists) = load_fixture();
    let list = find_list(&lists, "Empty");
    assert!(list.parse_error.is_none());
    assert_eq!(list.blocks.len(), 0);
    assert_eq!(evaluate_list(list, &snapshot).len(), 0);
}

#[test]
fn out_of_range_template_invalidates_list_silently() {
    let (_, lists) = load_fixture();
    let list = find_list(&lists, "Out Of Range");
    assert_eq!(
        list.parse_error.as_deref(),
        Some("template variable escapes lists.d boundary")
    );
    assert_eq!(list.blocks.len(), 0);
}

#[test]
fn missing_name_falls_back_to_filename_stem() {
    let (_, lists) = load_fixture();
    // No frontmatter `name`; fallback = filename stem "missing-name"
    let list = find_list(&lists, "missing-name");
    assert_eq!(list.blocks.len(), 1);
}

#[test]
fn unknown_filter_line_is_silently_skipped() {
    let (snapshot, lists) = load_fixture();
    let list = find_list(&lists, "With Unknown Filter");
    let result = evaluate_list(list, &snapshot);
    // foobar baz quux ignored; due <= today applies → 17 (Apr 1) and 21 (Apr 25)
    assert_eq!(result.len(), 2);
    assert_contains(&result, "Submit overdue paperwork");
    assert_contains(&result, "Submit weekly report");
}

#[test]
fn malformed_prefill_date_is_replaced_by_next_valid_one() {
    let (_, lists) = load_fixture();
    let list = find_list(&lists, "Bad Prefill Date");
    // First "prefill due next-week" discarded; second "prefill due today+3" wins
    assert_eq!(
        resolve_date_value(list.prefill.due.as_ref().unwrap(), TODAY),
        "2026-04-28"
    );
}

#[test]
fn unknown_prefill_field_is_silently_skipped() {
    let (_, lists) = load_fixture();
    let list = find_list(&lists, "Bad Prefill Field");
    // `prefill foo bar` ignored; `prefill project Work` applies
    assert_eq!(list.prefill.projects, vec!["Work"]);
}

#[test]
fn duplicate_scalar_prefill_uses_first_value() {
    let (_, lists) = load_fixture();
    let list = find_list(&lists, "Duplicate Scalar Prefill");
    // First "prefill due today + 1" wins
    assert_eq!(
        resolve_date_value(list.prefill.due.as_ref().unwrap(), TODAY),
        "2026-04-26"
    );
}

// ─── Spec coverage matrix ─────────────────────────────────────────────────────

/// Asserts that the fixture exercises at least one task/list per documented
/// spec rule. When a new rule is added to FORMAT.md / LISTS.md, add a
/// matching assertion here so a missing fixture surfaces as a test failure.
#[test]
fn fixture_covers_documented_parser_features() {
    let (snapshot, _) = load_fixture();
    let open: Vec<_> = snapshot.open_tasks.iter().map(|t| &t.task).collect();
    let done: Vec<_> = snapshot.done_tasks.iter().map(|t| &t.task).collect();

    // priority A and Z (boundary)
    assert!(open.iter().any(|t| t.priority == Some('A')));
    assert!(open.iter().any(|t| t.priority == Some('Z')));

    // creation date with and without priority
    assert!(
        open.iter()
            .any(|t| t.priority.is_some() && t.creation_date.is_some())
    );

    // every defined date tag
    for key in ["due", "scheduled", "starting", "updated"] {
        assert!(
            open.iter().any(|t| t.tags.contains_key(key)),
            "no open task carries {key} tag"
        );
    }

    // custom non-date tag preserved
    assert!(open.iter().any(|t| t.tags.contains_key("progress")));

    // multiple projects / contexts
    assert!(open.iter().any(|t| t.projects.len() >= 2));
    assert!(open.iter().any(|t| t.contexts.len() >= 2));

    // project at start of description
    assert!(
        open.iter()
            .any(|t| t.description.starts_with("+Errands Do laundry"))
    );

    // context at start of description
    assert!(open.iter().any(|t| t.description.starts_with("@phone")));

    // email-like @ does NOT become a context
    let email_task = open
        .iter()
        .find(|t| t.description.contains("john@example.com"))
        .expect("email task present");
    assert!(email_task.contexts.is_empty());

    // 2+2 math does NOT become a project
    let math_task = open
        .iter()
        .find(|t| t.description.contains("2+2"))
        .expect("math task present");
    assert_eq!(math_task.projects, vec!["Personal".to_string()]);

    // lowercase priority stays in description
    let lowercase = open
        .iter()
        .find(|t| t.description.contains("(b) Lowercase"))
        .expect("lowercase priority task");
    assert_eq!(lowercase.priority, None);

    // tag with two colons (time:09:00) is not parsed as a tag
    let time_task = open
        .iter()
        .find(|t| t.description.contains("time:09:00"))
        .expect("time-not-tag task");
    assert!(!time_task.tags.contains_key("time"));

    // duplicate due key — first wins
    let dup = open
        .iter()
        .find(|t| t.description.contains("Plan retro"))
        .expect("duplicate-due task");
    assert_eq!(dup.tags.get("due"), Some(&"2026-04-30".to_string()));

    // malformed first-occurrence due consumes the key (no due stored)
    let bad = open
        .iter()
        .find(|t| t.description.contains("Try thing"))
        .expect("malformed-due task");
    assert!(!bad.tags.contains_key("due"));

    // done tasks: minimal vs both-dates vs metadata-bearing
    assert!(done.iter().any(|t| t.creation_date.is_none()));
    assert!(done.iter().any(|t| t.creation_date.is_some()));
    assert!(done.iter().any(|t| !t.projects.is_empty()));
    assert!(done.iter().any(|t| t.tags.contains_key("updated")));
}

// ─── Live session: pressing 'a' on a prefill list seeds the editor ────────────

/// Reproduces the user-reported bug: pressing `a` while inside a smart list
/// with prefill should seed the editor with the prefill values, not leave it
/// empty. This drives the actual session like the live TUI binary does.
#[test]
fn pressing_a_on_work_inbox_seeds_editor_with_prefill() {
    let mut session =
        TuiSession::open_default(Path::new(FIXTURE_ROOT).to_path_buf(), TODAY).unwrap();
    let idx = session
        .smart_lists()
        .iter()
        .position(|l| l.name == "Work Inbox")
        .expect("Work Inbox list present");
    session.select_sidebar_item(SidebarItem::SmartList(idx));
    assert_eq!(session.active_sidebar_item(), SidebarItem::SmartList(idx));

    session.dispatch_key("a").unwrap();

    let editor = session
        .app()
        .editor
        .as_ref()
        .expect("editor opened after 'a'");
    assert!(
        !editor.raw_line.is_empty(),
        "editor should be seeded with prefill, but raw_line is empty"
    );
    assert!(
        editor.raw_line.contains("+Work"),
        "expected +Work in seeded editor, got {:?}",
        editor.raw_line
    );
    assert!(
        editor.raw_line.contains("@office"),
        "expected @office in seeded editor, got {:?}",
        editor.raw_line
    );
}

#[test]
fn pressing_a_on_this_week_seeds_priority_and_dates() {
    let mut session =
        TuiSession::open_default(Path::new(FIXTURE_ROOT).to_path_buf(), TODAY).unwrap();
    let idx = session
        .smart_lists()
        .iter()
        .position(|l| l.name == "This Week")
        .expect("This Week list present");
    session.select_sidebar_item(SidebarItem::SmartList(idx));

    session.dispatch_key("a").unwrap();

    let editor = session.app().editor.as_ref().expect("editor opened");
    let raw = &editor.raw_line;
    assert!(
        raw.starts_with("(A) "),
        "expected priority prefix, got {raw:?}"
    );
    assert!(
        raw.contains("due:2026-05-02"),
        "expected resolved due tag, got {raw:?}"
    );
    assert!(
        raw.contains("scheduled:2026-04-25"),
        "expected resolved scheduled tag, got {raw:?}"
    );
}

#[test]
fn rendered_editor_shows_prefill_content_after_pressing_a() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let mut session =
        TuiSession::open_default(Path::new(FIXTURE_ROOT).to_path_buf(), TODAY).unwrap();
    let idx = session
        .smart_lists()
        .iter()
        .position(|l| l.name == "Work Inbox")
        .unwrap();
    session.select_sidebar_item(SidebarItem::SmartList(idx));
    session.dispatch_key("a").unwrap();

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
        full.contains("+Work"),
        "rendered frame should show prefilled +Work; full screen:\n{full}"
    );
    assert!(
        full.contains("@office"),
        "rendered frame should show prefilled @office; full screen:\n{full}"
    );
}

#[test]
fn pressing_a_on_today_leaves_editor_empty() {
    // Today list has NO prefill — pressing 'a' should yield a blank editor.
    let mut session =
        TuiSession::open_default(Path::new(FIXTURE_ROOT).to_path_buf(), TODAY).unwrap();
    let idx = session
        .smart_lists()
        .iter()
        .position(|l| l.name == "Today")
        .expect("Today list present");
    session.select_sidebar_item(SidebarItem::SmartList(idx));

    session.dispatch_key("a").unwrap();
    let editor = session.app().editor.as_ref().expect("editor opened");
    assert!(
        editor.raw_line.is_empty(),
        "Today has no prefill; expected blank editor, got {:?}",
        editor.raw_line
    );
}

/// Assert at least one fixture list demonstrates every smart-list feature.
#[test]
fn fixture_covers_documented_smart_list_features() {
    let (_, lists) = load_fixture();

    // At least one list per Field used in filters/sort/group:
    // (this is a coarse check that specific fields appear somewhere)
    let mut sort_fields: Vec<&Field> = lists
        .iter()
        .flat_map(|l| l.sort_directives.iter().map(|d| &d.field))
        .collect();
    sort_fields.dedup();
    for required in [
        Field::Due,
        Field::CreationDate,
        Field::Priority,
        Field::Description,
    ] {
        assert!(
            sort_fields.iter().any(|f| **f == required),
            "no fixture list sorts by {required:?}"
        );
    }

    let group_fields: Vec<&Field> = lists
        .iter()
        .flat_map(|l| l.group_directives.iter().map(|d| &d.field))
        .collect();
    assert!(
        group_fields.iter().any(|f| **f == Field::Due),
        "no fixture list groups by due"
    );
    assert!(
        group_fields.iter().any(|f| **f == Field::Project),
        "no fixture list groups by project"
    );
    assert!(
        group_fields.iter().any(|f| **f == Field::Context),
        "no fixture list groups by context"
    );

    // Prefill — at least one list using each scalar field and each list field.
    let any_prefill = |pred: fn(&Prefill) -> bool| lists.iter().any(|l| pred(&l.prefill));
    assert!(any_prefill(|p| !p.projects.is_empty()), "prefill project");
    assert!(any_prefill(|p| !p.contexts.is_empty()), "prefill context");
    assert!(any_prefill(|p| p.priority.is_some()), "prefill priority");
    assert!(any_prefill(|p| p.due.is_some()), "prefill due");
    assert!(any_prefill(|p| p.scheduled.is_some()), "prefill scheduled");
    assert!(any_prefill(|p| p.starting.is_some()), "prefill starting");

    // Template variables resolved correctly in at least one grouped list
    let bugs = find_list(&lists, "Bugs");
    assert_eq!(bugs.prefill.projects, vec!["ttd"]); // {{dir}} → "ttd"

    // Invalid out-of-range template produces parse_error
    let bad = find_list(&lists, "Out Of Range");
    assert!(bad.parse_error.is_some());
}
