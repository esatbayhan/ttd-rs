use super::types::*;
use crate::store::StoredTask;
use crate::task::Task;

// -- Date arithmetic ----------------------------------------------------------

fn julian_day(year: i32, month: u32, day: u32) -> i32 {
    let a = (14 - month as i32) / 12;
    let y = year + 4800 - a;
    let m = month as i32 + 12 * a - 3;
    day as i32 + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045
}

fn from_julian_day(jdn: i32) -> (i32, u32, u32) {
    let a = jdn + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - (146097 * b) / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - (1461 * d) / 4;
    let m = (5 * e + 2) / 153;
    let day = (e - (153 * m + 2) / 5 + 1) as u32;
    let month = (m + 3 - 12 * (m / 10)) as u32;
    let year = 100 * b + d - 4800 + m / 10;
    (year, month, day)
}

pub fn add_days_to_date(date_str: &str, days: i32) -> String {
    let year: i32 = date_str[0..4].parse().unwrap_or(0);
    let month: u32 = date_str[5..7].parse().unwrap_or(1);
    let day: u32 = date_str[8..10].parse().unwrap_or(1);
    let jdn = julian_day(year, month, day) + days;
    let (y, mo, d) = from_julian_day(jdn);
    format!("{:04}-{:02}-{:02}", y, mo, d)
}

// -- Condition evaluation -----------------------------------------------------

fn get_date_field_value<'a>(task: &'a Task, field: &DateField) -> Option<&'a str> {
    match field {
        DateField::Due => task.tags.get("due").map(|s| s.as_str()),
        DateField::Scheduled => task.tags.get("scheduled").map(|s| s.as_str()),
        DateField::Starting => task.tags.get("starting").map(|s| s.as_str()),
        DateField::CreationDate => task.creation_date.as_deref(),
    }
}

fn eval_condition(cond: &Condition, task: &Task, today: &str) -> bool {
    match cond {
        Condition::DoneFilter { done } => task.done == *done,

        Condition::Existence { field, present } => {
            let has = match field {
                Field::Due => task.tags.contains_key("due"),
                Field::Scheduled => task.tags.contains_key("scheduled"),
                Field::Starting => task.tags.contains_key("starting"),
                Field::CreationDate => task.creation_date.is_some(),
                Field::Priority => task.priority.is_some(),
                Field::Project => !task.projects.is_empty(),
                Field::Context => !task.contexts.is_empty(),
                Field::Description => !task.description.is_empty(),
                Field::Done => task.done,
            };
            has == *present
        }

        Condition::DateComparison { field, op, offset } => {
            let task_date = match get_date_field_value(task, field) {
                Some(d) => d,
                None => return false,
            };
            let target = add_days_to_date(today, *offset);
            match op {
                CompareOp::Eq => task_date == target,
                CompareOp::Lt => task_date < target.as_str(),
                CompareOp::Lte => task_date <= target.as_str(),
                CompareOp::Gt => task_date > target.as_str(),
                CompareOp::Gte => task_date >= target.as_str(),
            }
        }

        Condition::PriorityComparison { op, letter } => {
            let task_priority = match task.priority {
                Some(p) => p,
                None => return false,
            };
            match op {
                PriorityOp::Eq => task_priority == *letter,
                // "above" means alphabetically earlier (A is above B)
                PriorityOp::Above => task_priority < *letter,
                // "below" means alphabetically later
                PriorityOp::Below => task_priority > *letter,
            }
        }

        Condition::TextMatch { field, op, text } => {
            let needle = text.to_lowercase();
            let matches = match field {
                TextField::Project => task
                    .projects
                    .iter()
                    .any(|p| p.to_lowercase().contains(&needle)),
                TextField::Context => task
                    .contexts
                    .iter()
                    .any(|c| c.to_lowercase().contains(&needle)),
                TextField::Description => task.description.to_lowercase().contains(&needle),
            };
            match op {
                TextOp::Includes => matches,
                TextOp::Excludes => !matches,
            }
        }
    }
}

pub fn has_done_filter(list: &SmartList) -> bool {
    list.blocks.iter().any(|block| {
        block
            .conditions
            .iter()
            .any(|c| matches!(c, Condition::DoneFilter { .. }))
    })
}

// -- Sort helpers -------------------------------------------------------------

fn task_sort_key(task: &Task, field: &Field) -> Option<String> {
    match field {
        Field::Due => task.tags.get("due").cloned(),
        Field::Scheduled => task.tags.get("scheduled").cloned(),
        Field::Starting => task.tags.get("starting").cloned(),
        Field::CreationDate => task.creation_date.clone(),
        Field::Priority => task.priority.map(|c| c.to_string()),
        Field::Project => task.projects.first().cloned(),
        Field::Context => task.contexts.first().cloned(),
        Field::Description => Some(task.description.clone()),
        Field::Done => Some(if task.done { "1" } else { "0" }.to_string()),
    }
}

// -- Public API ---------------------------------------------------------------

pub fn evaluate(list: &SmartList, tasks: &[StoredTask], today: &str) -> Vec<StoredTask> {
    if list.blocks.is_empty() {
        return Vec::new();
    }

    let implied_not_done = !has_done_filter(list);

    let mut matched: Vec<StoredTask> = tasks
        .iter()
        .filter(|st| {
            // Apply implied "not done" pre-filter
            if implied_not_done && st.task.done {
                return false;
            }
            // DNF: match if ANY block has ALL conditions true
            list.blocks.iter().any(|block| {
                block
                    .conditions
                    .iter()
                    .all(|cond| eval_condition(cond, &st.task, today))
            })
        })
        .cloned()
        .collect();

    // Apply sort directives (first directive = highest precedence)
    matched.sort_by(|a, b| {
        for directive in &list.sort_directives {
            let ka = task_sort_key(&a.task, &directive.field);
            let kb = task_sort_key(&b.task, &directive.field);

            // Items with values sort before items without
            let ord = match (&ka, &kb) {
                (Some(va), Some(vb)) => va.cmp(vb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            };

            let ord = if directive.direction == Direction::Desc {
                ord.reverse()
            } else {
                ord
            };

            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });

    matched
}

pub fn filter_only(list: &SmartList, tasks: &[StoredTask], today: &str) -> Vec<StoredTask> {
    if list.blocks.is_empty() {
        return Vec::new();
    }

    let implied_not_done = !has_done_filter(list);

    tasks
        .iter()
        .filter(|st| {
            if implied_not_done && st.task.done {
                return false;
            }
            list.blocks.iter().any(|block| {
                block
                    .conditions
                    .iter()
                    .all(|cond| eval_condition(cond, &st.task, today))
            })
        })
        .cloned()
        .collect()
}

pub fn sort_by_directives(tasks: &mut [StoredTask], directives: &[Directive]) {
    tasks.sort_by(|a, b| {
        for directive in directives {
            let ka = task_sort_key(&a.task, &directive.field);
            let kb = task_sort_key(&b.task, &directive.field);

            let ord = match (&ka, &kb) {
                (Some(va), Some(vb)) => va.cmp(vb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            };

            let ord = if directive.direction == Direction::Desc {
                ord.reverse()
            } else {
                ord
            };

            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });
}

pub fn group_by_directives(directives: &[Directive], tasks: &[StoredTask]) -> Vec<TaskGroup> {
    let directive = match directives.first() {
        Some(d) => d,
        None => {
            return vec![TaskGroup {
                label: String::new(),
                tasks: tasks.to_vec(),
            }];
        }
    };

    let mut labeled: Vec<(String, StoredTask)> = Vec::new();
    let mut no_value: Vec<StoredTask> = Vec::new();
    let fallback_label = format!("No {}", field_display_name(&directive.field));

    for st in tasks {
        match task_group_key(&st.task, &directive.field) {
            Some(key) => labeled.push((key, st.clone())),
            None => no_value.push(st.clone()),
        }
    }

    let mut group_map: std::collections::BTreeMap<String, Vec<StoredTask>> =
        std::collections::BTreeMap::new();
    for (key, st) in labeled {
        group_map.entry(key).or_default().push(st);
    }

    let field_prefix = capitalize(field_display_name(&directive.field));
    let mut groups: Vec<TaskGroup> = group_map
        .into_iter()
        .map(|(value, tasks)| TaskGroup {
            label: format!("{field_prefix}: {value}"),
            tasks,
        })
        .collect();

    groups.sort_by(|a, b| match directive.direction {
        Direction::Asc => a.label.cmp(&b.label),
        Direction::Desc => b.label.cmp(&a.label),
    });

    if !no_value.is_empty() {
        groups.push(TaskGroup {
            label: fallback_label,
            tasks: no_value,
        });
    }

    groups
}

// -- Grouping -----------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TaskGroup {
    pub label: String,
    pub tasks: Vec<StoredTask>,
}

fn task_group_key(task: &Task, field: &Field) -> Option<String> {
    match field {
        Field::Due => task.tags.get("due").cloned(),
        Field::Scheduled => task.tags.get("scheduled").cloned(),
        Field::Starting => task.tags.get("starting").cloned(),
        Field::CreationDate => task.creation_date.clone(),
        Field::Priority => task.priority.map(|c| c.to_string()),
        Field::Project => task.projects.first().cloned(),
        Field::Context => task.contexts.first().cloned(),
        Field::Description => Some(task.description.clone()),
        Field::Done => Some(if task.done { "Done" } else { "Not done" }.to_string()),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

pub fn field_display_name(field: &Field) -> &'static str {
    match field {
        Field::Due => "due",
        Field::Scheduled => "scheduled",
        Field::Starting => "starting",
        Field::CreationDate => "creation date",
        Field::Priority => "priority",
        Field::Project => "project",
        Field::Context => "context",
        Field::Description => "description",
        Field::Done => "done",
    }
}

pub fn group(list: &SmartList, tasks: &[StoredTask]) -> Vec<TaskGroup> {
    group_by_directives(&list.group_directives, tasks)
}
