use std::path::Path;

use super::types::*;

fn parse_date_offset(value: &str) -> Option<i32> {
    let value = value.trim();
    if value == "today" {
        return Some(0);
    }
    // Strip "today" prefix then parse optional offset
    let rest = value.strip_prefix("today")?;
    let rest = rest.trim();
    if rest.is_empty() {
        return Some(0);
    }
    if let Some(rest) = rest.strip_prefix('+') {
        let num: i32 = rest.trim().parse().ok()?;
        Some(num)
    } else if let Some(rest) = rest.strip_prefix('-') {
        let num: i32 = rest.trim().parse().ok()?;
        Some(-num)
    } else {
        None
    }
}

fn parse_date_field(s: &str) -> Option<DateField> {
    match s {
        "due" => Some(DateField::Due),
        "scheduled" => Some(DateField::Scheduled),
        "starting" => Some(DateField::Starting),
        "creation_date" => Some(DateField::CreationDate),
        _ => None,
    }
}

pub fn parse_field(s: &str) -> Option<Field> {
    match s {
        "due" => Some(Field::Due),
        "scheduled" => Some(Field::Scheduled),
        "starting" => Some(Field::Starting),
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

    // done / not done -- check before "no" prefix
    if line == "done" {
        return Some(Condition::DoneFilter { done: true });
    }
    if line == "not done" {
        return Some(Condition::DoneFilter { done: false });
    }

    // existence: "has <field>" or "no <field>"
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

    // comparison: field op value
    // Split into at most 3 parts: field, operator, value
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() < 3 {
        return None;
    }
    let field_str = parts[0];
    let op_str = parts[1];
    let value_str = parts[2];

    // date comparison
    if let (Some(date_field), Some(comp_op), Some(offset)) = (
        parse_date_field(field_str),
        parse_compare_op(op_str),
        parse_date_offset(value_str),
    ) {
        return Some(Condition::DateComparison {
            field: date_field,
            op: comp_op,
            offset,
        });
    }

    // priority comparison
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
            return Some(Condition::PriorityComparison {
                op: pop,
                letter: c,
            });
        }
    }

    // text comparison
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
    // Returns (is_sort, Directive) or None
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

/// Resolve template variables `{{dir}}` and `{{dir:N}}` in content.
///
/// - `source_path` is the path to the `.list` file.
/// - `lists_dir` is the root lists directory.
/// - `{{dir}}` (equivalent to `{{dir:0}}`) = immediate parent directory name.
/// - `{{dir:N}}` = ancestor N levels up from immediate parent.
///
/// Returns `None` if any variable would escape beyond `lists_dir`.
fn resolve_template_variables(content: &str, source_path: &Path, lists_dir: &Path) -> Option<String> {
    // Compute the relative path components from lists_dir to source_path's parent
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

    // If no template variables present, skip regex overhead
    if !content.contains("{{dir") {
        return Some(content.to_string());
    }

    let mut result = content.to_string();

    // Process {{dir:N}} patterns (must be done before {{dir}} to avoid partial matches)
    // Use a loop to find and replace all occurrences
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

        // components are ordered root-to-leaf, e.g. ["work", "client-a"]
        // dir:0 = immediate parent = last component
        // dir:1 = one level up = second-to-last component
        let index = components.len().checked_sub(1 + n)?;
        let value = components[index];
        let pattern = format!("{{{{dir:{}}}}}", n);
        result = result.replace(&pattern, value);
    }

    // Process {{dir}} (equivalent to {{dir:0}})
    if result.contains("{{dir}}") {
        let value = components.last()?;
        result = result.replace("{{dir}}", value);
    }

    Some(result)
}

/// Compute the group_path from source_path relative to lists_dir.
///
/// For `lists_dir/work/client-a/review.list`, returns `["work", "client-a"]`.
/// For `lists_dir/today.list` (root level), returns `[]`.
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
    // Normalize line endings
    let content = content.replace("\r\n", "\n");

    // Default name from filename stem
    let default_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    // Compute group_path
    let group_path = compute_group_path(source_path, lists_dir);

    // Split into lines and find frontmatter delimiters
    let lines: Vec<&str> = content.lines().collect();

    // Find first and second "---" lines
    let first_dash = lines.iter().position(|l| l.trim() == "---");
    let second_dash = first_dash.and_then(|start| {
        lines[start + 1..]
            .iter()
            .position(|l| l.trim() == "---")
            .map(|rel| start + 1 + rel)
    });

    let (parse_error, name, icon, description, body_str) =
        match (first_dash, second_dash) {
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
                            _ => {} // unknown keys ignored
                        }
                    }
                }

                let resolved_name = name.unwrap_or_else(|| default_name.clone());
                let body = body_lines.join("\n");
                (None, resolved_name, icon, description, body)
            }
            _ => {
                // No valid frontmatter delimiters
                let err = "missing frontmatter delimiters".to_string();
                let body = lines.join("\n");
                (Some(err), default_name, None, None, body)
            }
        };

    // Resolve template variables before parsing filters.
    // Per spec: if resolution fails (variable escapes lists_dir boundary),
    // the list is invalid — record a parse error and return empty filters.
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
            };
        }
    };

    // Split body by "OR" lines
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

    for raw_block in raw_blocks {
        let mut conditions: Vec<Condition> = Vec::new();

        for line in raw_block {
            let trimmed = line.trim();
            // Skip blank lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Try directive first
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

            // Try condition
            if let Some(condition) = parse_condition(trimmed) {
                conditions.push(condition);
            }
            // Unrecognized lines silently skipped
        }

        // Only add a block if it has conditions (empty blocks from trailing OR etc are skipped)
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
    }
}
