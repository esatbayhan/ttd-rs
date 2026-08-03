use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::app::AppState;
use crate::parser::display_date;
use crate::task::Task;

pub fn help_bar_text(app: &AppState) -> String {
    help_bar_entries(app)
        .iter()
        .map(|(key, desc)| format!("{key} {desc}"))
        .collect::<Vec<_>>()
        .join(" │ ")
}

pub fn render_help_bar(app: &AppState) -> Paragraph<'static> {
    let entries = help_bar_entries(app);
    let mut spans = Vec::new();
    for (i, (key, desc)) in entries.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(
            key.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {desc}"),
            Style::default().fg(Color::Gray),
        ));
    }
    Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray))
}

fn help_bar_entries(app: &AppState) -> Vec<(&'static str, &'static str)> {
    if app.save_conflict.is_some() {
        return vec![("r", "reload"), ("o", "overwrite"), ("c", "cancel")];
    }
    if app.list_viewer.is_some() {
        return vec![
            ("j/k", "scroll"),
            ("e", "edit externally"),
            ("esc", "close"),
        ];
    }
    if app.about.is_some() {
        return vec![("esc/ent/i/q", "close")];
    }
    if app.picker.is_some() {
        return vec![("j/k", "nav"), ("enter", "select"), ("esc", "cancel")];
    }
    if app.editor.as_ref().is_some_and(|e| e.shortcut.is_some()) {
        return vec![("enter", "apply"), ("esc", "cancel")];
    }
    if app.editor.is_some() {
        return vec![
            ("enter", "save"),
            ("esc", "cancel"),
            ("ctrl+d", "due"),
            ("ctrl+s", "sched"),
            ("ctrl+t", "start"),
        ];
    }
    if app.confirm_delete {
        return vec![("enter", "confirm"), ("esc", "cancel")];
    }
    if app.search_active {
        return vec![("esc", "cancel"), ("n", "next"), ("N", "prev")];
    }
    vec![
        ("j/k", "nav"),
        ("h/l", "focus"),
        ("spc", "toggle"),
        ("a", "add"),
        ("e", "edit/view"),
        ("x", "toggle done"),
        ("u", "update"),
        ("U", "undo"),
        ("D", "delete"),
        ("s", "sort"),
        ("o", "group"),
        ("r", "reverse"),
        ("/", "search"),
        ("ctrl+b", "sidebar"),
        ("ctrl+←/→", "resize"),
        ("?", "toggle hints"),
        ("i", "about"),
        ("q", "quit"),
    ]
}

pub fn task_line_text(task: &Task, is_selected: bool) -> String {
    let marker = if is_selected {
        "> "
    } else if task.done {
        "\u{2713} "
    } else {
        "  "
    };
    let description = strip_date_tags(&task.description);
    let mut result = format!("{marker}{description}");
    let tag_line = build_tag_line(task);
    if !tag_line.is_empty() {
        result.push('\n');
        result.push_str("    ");
        result.push_str(&tag_line);
    }
    result
}

pub fn render_task_lines<'a>(task: &Task, is_selected: bool, wrap_width: u16) -> Vec<Line<'a>> {
    let marker = if is_selected {
        "> "
    } else if task.done {
        "\u{2713} "
    } else {
        "  "
    };
    let description = strip_date_tags(&task.description);
    let mut lines = wrap_colored_description(marker, &description, wrap_width);
    let tag_spans = build_tag_spans(task);
    if !tag_spans.is_empty() {
        let mut spans = vec![Span::raw("    ")];
        spans.extend(tag_spans);
        lines.push(Line::from(spans));
    }
    if task.done {
        let dim = Style::default().add_modifier(Modifier::DIM);
        lines = lines
            .into_iter()
            .map(|line| {
                Line::from(
                    line.spans
                        .into_iter()
                        .map(|span| Span::styled(span.content, dim))
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
    }
    lines
}

fn strip_date_tags(description: &str) -> String {
    description
        .split_whitespace()
        .filter(|token| {
            !token.starts_with("due:")
                && !token.starts_with("scheduled:")
                && !token.starts_with("starting:")
                && !token.starts_with("updated:")
                && !token.starts_with("created:")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn wrap_colored_description(marker: &str, description: &str, width: u16) -> Vec<Line<'static>> {
    let width = width as usize;
    let indent = "  ";
    let prefix_len = marker.len();
    let words: Vec<&str> = description.split_whitespace().collect();

    if words.is_empty() {
        return vec![Line::from(Span::raw(marker.to_string()))];
    }

    let mut result: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = vec![Span::raw(marker.to_string())];
    let mut col = prefix_len;

    for word in &words {
        let need_space = col > prefix_len;
        let extra = if need_space { 1 } else { 0 };

        if col + extra + word.len() > width && col > prefix_len {
            result.push(Line::from(spans));
            spans = vec![Span::raw(indent.to_string())];
            col = prefix_len;
        } else if need_space {
            spans.push(Span::raw(" "));
            col += 1;
        }

        // If the word is longer than the remaining space on this line,
        // split it into chunks so each continuation line gets the
        // hanging indent instead of wrapping flush-left.
        let remaining = width.saturating_sub(col);
        if word.len() > remaining && width > prefix_len {
            let chars: Vec<char> = word.chars().collect();
            let mut pos = 0;

            // Fill the rest of the current line
            if remaining > 0 {
                let end = remaining.min(chars.len());
                spans.push(color_token(&chars[..end].iter().collect::<String>()));
                pos = end;
            }

            // Each subsequent chunk gets its own indented line
            let chunk_size = width - prefix_len;
            while pos < chars.len() {
                result.push(Line::from(spans));
                spans = vec![Span::raw(indent.to_string())];
                let end = (pos + chunk_size).min(chars.len());
                spans.push(color_token(&chars[pos..end].iter().collect::<String>()));
                col = prefix_len + (end - pos);
                pos = end;
            }
        } else {
            spans.push(color_token(word));
            col += word.len();
        }
    }

    result.push(Line::from(spans));
    result
}

pub(crate) fn color_token(word: &str) -> Span<'static> {
    if word.starts_with('+') {
        return Span::styled(word.to_string(), Style::default().fg(Color::Cyan));
    }
    if word.starts_with('@') {
        return Span::styled(word.to_string(), Style::default().fg(Color::Green));
    }

    let bytes = word.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'(' && bytes[2] == b')' && bytes[1].is_ascii_uppercase() {
        return Span::styled(word.to_string(), Style::default().fg(Color::Yellow));
    }

    if word.len() >= 10 && is_date_lax(&word[..10.min(word.len())]) {
        if word.len() == 10 {
            return Span::styled(word.to_string(), Style::default().fg(Color::Magenta));
        }
        if word.len() >= 16 && word.as_bytes().get(10) == Some(&b'T') {
            let time_part = &word[11..];
            if time_part.len() >= 5 && time_part.chars().filter(|c| c == &':').count() == 1 {
                return Span::styled(word.to_string(), Style::default().fg(Color::Magenta));
            }
        }
    }

    let tag_colors: &[(&[&str], Color)] = &[
        (&["due"], Color::Yellow),
        (&["scheduled", "sched"], Color::Cyan),
        (&["starting", "start"], Color::Green),
        (&["updated"], Color::Blue),
        (&["created"], Color::Magenta),
    ];
    if let Some(colon) = word.find(':') {
        let key = &word[..colon];
        for (keys, color) in tag_colors {
            if keys.contains(&key) {
                return Span::styled(word.to_string(), Style::default().fg(*color));
            }
        }
    }

    Span::raw(word.to_string())
}

fn is_date_lax(part: &str) -> bool {
    if part.len() != 10 {
        return false;
    }
    let bytes = part.as_bytes();
    if !(bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit())
    {
        return false;
    }
    let month: u32 = part[5..7].parse().unwrap_or(0);
    let day: u32 = part[8..10].parse().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

pub(crate) fn highlight_editor_text(line: &str, wrap_width: usize) -> Vec<Line<'static>> {
    if line.is_empty() {
        return vec![Line::default()];
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut idx = 0usize;
    let bytes = line.as_bytes();

    while idx < bytes.len() {
        if bytes[idx].is_ascii_whitespace() {
            let start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            spans.push(Span::raw(line[start..idx].to_string()));
            continue;
        }
        let start = idx;
        while idx < bytes.len() && !bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        let tok = &line[start..idx];
        spans.push(color_token(tok));
    }

    char_wrap_spans(spans, wrap_width)
}

fn char_wrap_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![Line::from(spans)];
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut col: usize = 0;

    for span in spans {
        let chars: Vec<char> = span.content.chars().collect();
        let style = span.style;

        let mut pos = 0;
        while pos < chars.len() {
            let remaining = width.saturating_sub(col);
            if remaining == 0 {
                lines.push(Line::from(std::mem::take(&mut current)));
                col = 0;
                continue;
            }
            let take = remaining.min(chars.len() - pos);
            let chunk: String = chars[pos..pos + take].iter().collect();
            current.push(Span::styled(chunk, style));
            col += take;
            pos += take;
            if col >= width {
                lines.push(Line::from(std::mem::take(&mut current)));
                col = 0;
            }
        }
    }

    if !current.is_empty() {
        lines.push(Line::from(current));
    }

    if lines.is_empty() {
        lines.push(Line::default());
    }

    lines
}

fn build_tag_line(task: &Task) -> String {
    let mut parts = Vec::new();
    if let Some(due) = task.tags.get("due") {
        parts.push(format!("due: {}", display_date(due)));
    }
    if let Some(scheduled) = task.tags.get("scheduled") {
        parts.push(format!("sched: {}", display_date(scheduled)));
    }
    if let Some(starting) = task.tags.get("starting") {
        parts.push(format!("start: {}", display_date(starting)));
    }
    if let Some(updated) = task.tags.get("updated") {
        parts.push(format!("updated: {}", display_date(updated)));
    }
    if let Some(created) = task.creation_date.as_deref() {
        parts.push(format!("created: {}", display_date(created)));
    }
    parts.join("  ")
}

fn build_tag_spans(task: &Task) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    if let Some(due) = task.tags.get("due") {
        spans.push(Span::styled(
            format!("due: {}", display_date(due)),
            Style::default().fg(Color::Yellow),
        ));
    }
    if let Some(scheduled) = task.tags.get("scheduled") {
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("sched: {}", display_date(scheduled)),
            Style::default().fg(Color::Cyan),
        ));
    }
    if let Some(starting) = task.tags.get("starting") {
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("start: {}", display_date(starting)),
            Style::default().fg(Color::Green),
        ));
    }
    if let Some(updated) = task.tags.get("updated") {
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("updated: {}", display_date(updated)),
            Style::default().fg(Color::Blue),
        ));
    }
    if let Some(created) = task.creation_date.as_deref() {
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("created: {}", display_date(created)),
            Style::default().fg(Color::Magenta),
        ));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_is_cyan() {
        let span = color_token("+Family");
        assert_eq!(span.style.fg, Some(Color::Cyan));
    }

    #[test]
    fn context_is_green() {
        let span = color_token("@phone");
        assert_eq!(span.style.fg, Some(Color::Green));
    }

    #[test]
    fn priority_is_yellow() {
        let span = color_token("(A)");
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn priority_with_leading_done_marker_is_plain() {
        let span = color_token("x");
        assert_eq!(span.style.fg, None);
        assert_eq!(span.style.add_modifier, Modifier::empty());
    }

    #[test]
    fn bare_date_is_magenta() {
        let span = color_token("2026-07-17");
        assert_eq!(span.style.fg, Some(Color::Magenta));
    }

    #[test]
    fn date_with_time_is_magenta() {
        let span = color_token("2026-07-17T14:30");
        assert_eq!(span.style.fg, Some(Color::Magenta));
    }

    #[test]
    fn invalid_date_is_plain() {
        let span = color_token("2026-13-45");
        assert_eq!(span.style.fg, None);
    }

    #[test]
    fn due_tag_is_yellow() {
        let span = color_token("due:2026-08-01");
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn due_tag_incomplete_is_yellow() {
        let span = color_token("due:");
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn sched_tag_is_cyan() {
        let span = color_token("sched:2026-08-01");
        assert_eq!(span.style.fg, Some(Color::Cyan));
    }

    #[test]
    fn start_tag_is_green() {
        let span = color_token("start:2026-08-01");
        assert_eq!(span.style.fg, Some(Color::Green));
    }

    #[test]
    fn updated_tag_is_blue() {
        let span = color_token("updated:2026-08-01");
        assert_eq!(span.style.fg, Some(Color::Blue));
    }

    #[test]
    fn created_tag_is_magenta() {
        let span = color_token("created:2026-08-01");
        assert_eq!(span.style.fg, Some(Color::Magenta));
    }

    #[test]
    fn unknown_key_is_plain() {
        let span = color_token("foo:bar");
        assert_eq!(span.style.fg, None);
    }

    #[test]
    fn plain_word_is_unstyled() {
        let span = color_token("hello");
        assert_eq!(span.style.fg, None);
    }

    #[test]
    fn highlight_empty_string_returns_empty() {
        let lines = highlight_editor_text("", 40);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans.is_empty());
    }

    #[test]
    fn highlight_preserves_plain_text() {
        let lines = highlight_editor_text("hello world", 40);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(text, "hello world");
    }

    #[test]
    fn highlight_colors_project_in_text() {
        let lines = highlight_editor_text("call +Family", 40);
        let spans = &lines[0].spans;
        let family = spans
            .iter()
            .find(|s| s.content.contains("+Family"))
            .unwrap();
        assert_eq!(family.style.fg, Some(Color::Cyan));
    }

    #[test]
    fn highlight_wraps_at_width_boundary() {
        let lines = highlight_editor_text("1234567890", 5);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn highlight_preserves_leading_whitespace() {
        let lines = highlight_editor_text("  hello", 40);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(text, "  hello");
    }

    #[test]
    fn highlight_preserves_multiple_spaces() {
        let lines = highlight_editor_text("a   b", 40);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(text, "a   b");
    }
}
