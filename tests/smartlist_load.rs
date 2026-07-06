use std::fs;
use std::path::PathBuf;
use ttd::smartlist::load_all;

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("ttd-smartlist-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    path
}

#[test]
fn loads_and_sorts_by_filename_alphabetically() {
    let dir = temp_path("sort_alpha");
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("3 Upcoming.list"), "---\nname: Upcoming\n---\n").unwrap();
    fs::write(dir.join("1 Today.list"), "---\nname: Today\n---\n").unwrap();
    fs::write(dir.join("2 Inbox.list"), "---\nname: Inbox\n---\n").unwrap();

    let lists = load_all(&dir);
    fs::remove_dir_all(&dir).unwrap();

    assert_eq!(lists.len(), 3);
    assert_eq!(lists[0].name, "Today");
    assert_eq!(lists[1].name, "Inbox");
    assert_eq!(lists[2].name, "Upcoming");
}

#[test]
fn legacy_order_key_is_ignored_in_sorting() {
    let dir = temp_path("legacy_order");
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("alpha.list"), "---\nname: Alpha\norder: 99\n---\n").unwrap();
    fs::write(dir.join("beta.list"), "---\nname: Beta\norder: 1\n---\n").unwrap();

    let lists = load_all(&dir);
    fs::remove_dir_all(&dir).unwrap();

    assert_eq!(lists.len(), 2);
    // Sorted alphabetically by full path, not by order
    assert_eq!(lists[0].name, "Alpha");
    assert_eq!(lists[1].name, "Beta");
}

#[test]
fn returns_empty_vec_when_lists_dir_does_not_exist() {
    let dir = temp_path("nonexistent");
    // Ensure the directory doesn't exist
    let _ = fs::remove_dir_all(&dir);

    let lists = load_all(&dir);

    assert!(lists.is_empty());
}

#[test]
fn skips_non_list_files() {
    let dir = temp_path("skip_non_list");
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("valid.list"), "---\nname: Valid\n---\n").unwrap();
    fs::write(dir.join("readme.txt"), "this is a text file").unwrap();
    fs::write(dir.join("notes.md"), "# markdown notes").unwrap();

    let lists = load_all(&dir);
    fs::remove_dir_all(&dir).unwrap();

    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].name, "Valid");
}

#[test]
fn malformed_file_included_with_parse_error() {
    let dir = temp_path("malformed");
    fs::create_dir_all(&dir).unwrap();

    // File without frontmatter delimiters
    fs::write(dir.join("broken.list"), "not done\ndue < today\n").unwrap();

    let lists = load_all(&dir);
    fs::remove_dir_all(&dir).unwrap();

    assert_eq!(lists.len(), 1);
    assert!(lists[0].parse_error.is_some());
}

#[test]
fn loads_lists_from_subdirectories_recursively() {
    let dir = temp_path("recursive");
    fs::create_dir_all(dir.join("work")).unwrap();

    fs::write(
        dir.join("today.list"),
        "---\nname: Today\n---\ndue <= today\n",
    )
    .unwrap();
    fs::write(
        dir.join("work/tasks.list"),
        "---\nname: Work Tasks\n---\nproject includes Work\n",
    )
    .unwrap();

    let lists = load_all(&dir);
    fs::remove_dir_all(&dir).unwrap();

    assert_eq!(lists.len(), 2);
    // Both root and subdirectory files found
    let names: Vec<&str> = lists.iter().map(|l| l.name.as_str()).collect();
    assert!(names.contains(&"Today"));
    assert!(names.contains(&"Work Tasks"));

    // The grouped list has correct group_path
    let work_list = lists.iter().find(|l| l.name == "Work Tasks").unwrap();
    assert_eq!(work_list.group_path, vec!["work".to_string()]);

    // Root-level list has empty group_path
    let today_list = lists.iter().find(|l| l.name == "Today").unwrap();
    assert!(today_list.group_path.is_empty());
}

#[test]
fn template_variable_out_of_range_still_loads_with_error() {
    let dir = temp_path("template_error");
    fs::create_dir_all(dir.join("sub")).unwrap();

    fs::write(
        dir.join("sub/bad.list"),
        "---\nname: Bad\n---\nproject includes {{dir:5}}\n",
    )
    .unwrap();

    let lists = load_all(&dir);
    fs::remove_dir_all(&dir).unwrap();

    assert_eq!(lists.len(), 1);
    assert!(lists[0].parse_error.is_some());
}

#[test]
fn nested_groups_have_correct_group_path() {
    let dir = temp_path("nested_groups");
    fs::create_dir_all(dir.join("work/client-a")).unwrap();

    fs::write(
        dir.join("work/client-a/review.list"),
        "---\nname: Review\n---\nproject includes review\n",
    )
    .unwrap();

    let lists = load_all(&dir);
    fs::remove_dir_all(&dir).unwrap();

    assert_eq!(lists.len(), 1);
    assert_eq!(
        lists[0].group_path,
        vec!["work".to_string(), "client-a".to_string()]
    );
}
