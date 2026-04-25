use std::path::Path;

use super::types::*;

fn is_date_literal(value: &str) -> bool {
    value.len() == 10
        && value
            .as_bytes()
            .iter()
            .enumerate()
            .all(|(idx, b)| match idx {
                4 | 7 => *b == b'-',
                _ => b.is_ascii_digit(),
            })
}

/// Parse a `date-value` per LISTS.md grammar.
///
/// Accepts:
/// - `today`
/// - `today + N` / `today - N` (spaces optional)
/// - `YYYY-MM-DD`
/// - `YYYY-MM-DD + N` / `YYYY-MM-DD - N` (spaces optional)
fn parse_date_value(value: &str) -> Option<DateValue> {
    let value = value.trim();

    let (anchor, rest) = if let Some(rest) = value.strip_prefix("today") {
        (DateAnchor::Today, rest)
    } else if value.len() >= 10 && is_date_literal(&value[..10]) {
        (DateAnchor::Date(value[..10].to_string()), &value[10..])
    } else {
        return None;
    };

    let rest = rest.trim_start();
    if rest.is_empty() {
        return Some(DateValue { anchor, offset: 0 });
    }

    let (sign, num_str) = if let Some(num) = rest.strip_prefix('+') {
        (1i32, num.trim_start())
    } else if let Some(num) = rest.strip_prefix('-') {
        (-1i32, num.trim_start())
    } else {
        return None;
    };

    let num: i32 = num_str.trim().parse().ok()?;
    Some(DateValue {
        anchor,
        offset: sign * num,
    })
}

fn parse_date_field(s: &str) -> Option<DateField> {
    match s {
        "due" => Some(DateField::Due),
        "scheduled" => Some(DateField::Scheduled),
        "starting" => Some(DateField::Starting),
        "updated" => Some(DateField::Updated),
        "creation_date" => Some(DateField::CreationDate),
        _ => None,
    }
}

pub fn parse_field(s: &str) -> Option<Field> {
    match s {
        "due" => Some(Field::Due),
        "scheduled" => Some(Field::Scheduled),
        "starting" => Some(Field::Starting),
        "updated" => Some(Field::Updated),
        "creation_date" => Some(Field::CreationDate),
        "priority" => Some(Field::Priority),
        "project" => Some(Field::Project),
        "context" => Some(Field::Context),
        "description" => Some(Field::Description),
        "done" => Some(Field::Done),
        _ => None,
    }
}

fn parse_text_field(s: &str) -> Option<TextField> {
    match s {
        "project" => Some(TextField::Project),
        "context" => Some(TextField::Context),
        "description" => Some(TextField::Description),
        _ => None,
    }
}

fn parse_compare_op(s: &str) -> Option<CompareOp> {
    match s {
        "=" => Some(CompareOp::Eq),
        "<" => Some(CompareOp::Lt),
        "<=" => Some(CompareOp::Lte),
        ">" => Some(CompareOp::Gt),
        ">=" => Some(CompareOp::Gte),
        _ => None,
    }
}

fn parse_condition(line: &str) -> Option<Condition> {
    let line = line.trim();

    if line == "done" {
        return Some(Condition::DoneFilter { done: true });
    }
    if line == "not done" {
        return Some(Condition::DoneFilter { done: false });
    }

    if let Some(rest) = line.strip_prefix("has ") {
        if let Some(field) = parse_field(rest.trim()) {
            return Some(Condition::Existence {
                field,
                present: true,
            });
        }
        return None;
    }
    if let Some(rest) = line.strip_prefix("no ") {
        if let Some(field) = parse_field(rest.trim()) {
            return Some(Condition::Existence {
                field,
                present: false,
            });
        }
        return None;
    }

    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() < 3 {
        return None;
    }
    let field_str = parts[0];
    let op_str = parts[1];
    let value_str = parts[2];

    if let (Some(date_field), Some(comp_op), Some(date_value)) = (
        parse_date_field(field_str),
        parse_compare_op(op_str),
        parse_date_value(value_str),
    ) {
        return Some(Condition::DateComparison {
            field: date_field,
            op: comp_op,
            value: date_value,
        });
    }

    if field_str == "priority" {
        let priority_op = match op_str {
            "=" => Some(PriorityOp::Eq),
            "above" => Some(PriorityOp::Above),
            "below" => Some(PriorityOp::Below),
            _ => None,
        };
        let letter_str = value_str.trim();
        let first_char = if letter_str.len() == 1 {
            letter_str.chars().next()
        } else {
            None
        };
        if let (Some(pop), Some(c)) = (priority_op, first_char)
            && c.is_ascii_uppercase()
        {
            return Some(Condition::PriorityComparison { op: pop, letter: c });
        }
    }

    let text_op = match op_str {
        "includes" => Some(TextOp::Includes),
        "excludes" => Some(TextOp::Excludes),
        _ => None,
    };
    if let (Some(text_field), Some(top)) = (parse_text_field(field_str), text_op) {
        return Some(Condition::TextMatch {
            field: text_field,
            op: top,
            text: value_str.to_string(),
        });
    }

    None
}

fn parse_directive(line: &str) -> Option<(bool, Directive)> {
    let line = line.trim();
    let (is_sort, rest) = if let Some(r) = line.strip_prefix("sort by ") {
        (true, r)
    } else if let Some(r) = line.strip_prefix("group by ") {
        (false, r)
    } else {
        return None;
    };

    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
    let field_str = parts[0];
    let direction = if parts.len() == 2 {
        match parts[1].trim() {
            "asc" => Direction::Asc,
            "desc" => Direction::Desc,
            _ => Direction::Asc,
        }
    } else {
        Direction::Asc
    };

    let field = parse_field(field_str)?;
    Some((is_sort, Directive { field, direction }))
}

/// Classified result of parsing a `prefill` line.
enum PrefillLine {
    Project(String),
    Context(String),
    Priority(char),
    Due(DateValue),
    Scheduled(DateValue),
    Starting(DateValue),
}

/// Parse a `prefill FIELD VALUE` line.
///
/// Returns `None` for any malformed input — unknown field, missing value,
/// invalid grammar for the field type. The line is then silently dropped
/// per LISTS.md "Lenient Parsing".
fn parse_prefill_line(line: &str) -> Option<PrefillLine> {
    let rest = line.trim().strip_prefix("prefill ")?;
    let (field, value) = rest.split_once(' ')?;
    let field = field.trim();
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    match field {
        "project" => Some(PrefillLine::Project(value.to_string())),
        "context" => Some(PrefillLine::Context(value.to_string())),
        "priority" => {
            let mut chars = value.chars();
            let first = chars.next()?;
            if chars.next().is_some() || !first.is_ascii_uppercase() {
                return None;
            }
            Some(PrefillLine::Priority(first))
        }
        "due" => parse_date_value(value).map(PrefillLine::Due),
        "scheduled" => parse_date_value(value).map(PrefillLine::Scheduled),
        "starting" => parse_date_value(value).map(PrefillLine::Starting),
        _ => None,
    }
}

/// Resolve template variables `{{dir}}` and `{{dir:N}}` in content.
///
/// - `source_path` is the path to the `.list` file.
/// - `lists_dir` is the root lists directory.
/// - `{{dir}}` (equivalent to `{{dir:0}}`) = immediate parent directory name.
/// - `{{dir:N}}` = ancestor N levels up from immediate parent.
///
/// Returns `None` if any variable would escape beyond `lists_dir`.
fn resolve_template_variables(
    content: &str,
    source_path: &Path,
    lists_dir: &Path,
) -> Option<String> {
    let parent = source_path.parent()?;
    let rel = parent.strip_prefix(lists_dir).ok()?;
    let components: Vec<&str> = rel
        .components()
        .filter_map(|c| {
            if let std::path::Component::Normal(s) = c {
                s.to_str()
            } else {
                None
            }
        })
        .collect();

    if !content.contains("{{dir") {
        return Some(content.to_string());
    }

    let mut result = content.to_string();

    loop {
        let Some(start) = result.find("{{dir:") else {
            break;
        };
        let rest = &result[start + 6..];
        let Some(end) = rest.find("}}") else {
            break;
        };
        let n_str = &rest[..end];
        let n: usize = match n_str.parse() {
            Ok(v) => v,
            Err(_) => return None,
        };

        let index = components.len().checked_sub(1 + n)?;
        let value = components[index];
        let pattern = format!("{{{{dir:{}}}}}", n);
        result = result.replace(&pattern, value);
    }

    if result.contains("{{dir}}") {
        let value = components.last()?;
        result = result.replace("{{dir}}", value);
    }

    Some(result)
}

/// Compute the group_path from source_path relative to lists_dir.
fn compute_group_path(source_path: &Path, lists_dir: &Path) -> Vec<String> {
    let parent = match source_path.parent() {
        Some(p) => p,
        None => return Vec::new(),
    };
    let rel = match parent.strip_prefix(lists_dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    rel.components()
        .filter_map(|c| {
            if let std::path::Component::Normal(s) = c {
                s.to_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect()
}

pub fn parse_list(content: &str, source_path: &Path, lists_dir: &Path) -> SmartList {
    let content = content.replace("\r\n", "\n");

    let default_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let group_path = compute_group_path(source_path, lists_dir);

    let lines: Vec<&str> = content.lines().collect();

    let first_dash = lines.iter().position(|l| l.trim() == "---");
    let second_dash = first_dash.and_then(|start| {
        lines[start + 1..]
            .iter()
            .position(|l| l.trim() == "---")
            .map(|rel| start + 1 + rel)
    });

    let (parse_error, name, icon, description, body_str) = match (first_dash, second_dash) {
        (Some(start), Some(end)) => {
            let frontmatter_lines = &lines[start + 1..end];
            let body_lines = &lines[end + 1..];

            let mut name: Option<String> = None;
            let mut icon: Option<String> = None;
            let mut description: Option<String> = None;

            for fm_line in frontmatter_lines {
                if let Some(colon_pos) = fm_line.find(':') {
                    let key = fm_line[..colon_pos].trim();
                    let value = fm_line[colon_pos + 1..].trim();
                    match key {
                        "name" => name = Some(value.to_string()),
                        "icon" => icon = Some(value.to_string()),
                        "description" => description = Some(value.to_string()),
                        _ => {}
                    }
                }
            }

            let resolved_name = name.unwrap_or_else(|| default_name.clone());
            let body = body_lines.join("\n");
            (None, resolved_name, icon, description, body)
        }
        _ => {
            let err = "missing frontmatter delimiters".to_string();
            let body = lines.join("\n");
            (Some(err), default_name, None, None, body)
        }
    };

    let resolved_body = match resolve_template_variables(&body_str, source_path, lists_dir) {
        Some(body) => body,
        None => {
            return SmartList {
                name,
                icon,
                description,
                group_path,
                source_path: source_path.to_path_buf(),
                parse_error: Some("template variable escapes lists.d boundary".to_string()),
                blocks: Vec::new(),
                sort_directives: Vec::new(),
                group_directives: Vec::new(),
                prefill: Prefill::default(),
            };
        }
    };

    let body_lines: Vec<&str> = resolved_body.lines().collect();
    let mut raw_blocks: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in &body_lines {
        if line.trim() == "OR" {
            raw_blocks.push(current);
            current = Vec::new();
        } else {
            current.push(line);
        }
    }
    raw_blocks.push(current);

    let mut blocks: Vec<FilterBlock> = Vec::new();
    let mut sort_directives: Vec<Directive> = Vec::new();
    let mut group_directives: Vec<Directive> = Vec::new();
    let mut prefill = Prefill::default();

    for raw_block in raw_blocks {
        let mut conditions: Vec<Condition> = Vec::new();

        for line in raw_block {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with("sort by ") || trimmed.starts_with("group by ") {
                if let Some((is_sort, directive)) = parse_directive(trimmed) {
                    if is_sort {
                        sort_directives.push(directive);
                    } else {
                        group_directives.push(directive);
                    }
                }
                continue;
            }

            if trimmed.starts_with("prefill ") {
                if let Some(p) = parse_prefill_line(trimmed) {
                    apply_prefill(&mut prefill, p);
                }
                continue;
            }

            if let Some(condition) = parse_condition(trimmed) {
                conditions.push(condition);
            }
        }

        if !conditions.is_empty() {
            blocks.push(FilterBlock { conditions });
        }
    }

    SmartList {
        name,
        icon,
        description,
        group_path,
        source_path: source_path.to_path_buf(),
        parse_error,
        blocks,
        sort_directives,
        group_directives,
        prefill,
    }
}

fn apply_prefill(prefill: &mut Prefill, line: PrefillLine) {
    match line {
        PrefillLine::Project(v) => prefill.projects.push(v),
        PrefillLine::Context(v) => prefill.contexts.push(v),
        PrefillLine::Priority(c) => {
            if prefill.priority.is_none() {
                prefill.priority = Some(c);
            }
        }
        PrefillLine::Due(v) => {
            if prefill.due.is_none() {
                prefill.due = Some(v);
            }
        }
        PrefillLine::Scheduled(v) => {
            if prefill.scheduled.is_none() {
                prefill.scheduled = Some(v);
            }
        }
        PrefillLine::Starting(v) => {
            if prefill.starting.is_none() {
                prefill.starting = Some(v);
            }
        }
    }
}
