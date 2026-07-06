//! Minimal keyword highlighter for `.list` smart-list source files.
//!
//! Splits a line into ratatui-styleable spans — frontmatter delimiter,
//! comment, keyword, field name, operator, and the rest — without parsing
//! the grammar or validating its structure. Wrong syntax is rendered as
//! plain text rather than rejected.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

const KEYWORDS: &[&str] = &[
    "OR", "done", "not", "has", "no", "sort", "by", "group", "prefill", "today", "asc", "desc",
    "above", "below", "includes", "excludes",
];

const FIELD_NAMES: &[&str] = &[
    "due",
    "scheduled",
    "starting",
    "updated",
    "creation_date",
    "priority",
    "project",
    "context",
    "description",
];

/// Classify a single source line into ratatui spans. Frontmatter lines
/// (between the two `---` delimiters) take a `key: value` rendering;
/// body lines are tokenized whitespace-separated.
pub fn highlight_line(line: &str, in_frontmatter: bool) -> Vec<Span<'static>> {
    let trimmed_start = line.trim_start();
    let leading_ws_len = line.len() - trimmed_start.len();

    if trimmed_start == "---" {
        return vec![
            owned_span(&line[..leading_ws_len], Style::default()),
            owned_span("---", style_delimiter()),
        ];
    }

    if trimmed_start.starts_with('#') {
        return vec![owned_span(line, style_comment())];
    }

    if in_frontmatter {
        return highlight_frontmatter_line(line);
    }

    highlight_body_line(line)
}

/// Split a string into `(key, value)` at the first `:` and style the key
/// distinctly. Lines without `:` fall back to plain rendering.
fn highlight_frontmatter_line(line: &str) -> Vec<Span<'static>> {
    let Some(colon) = line.find(':') else {
        return vec![owned_span(line, Style::default())];
    };
    vec![
        owned_span(&line[..colon], style_frontmatter_key()),
        owned_span(":", Style::default()),
        owned_span(&line[colon + 1..], Style::default()),
    ]
}

/// Tokenize a body line on whitespace and style each token by class.
/// Whitespace runs are preserved as their own (unstyled) spans so the
/// rendered line is a faithful reproduction of the source.
fn highlight_body_line(line: &str) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut idx = 0usize;
    let bytes = line.as_bytes();

    while idx < bytes.len() {
        if bytes[idx].is_ascii_whitespace() {
            let start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            spans.push(owned_span(&line[start..idx], Style::default()));
            continue;
        }
        let start = idx;
        while idx < bytes.len() && !bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        let tok = &line[start..idx];
        spans.push(owned_span(tok, classify_token(tok)));
    }
    spans
}

fn classify_token(tok: &str) -> Style {
    if KEYWORDS.contains(&tok) {
        return style_keyword();
    }
    if FIELD_NAMES.contains(&tok) {
        return style_field();
    }
    if matches!(tok, "<=" | ">=" | "<" | ">" | "=") {
        return style_operator();
    }
    Style::default()
}

fn owned_span(s: &str, style: Style) -> Span<'static> {
    Span::styled(s.to_string(), style)
}

fn style_keyword() -> Style {
    Style::default()
        .fg(Color::Magenta)
        .add_modifier(Modifier::BOLD)
}

fn style_field() -> Style {
    Style::default().fg(Color::Green)
}

fn style_operator() -> Style {
    Style::default().fg(Color::Yellow)
}

fn style_delimiter() -> Style {
    Style::default().fg(Color::Cyan)
}

fn style_comment() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::ITALIC)
}

fn style_frontmatter_key() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}

/// Walk the full source, tracking frontmatter scope, and return one
/// styled `Vec<Span>` per source line. Frontmatter is the region between
/// the first two `---` lines; everything else is body.
pub fn highlight_source(content: &str) -> Vec<Vec<Span<'static>>> {
    let mut in_fm = false;
    let mut seen_first = false;
    let mut out = Vec::new();
    for line in content.lines() {
        if line.trim() == "---" {
            out.push(highlight_line(line, in_fm));
            if !seen_first {
                in_fm = true;
                seen_first = true;
            } else {
                in_fm = false;
            }
            continue;
        }
        out.push(highlight_line(line, in_fm));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styles_for(line: &str, in_fm: bool) -> Vec<(String, Style)> {
        highlight_line(line, in_fm)
            .into_iter()
            .map(|s| (s.content.into_owned(), s.style))
            .collect()
    }

    #[test]
    fn comment_line_uses_comment_style() {
        let spans = styles_for("# this is a comment", false);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, "# this is a comment");
        assert_eq!(spans[0].1.fg, Some(Color::DarkGray));
    }

    #[test]
    fn frontmatter_delimiter_is_cyan() {
        let spans = styles_for("---", false);
        // leading ws span (empty) + "---" span
        assert!(
            spans
                .iter()
                .any(|(t, s)| t == "---" && s.fg == Some(Color::Cyan))
        );
    }

    #[test]
    fn keywords_get_magenta_bold() {
        let spans = styles_for("sort by priority asc", false);
        let sort_style = spans.iter().find(|(t, _)| t == "sort").unwrap().1;
        let by_style = spans.iter().find(|(t, _)| t == "by").unwrap().1;
        let asc_style = spans.iter().find(|(t, _)| t == "asc").unwrap().1;
        assert_eq!(sort_style.fg, Some(Color::Magenta));
        assert_eq!(by_style.fg, Some(Color::Magenta));
        assert_eq!(asc_style.fg, Some(Color::Magenta));
    }

    #[test]
    fn field_names_get_green() {
        let spans = styles_for("due <= today", false);
        let due_style = spans.iter().find(|(t, _)| t == "due").unwrap().1;
        assert_eq!(due_style.fg, Some(Color::Green));
    }

    #[test]
    fn comparison_operators_get_yellow() {
        let spans = styles_for("due <= today", false);
        let op_style = spans.iter().find(|(t, _)| t == "<=").unwrap().1;
        assert_eq!(op_style.fg, Some(Color::Yellow));
    }

    #[test]
    fn frontmatter_key_is_styled_distinctly() {
        let spans = styles_for("name: Today", true);
        // First span is "name", styled as frontmatter key.
        assert_eq!(spans[0].0, "name");
        assert_eq!(spans[0].1.fg, Some(Color::Blue));
        assert_eq!(spans[1].0, ":");
    }

    #[test]
    fn highlight_source_tracks_frontmatter_scope() {
        let content = "---\nname: Today\n---\ndue <= today\n";
        let lines = highlight_source(content);
        assert_eq!(lines.len(), 4);
        // line 1 = "name: Today" — name is frontmatter key (Blue)
        let name_span = lines[1].iter().find(|s| s.content == "name").unwrap();
        assert_eq!(name_span.style.fg, Some(Color::Blue));
        // line 3 = "due <= today" — due is field (Green), not frontmatter key
        let due_span = lines[3].iter().find(|s| s.content == "due").unwrap();
        assert_eq!(due_span.style.fg, Some(Color::Green));
    }

    #[test]
    fn unknown_token_falls_back_to_default_style() {
        let spans = styles_for("project includes randomvalue", false);
        let value_span = spans
            .iter()
            .find(|(t, _)| t == "randomvalue")
            .expect("token present");
        assert_eq!(value_span.1.fg, None);
    }
}
