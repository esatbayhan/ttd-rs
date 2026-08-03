use ttd::parser::{format_task, parse_task_line};

#[test]
fn parses_valid_due_fixture() {
    let raw = "Call Mom due:2024-04-15";
    let task = parse_task_line(raw);

    assert!(!task.done);
    assert_eq!(task.tags.get("due").map(String::as_str), Some("2024-04-15"));
}

#[test]
fn parses_priority_only_fixture() {
    let raw = "(A) Call Mom";
    let task = parse_task_line(raw);

    assert_eq!(task.priority, Some('A'));
    assert_eq!(task.creation_date, None);
    assert_eq!(task.description, "Call Mom");
}

#[test]
fn parses_creation_date_without_priority_fixture() {
    let raw = "2024-01-15 Call Mom";
    let task = parse_task_line(raw);

    assert_eq!(task.priority, None);
    assert_eq!(task.creation_date.as_deref(), Some("2024-01-15"));
    assert_eq!(task.description, "Call Mom");
}

#[test]
fn malformed_first_duplicate_key_blocks_second_key() {
    let raw = "Call Mom due:not-a-date due:2024-01-01";
    let task = parse_task_line(raw);

    assert_eq!(task.tags.get("due"), None);
    assert!(task.description.contains("due:2024-01-01"));
}

#[test]
fn malformed_first_due_blocks_valid_second_fixture() {
    let raw = "Call Mom due:next-week due:2024-05-01";
    let task = parse_task_line(raw);

    assert_eq!(task.tags.get("due"), None);
    assert!(task.description.contains("due:next-week"));
    assert!(task.description.contains("due:2024-05-01"));
}

#[test]
fn invalid_completion_marker_stays_in_description() {
    let raw = "x Call Mom";
    let task = parse_task_line(raw);

    assert!(!task.done);
    assert!(task.description.starts_with("x "));
}

#[test]
fn uppercase_completion_marker_stays_in_description() {
    let raw = "X 2024-01-01 Call Mom";
    let task = parse_task_line(raw);

    assert!(!task.done);
    assert_eq!(task.completion_date, None);
    assert_eq!(task.description, raw);
}

#[test]
fn extracts_metadata_after_tab_boundary() {
    let task = parse_task_line("Call Mom\t+Family\t@phone\tdue:2024-04-15");

    assert_eq!(
        task.description,
        "Call Mom\t+Family\t@phone\tdue:2024-04-15"
    );
    assert_eq!(task.projects, vec!["Family"]);
    assert_eq!(task.contexts, vec!["phone"]);
    assert_eq!(task.tags.get("due").map(String::as_str), Some("2024-04-15"));
}

#[test]
fn parses_completed_basic_fixture() {
    let raw = "x 2024-03-01 Call Mom";
    let task = parse_task_line(raw);

    assert!(task.done);
    assert_eq!(task.completion_date.as_deref(), Some("2024-03-01"));
    assert_eq!(task.creation_date, None);
    assert_eq!(task.description, "Call Mom");
}

#[test]
fn parses_completed_task_fixture_with_both_dates() {
    let raw = "x 2024-01-15 2024-01-10 Something +TodoTxtTouch @github";
    let task = parse_task_line(raw);

    assert!(task.done);
    assert_eq!(task.completion_date.as_deref(), Some("2024-01-15"));
    assert_eq!(task.creation_date.as_deref(), Some("2024-01-10"));
    assert_eq!(task.projects, vec!["TodoTxtTouch"]);
    assert_eq!(task.contexts, vec!["github"]);
}

#[test]
fn parses_completed_with_metadata_fixture() {
    let raw = "x 2024-02-15 2024-01-01 Something +Shopping @errands due:2024-02-15";
    let task = parse_task_line(raw);

    assert!(task.done);
    assert_eq!(task.completion_date.as_deref(), Some("2024-02-15"));
    assert_eq!(task.creation_date.as_deref(), Some("2024-01-01"));
    assert_eq!(task.projects, vec!["Shopping"]);
    assert_eq!(task.contexts, vec!["errands"]);
    assert_eq!(task.tags.get("due").map(String::as_str), Some("2024-02-15"));
}

#[test]
fn parses_scheduled_fixture() {
    let raw = "2024-03-01 Something scheduled:2024-03-20 due:2024-03-31";
    let task = parse_task_line(raw);

    assert_eq!(task.creation_date.as_deref(), Some("2024-03-01"));
    assert_eq!(
        task.tags.get("scheduled").map(String::as_str),
        Some("2024-03-20")
    );
    assert_eq!(task.tags.get("due").map(String::as_str), Some("2024-03-31"));
}

#[test]
fn parses_updated_fixture() {
    let raw = "(B) Something updated:2024-03-20 due:2024-03-31";
    let task = parse_task_line(raw);

    assert_eq!(task.priority, Some('B'));
    assert_eq!(
        task.tags.get("updated").map(String::as_str),
        Some("2024-03-20")
    );
    assert_eq!(task.tags.get("due").map(String::as_str), Some("2024-03-31"));
}

#[test]
fn rejects_non_date_updated_value() {
    let task = parse_task_line("Review goals updated:next-week");
    assert_eq!(task.tags.get("updated"), None);
    assert!(task.description.contains("updated:next-week"));
}

#[test]
fn parses_starting_fixture() {
    let raw = "Something starting:2024-06-01 due:2024-06-10";
    let task = parse_task_line(raw);

    assert_eq!(
        task.tags.get("starting").map(String::as_str),
        Some("2024-06-01")
    );
    assert_eq!(task.tags.get("due").map(String::as_str), Some("2024-06-10"));
}

#[test]
fn parses_multiple_projects_and_contexts_fixture() {
    let raw = "(A) Something +Family +PeaceLoveAndHappiness @iphone @phone";
    let task = parse_task_line(raw);

    assert_eq!(task.priority, Some('A'));
    assert_eq!(task.projects, vec!["Family", "PeaceLoveAndHappiness"]);
    assert_eq!(task.contexts, vec!["iphone", "phone"]);
}

#[test]
fn parses_multiple_tags_fixture() {
    let raw = "(A) 2024-01-01 Something due:2024-06-01 scheduled:2024-05-15 starting:2024-03-01";
    let task = parse_task_line(raw);

    assert_eq!(task.priority, Some('A'));
    assert_eq!(task.creation_date.as_deref(), Some("2024-01-01"));
    assert_eq!(task.tags.get("due").map(String::as_str), Some("2024-06-01"));
    assert_eq!(
        task.tags.get("scheduled").map(String::as_str),
        Some("2024-05-15")
    );
    assert_eq!(
        task.tags.get("starting").map(String::as_str),
        Some("2024-03-01")
    );
}

#[test]
fn valid_duplicate_key_fixture_keeps_first_value() {
    let raw = "Something due:2024-04-15 due:2024-05-01";
    let task = parse_task_line(raw);

    assert_eq!(task.tags.get("due").map(String::as_str), Some("2024-04-15"));
    assert!(task.description.contains("due:2024-05-01"));
}

#[test]
fn plus_sign_inside_token_is_not_a_project() {
    let raw = "2+2=4";
    let task = parse_task_line(raw);

    assert!(task.projects.is_empty());
    assert_eq!(task.description, raw);
}

#[test]
fn at_sign_inside_token_is_not_a_context() {
    let raw = "user@example.com";
    let task = parse_task_line(raw);

    assert!(task.contexts.is_empty());
    assert_eq!(task.description, raw);
}

#[test]
fn multi_colon_token_is_now_a_tag_v3() {
    let raw = "12:30:45";
    let task = parse_task_line(raw);

    assert_eq!(task.tags.get("12"), Some(&"30:45".to_string()));
    assert_eq!(task.description, raw);
}

#[test]
fn parses_project_at_start_of_description_fixture() {
    let raw = "+GarageSale something";
    let task = parse_task_line(raw);

    assert_eq!(task.projects, vec!["GarageSale"]);
    assert_eq!(task.description, raw);
}

#[test]
fn parses_context_at_start_of_description_fixture() {
    let raw = "@phone something";
    let task = parse_task_line(raw);

    assert_eq!(task.contexts, vec!["phone"]);
    assert_eq!(task.description, raw);
}

#[test]
fn invalid_due_partial_date_stays_in_description() {
    let raw = "something due:2024-01";
    let task = parse_task_line(raw);

    assert_eq!(task.tags.get("due"), None);
    assert_eq!(task.description, raw);
}

#[test]
fn invalid_due_non_date_stays_in_description() {
    let raw = "something due:tomorrow";
    let task = parse_task_line(raw);

    assert_eq!(task.tags.get("due"), None);
    assert_eq!(task.description, raw);
}

#[test]
fn malformed_unicode_date_stays_in_description_without_panicking() {
    let raw = "Task due:€€€€";
    let task = parse_task_line(raw);
    assert_eq!(task.tags.get("due"), None);
    assert_eq!(task.description, raw);
}

#[test]
fn out_of_range_time_stays_in_description() {
    let raw = "Task due:2026-04-03T24:00";
    let task = parse_task_line(raw);
    assert_eq!(task.tags.get("due"), None);
    assert_eq!(task.description, raw);
}

#[test]
fn invalid_priority_fixture_stays_in_description() {
    let raw = "(a) Call Mom";
    let task = parse_task_line(raw);

    assert_eq!(task.priority, None);
    assert_eq!(task.description, raw);
}

#[test]
fn wrong_format_creation_date_stays_in_description() {
    let raw = "(A) 01-15-2024 Call Mom";
    let task = parse_task_line(raw);

    assert_eq!(task.priority, Some('A'));
    assert_eq!(task.creation_date, None);
    assert_eq!(task.description, "01-15-2024 Call Mom");
}

#[test]
fn formatter_round_trips_normalized_open_task() {
    let task = parse_task_line("(A) 2024-01-15 Call Mom +Family due:2024-04-15");

    assert_eq!(
        format_task(&task),
        "(A) 2024-01-15 Call Mom +Family due:2024-04-15"
    );
}
