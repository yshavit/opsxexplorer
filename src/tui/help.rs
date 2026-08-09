use ratatui::style::{Modifier, Style, Styled};
use ratatui::text::{Line, Span};

/// One row of the help modal's static keybinding table: the key label and a
/// succinct description of its effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub key: &'static str,
    pub description: &'static str,
}

/// Keys bound identically regardless of which pane holds focus (spec.md).
pub const GLOBAL: &[Entry] = &[
    Entry {
        key: "q",
        description: "Quit",
    },
    Entry {
        key: "?",
        description: "Toggle this help",
    },
    Entry {
        key: "Tab",
        description: "Switch focus between panes",
    },
    Entry {
        key: "j / k / ↓ / ↑",
        description: "Move selection",
    },
    Entry {
        key: "Ctrl+d / Ctrl+u",
        description: "Move selection by half a page",
    },
    Entry {
        key: "Enter / Space",
        description: "Activate the selected row",
    },
];

/// Keys the left pane alone binds.
pub const LEFT_PANE: &[Entry] = &[
    Entry {
        key: "h / l / ← / →",
        description: "Scroll left/right",
    },
    Entry {
        key: "^ / Home / $ / End",
        description: "Jump to start/end of scroll",
    },
];

/// Keys the right pane alone binds.
pub const RIGHT_PANE: &[Entry] = &[
    Entry {
        key: "h / l / ← / →",
        description: "Expand/collapse the selected row",
    },
    Entry {
        key: "[ / ]",
        description: "Previous/next tab",
    },
];

/// The width of the widest key label across every section, so every entry's
/// description lines up in a common column regardless of which section it's in.
pub fn key_column_width() -> usize {
    GLOBAL
        .iter()
        .chain(LEFT_PANE)
        .chain(RIGHT_PANE)
        .map(|e| e.key.chars().count())
        .max()
        .unwrap_or(0)
}

/// One level of indent under a section heading.
const ENTRY_INDENT: &str = "  ";

/// The help modal's full content, always in full: there's nothing to
/// expand or collapse, so this is the only content the modal ever shows,
/// and its shape (and therefore the modal's width) never varies with scroll
/// position (see design.md).
pub fn lines() -> Vec<Line<'static>> {
    let key_width = key_column_width();
    let mut lines = Vec::new();
    push_section(&mut lines, "Global", GLOBAL, key_width);
    lines.push(Line::default());
    push_section(&mut lines, "Left pane", LEFT_PANE, key_width);
    lines.push(Line::default());
    push_section(&mut lines, "Right pane", RIGHT_PANE, key_width);
    lines
}

fn push_section(
    lines: &mut Vec<Line<'static>>,
    heading: &'static str,
    entries: &'static [Entry],
    key_width: usize,
) {
    lines.push(heading_line(heading));
    lines.extend(entries.iter().map(|entry| entry_line(entry, key_width)));
}

fn heading_line(text: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::new().add_modifier(Modifier::UNDERLINED).bold(),
    ))
}

fn entry_line(entry: &Entry, key_width: usize) -> Line<'static> {
    let key_segments = entry.key.split('/').collect::<Vec<_>>();
    let mut spans = Vec::with_capacity(key_segments.len() * 2 + 2);
    spans.push(Span::from(ENTRY_INDENT));

    let mut key_desc_len = 0;
    let mut keys_iter = key_segments.into_iter().peekable();
    while let Some(key) = keys_iter.next() {
        key_desc_len += key.chars().count();
        spans.push(Span::from(key));
        if keys_iter.peek().is_some() {
            key_desc_len += 1;
            spans.push(Span::from("/").set_style(Style::new().dim()));
        }
    }
    let remainder = format!(
        "{:<width$}  {}",
        "",
        entry.description,
        width = key_width.saturating_sub(key_desc_len)
    );
    spans.push(Span::from(remainder));

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn content_starts_with_the_global_heading() {
        let lines = lines();
        assert_eq!(text(&lines[0]), "Global");
        assert!(
            lines[0].spans[0]
                .style
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
    }

    #[test]
    fn a_blank_line_separates_each_section() {
        let lines = lines();
        let universal_end = 1 + GLOBAL.len();
        assert_eq!(text(&lines[universal_end]), "");

        let left_heading = universal_end + 1;
        assert_eq!(text(&lines[left_heading]), "Left pane");
        let left_end = left_heading + 1 + LEFT_PANE.len();
        assert_eq!(text(&lines[left_end]), "");

        let right_heading = left_end + 1;
        assert_eq!(text(&lines[right_heading]), "Right pane");
    }

    #[test]
    fn content_length_is_fixed() {
        // No expand/collapse state exists anymore, so this is the only shape
        // the content ever takes — asserted here so a future regression that
        // reintroduces state-dependent content is caught immediately.
        let expected = 3 // headings
            + GLOBAL.len() + LEFT_PANE.len() + RIGHT_PANE.len()
            + 2; // blank separators
        assert_eq!(lines().len(), expected);
    }

    #[test]
    fn entries_render_key_and_description() {
        let key_width = key_column_width();
        let line = entry_line(&GLOBAL[0], key_width);
        let rendered = text(&line);
        assert!(rendered.contains(GLOBAL[0].key));
        assert!(rendered.contains(GLOBAL[0].description));
    }
}
