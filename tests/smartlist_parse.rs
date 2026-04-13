use std::path::Path;
use ttd::smartlist::{
    parse_list, CompareOp, Condition, DateField, Direction, Directive, Field,
    PriorityOp, TextOp, TextField,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn path(s: &str) -> &Path {
    Path::new(s)
}

fn lists_dir() -> &'static Path {
    Path::new("lists.d")
}

fn make_content(frontmatter: &str, body: &str) -> String {
    format!("---\n{frontmatter}---\n{body}")
}

// ---------------------------------------------------------------------------
// Frontmatter tests
// ---------------------------------------------------------------------------

#[test]
fn parses_frontmatter_with_all_fields() {
    let content = make_content("name: Today\nicon: 📅\ndescription: Tasks due today\n", "");
    let list = parse_list(&content, path("lists.d/today.list"), lists_dir());
    assert_eq!(list.name, "Today");
    assert_eq!(list.icon, Some("📅".to_string()));
    assert_eq!(list.description, Some("Tasks due today".to_string()));
    assert!(list.parse_error.is_none());
}

#[test]
fn missing_name_falls_back_to_filename_stem() {
    let content = make_content("icon: 🗂\n", "");
    let list = parse_list(&content, path("lists.d/inbox.list"), lists_dir());
    assert_eq!(list.name, "inbox");
    assert!(list.parse_error.is_none());
}

#[test]
fn unknown_frontmatter_keys_are_ignored() {
    let content = make_content("name: Test\nunknown_key: whatever\nanother: value\n", "");
    let list = parse_list(&content, path("lists.d/test.list"), lists_dir());
    assert_eq!(list.name, "Test");
    assert!(list.parse_error.is_none());
}

#[test]
fn missing_frontmatter_delimiters_set_parse_error() {
    let content = "name: Test\ndue <= today\n";
    let list = parse_list(content, path("lists.d/test.list"), lists_dir());
    assert!(list.parse_error.is_some());
}

#[test]
fn empty_body_produces_no_filter_blocks() {
    let content = make_content("name: Empty\n", "");
    let list = parse_list(&content, path("lists.d/empty.list"), lists_dir());
    assert!(list.blocks.is_empty());
    assert!(list.sort_directives.is_empty());
    assert!(list.group_directives.is_empty());
}

#[test]
fn parses_description_frontmatter() {
    let content = make_content("name: Test\ndescription: A helpful description\n", "");
    let list = parse_list(&content, path("lists.d/test.list"), lists_dir());
    assert_eq!(list.description, Some("A helpful description".to_string()));
}

#[test]
fn description_is_none_when_omitted() {
    let content = make_content("name: Test\n", "");
    let list = parse_list(&content, path("lists.d/test.list"), lists_dir());
    assert_eq!(list.description, None);
}

#[test]
fn legacy_order_key_is_ignored_without_error() {
    let content = make_content("name: Test\norder: 5\n", "");
    let list = parse_list(&content, path("lists.d/test.list"), lists_dir());
    assert_eq!(list.name, "Test");
    assert!(list.parse_error.is_none());
}

// ---------------------------------------------------------------------------
// Filter body tests
// ---------------------------------------------------------------------------

#[test]
fn parses_date_comparison_with_today() {
    let content = make_content("name: Today\n", "due <= today\n");
    let list = parse_list(&content, path("lists.d/today.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::DateComparison {
            field: DateField::Due,
            op: CompareOp::Lte,
            offset: 0,
        }
    );
}

#[test]
fn parses_date_comparison_with_offset() {
    let content = make_content("name: Upcoming\n", "due <= today + 7\n");
    let list = parse_list(&content, path("lists.d/upcoming.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::DateComparison {
            field: DateField::Due,
            op: CompareOp::Lte,
            offset: 7,
        }
    );
}

#[test]
fn parses_date_comparison_with_negative_offset() {
    let content = make_content("name: Overdue\n", "due < today - 14\n");
    let list = parse_list(&content, path("lists.d/overdue.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::DateComparison {
            field: DateField::Due,
            op: CompareOp::Lt,
            offset: -14,
        }
    );
}

#[test]
fn parses_date_offset_without_spaces() {
    let content = make_content("name: Soon\n", "due <= today+7\n");
    let list = parse_list(&content, path("lists.d/soon.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::DateComparison {
            field: DateField::Due,
            op: CompareOp::Lte,
            offset: 7,
        }
    );
}

#[test]
fn parses_priority_comparison() {
    let content = make_content("name: High Priority\n", "priority above C\n");
    let list = parse_list(&content, path("lists.d/highpri.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::PriorityComparison {
            op: PriorityOp::Above,
            letter: 'C',
        }
    );
}

#[test]
fn parses_text_match() {
    let content = make_content("name: Work\n", "project includes Work\n");
    let list = parse_list(&content, path("lists.d/work.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::TextMatch {
            field: TextField::Project,
            op: TextOp::Includes,
            text: "Work".to_string(),
        }
    );
}

#[test]
fn parses_existence_conditions() {
    let content = make_content("name: No Dates\n", "no due\nno scheduled\nno starting\n");
    let list = parse_list(&content, path("lists.d/nodates.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    let conds = &list.blocks[0].conditions;
    assert_eq!(conds.len(), 3);
    assert_eq!(
        conds[0],
        Condition::Existence {
            field: Field::Due,
            present: false,
        }
    );
    assert_eq!(
        conds[1],
        Condition::Existence {
            field: Field::Scheduled,
            present: false,
        }
    );
    assert_eq!(
        conds[2],
        Condition::Existence {
            field: Field::Starting,
            present: false,
        }
    );
}

#[test]
fn parses_done_and_not_done() {
    let done_content = make_content("name: Done\n", "done\n");
    let done_list = parse_list(&done_content, path("lists.d/done.list"), lists_dir());
    assert_eq!(done_list.blocks.len(), 1);
    assert_eq!(
        done_list.blocks[0].conditions[0],
        Condition::DoneFilter { done: true }
    );

    let not_done_content = make_content("name: Active\n", "not done\n");
    let not_done_list = parse_list(&not_done_content, path("lists.d/active.list"), lists_dir());
    assert_eq!(not_done_list.blocks.len(), 1);
    assert_eq!(
        not_done_list.blocks[0].conditions[0],
        Condition::DoneFilter { done: false }
    );
}

#[test]
fn parses_or_blocks() {
    let body = "due <= today\nOR\nscheduled <= today\n";
    let content = make_content("name: Actionable\n", body);
    let list = parse_list(&content, path("lists.d/actionable.list"), lists_dir());
    assert_eq!(list.blocks.len(), 2);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::DateComparison {
            field: DateField::Due,
            op: CompareOp::Lte,
            offset: 0,
        }
    );
    assert_eq!(
        list.blocks[1].conditions[0],
        Condition::DateComparison {
            field: DateField::Scheduled,
            op: CompareOp::Lte,
            offset: 0,
        }
    );
}

#[test]
fn parses_sort_and_group_directives() {
    let body = "due <= today\nsort by due asc\ngroup by priority desc\n";
    let content = make_content("name: Sorted\n", body);
    let list = parse_list(&content, path("lists.d/sorted.list"), lists_dir());
    assert_eq!(list.sort_directives.len(), 1);
    assert_eq!(
        list.sort_directives[0],
        Directive {
            field: Field::Due,
            direction: Direction::Asc,
        }
    );
    assert_eq!(list.group_directives.len(), 1);
    assert_eq!(
        list.group_directives[0],
        Directive {
            field: Field::Priority,
            direction: Direction::Desc,
        }
    );
}

#[test]
fn unrecognized_filter_lines_are_silently_skipped() {
    let body = "due <= today\nthis is nonsense\nfoo bar baz\nscheduled <= today\n";
    let content = make_content("name: Test\n", body);
    let list = parse_list(&content, path("lists.d/test.list"), lists_dir());
    // Only valid conditions are kept; no panic or error
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(list.blocks[0].conditions.len(), 2);
    assert!(list.parse_error.is_none());
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let body = "# This is a comment\n\ndue <= today\n\n# Another comment\nscheduled <= today\n";
    let content = make_content("name: Test\n", body);
    let list = parse_list(&content, path("lists.d/test.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(list.blocks[0].conditions.len(), 2);
}

#[test]
fn has_existence_form_parses_correctly() {
    let content = make_content("name: Has Due\n", "has due\n");
    let list = parse_list(&content, path("lists.d/hasdue.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::Existence {
            field: Field::Due,
            present: true,
        }
    );
}

#[test]
fn text_excludes_operator_parses_correctly() {
    let content = make_content("name: Not Work\n", "project excludes Work\n");
    let list = parse_list(&content, path("lists.d/notwork.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::TextMatch {
            field: TextField::Project,
            op: TextOp::Excludes,
            text: "Work".to_string(),
        }
    );
}

#[test]
fn priority_eq_and_below_parse_correctly() {
    let eq_content = make_content("name: Priority A\n", "priority = A\n");
    let eq_list = parse_list(&eq_content, path("lists.d/pria.list"), lists_dir());
    assert_eq!(eq_list.blocks.len(), 1);
    assert_eq!(
        eq_list.blocks[0].conditions[0],
        Condition::PriorityComparison {
            op: PriorityOp::Eq,
            letter: 'A',
        }
    );

    let below_content = make_content("name: Below C\n", "priority below C\n");
    let below_list = parse_list(&below_content, path("lists.d/belowc.list"), lists_dir());
    assert_eq!(below_list.blocks.len(), 1);
    assert_eq!(
        below_list.blocks[0].conditions[0],
        Condition::PriorityComparison {
            op: PriorityOp::Below,
            letter: 'C',
        }
    );
}

#[test]
fn today_negative_offset_without_spaces() {
    let content = make_content("name: Recent\n", "due <= today-7\n");
    let list = parse_list(&content, path("lists.d/recent.list"), lists_dir());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::DateComparison {
            field: DateField::Due,
            op: CompareOp::Lte,
            offset: -7,
        }
    );
}

#[test]
fn crlf_line_endings_are_handled() {
    let content = "---\r\nname: CRLF Test\r\n---\r\ndue <= today\r\nscheduled <= today\r\n";
    let list = parse_list(content, path("lists.d/crlf.list"), lists_dir());
    assert_eq!(list.name, "CRLF Test");
    assert!(list.parse_error.is_none());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(list.blocks[0].conditions.len(), 2);
}

#[test]
fn sort_directive_defaults_to_asc() {
    let content = make_content("name: Sorted\n", "due <= today\nsort by due\n");
    let list = parse_list(&content, path("lists.d/sorted.list"), lists_dir());
    assert_eq!(list.sort_directives.len(), 1);
    assert_eq!(
        list.sort_directives[0],
        Directive {
            field: Field::Due,
            direction: Direction::Asc,
        }
    );
}

// ---------------------------------------------------------------------------
// Template variable tests
// ---------------------------------------------------------------------------

#[test]
fn resolves_dir_template_variable() {
    let content = make_content("name: Bugs\n", "project includes {{dir}}\n");
    let list = parse_list(&content, path("lists.d/ttd/bugs.list"), lists_dir());
    assert!(list.parse_error.is_none());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::TextMatch {
            field: TextField::Project,
            op: TextOp::Includes,
            text: "ttd".to_string(),
        }
    );
}

#[test]
fn resolves_dir_n_template_variable() {
    let content = make_content("name: Deep\n", "project includes {{dir:1}}\n");
    let list = parse_list(&content, path("lists.d/work/ttd/deep.list"), lists_dir());
    assert!(list.parse_error.is_none());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::TextMatch {
            field: TextField::Project,
            op: TextOp::Includes,
            text: "work".to_string(),
        }
    );
}

#[test]
fn dir_0_is_same_as_dir() {
    let content_dir = make_content("name: A\n", "project includes {{dir}}\n");
    let content_dir0 = make_content("name: B\n", "project includes {{dir:0}}\n");
    let list_dir = parse_list(&content_dir, path("lists.d/sub/a.list"), lists_dir());
    let list_dir0 = parse_list(&content_dir0, path("lists.d/sub/b.list"), lists_dir());
    assert!(list_dir.parse_error.is_none());
    assert!(list_dir0.parse_error.is_none());
    assert_eq!(list_dir.blocks[0].conditions[0], list_dir0.blocks[0].conditions[0]);
}

#[test]
fn dir_escaping_boundary_sets_parse_error() {
    let content = make_content("name: Bad\n", "project includes {{dir:2}}\n");
    let list = parse_list(&content, path("lists.d/invalid/bad.list"), lists_dir());
    assert!(list.parse_error.is_some());
}

#[test]
fn dir_on_root_level_file_sets_parse_error() {
    let content = make_content("name: Root\n", "project includes {{dir}}\n");
    let list = parse_list(&content, path("lists.d/root.list"), lists_dir());
    assert!(list.parse_error.is_some());
}

#[test]
fn no_template_variables_works_normally() {
    let content = make_content("name: Normal\n", "project includes Work\n");
    let list = parse_list(&content, path("lists.d/sub/normal.list"), lists_dir());
    assert!(list.parse_error.is_none());
    assert_eq!(list.blocks.len(), 1);
    assert_eq!(
        list.blocks[0].conditions[0],
        Condition::TextMatch {
            field: TextField::Project,
            op: TextOp::Includes,
            text: "Work".to_string(),
        }
    );
}
