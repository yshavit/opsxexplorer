use ratatui::style::Style;
use ratatui::text::Span;

/// Wraps a row's spans to `width` display columns, breaking on word
/// boundaries and preserving each character's original style across a
/// break. Character-based, not byte-based, so multi-byte content never
/// panics on a mid-character split. A single word longer than `width` is
/// hard-broken rather than left to overflow. `width == 0` is treated as `1`
/// so wrapping always terminates rather than looping forever.
pub fn wrap_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Vec<Span<'static>>> {
    let width = width.max(1);

    let chars: Vec<(char, Style)> = spans
        .into_iter()
        .flat_map(|span| {
            let style = span.style;
            span.content
                .chars()
                .collect::<Vec<_>>()
                .into_iter()
                .map(move |c| (c, style))
        })
        .collect();

    let mut lines: Vec<Vec<(char, Style)>> = Vec::new();
    let mut line: Vec<(char, Style)> = Vec::new();

    for token in tokenize(&chars) {
        match token {
            Token::Space(space) => {
                if line.is_empty() {
                    // Never start a wrapped line with whitespace.
                    continue;
                }
                if line.len() + space.len() <= width {
                    line.extend(space);
                } else {
                    lines.push(trim_trailing_space(std::mem::take(&mut line)));
                }
            }
            Token::Word(word) => {
                if word.len() > width {
                    if !line.is_empty() {
                        lines.push(trim_trailing_space(std::mem::take(&mut line)));
                    }
                    let mut rest = word.as_slice();
                    while rest.len() > width {
                        let (head, tail) = rest.split_at(width);
                        lines.push(head.to_vec());
                        rest = tail;
                    }
                    line = rest.to_vec();
                } else if line.len() + word.len() <= width {
                    line.extend(word);
                } else {
                    lines.push(trim_trailing_space(std::mem::take(&mut line)));
                    line = word;
                }
            }
        }
    }
    lines.push(trim_trailing_space(line));

    lines.into_iter().map(chars_to_spans).collect()
}

/// Drops trailing whitespace characters, so a wrap point never leaves a
/// dangling space at the end of a rendered line.
fn trim_trailing_space(mut line: Vec<(char, Style)>) -> Vec<(char, Style)> {
    while matches!(line.last(), Some((c, _)) if c.is_whitespace()) {
        line.pop();
    }
    line
}

enum Token {
    Word(Vec<(char, Style)>),
    Space(Vec<(char, Style)>),
}

/// Splits a char stream into maximal whitespace and non-whitespace runs, in order.
fn tokenize(chars: &[(char, Style)]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current: Vec<(char, Style)> = Vec::new();
    let mut current_is_space = false;

    for &(c, style) in chars {
        let is_space = c.is_whitespace();
        if !current.is_empty() && is_space != current_is_space {
            tokens.push(finish_token(std::mem::take(&mut current), current_is_space));
        }
        current_is_space = is_space;
        current.push((c, style));
    }
    if !current.is_empty() {
        tokens.push(finish_token(current, current_is_space));
    }
    tokens
}

fn finish_token(chars: Vec<(char, Style)>, is_space: bool) -> Token {
    if is_space {
        Token::Space(chars)
    } else {
        Token::Word(chars)
    }
}

/// Merges adjacent same-styled characters back into spans.
fn chars_to_spans(chars: Vec<(char, Style)>) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current: Option<(String, Style)> = None;
    for (c, style) in chars {
        match &mut current {
            Some((s, st)) if *st == style => s.push(c),
            _ => {
                if let Some((s, st)) = current.take() {
                    spans.push(Span::styled(s, st));
                }
                current = Some((c.to_string(), style));
            }
        }
    }
    if let Some((s, st)) = current {
        spans.push(Span::styled(s, st));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Modifier};

    fn line_text(line: &[Span<'static>]) -> String {
        line.iter().map(|s| s.content.as_ref()).collect()
    }

    fn texts(lines: &[Vec<Span<'static>>]) -> Vec<String> {
        lines.iter().map(|l| line_text(l)).collect()
    }

    #[test]
    fn shorter_than_width_returns_one_line() {
        let spans = vec![Span::raw("hello world")];
        let lines = wrap_spans(spans, 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(line_text(&lines[0]), "hello world");
    }

    #[test]
    fn wraps_on_word_boundaries() {
        let spans = vec![Span::raw("the quick brown fox")];
        // "the quick" is 9 chars, adding " brown" would be 15, still <= 10? let's pick width 10
        let lines = wrap_spans(spans, 10);
        assert_eq!(texts(&lines), vec!["the quick", "brown fox"]);
    }

    #[test]
    fn run_straddling_a_break_keeps_style_on_both_sides() {
        let red = Style::new().fg(Color::Red);
        // "bar baz" is one styled run; width forces a break between "bar" and "baz".
        let spans = vec![
            Span::raw("foo "),
            Span::styled("bar baz", red),
            Span::raw(" qux"),
        ];
        let lines = wrap_spans(spans, 7);
        assert_eq!(texts(&lines), vec!["foo bar", "baz qux"]);

        // "bar" at the end of line 1 keeps the red style.
        let last = lines[0].last().unwrap();
        assert_eq!(last.content.as_ref(), "bar");
        assert_eq!(last.style, red);

        // "baz" at the start of line 2 keeps the red style.
        let first = lines[1].first().unwrap();
        assert_eq!(first.content.as_ref(), "baz");
        assert_eq!(first.style, red);
    }

    #[test]
    fn word_longer_than_width_is_hard_broken() {
        let spans = vec![Span::raw("supercalifragilistic")];
        let lines = wrap_spans(spans, 5);
        for line in &lines {
            let len: usize = line.iter().map(|s| s.content.chars().count()).sum();
            assert!(len <= 5, "line {line:?} exceeds width 5");
        }
        let reconstructed: String = texts(&lines).concat();
        assert_eq!(reconstructed, "supercalifragilistic");
    }

    #[test]
    fn hard_break_mid_word_preserves_style_on_both_sides() {
        let red = Style::new().fg(Color::Red);
        let green = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
        let spans = vec![Span::styled("abc", red), Span::styled("def", green)];
        // "abcdef" is one word, 6 chars, width 4 forces a hard break inside it.
        let lines = wrap_spans(spans, 4);
        assert_eq!(texts(&lines), vec!["abcd", "ef"]);
        assert_eq!(lines[0][0].style, red);
        assert_eq!(lines[0][1].style, green);
        assert_eq!(lines[1][0].style, green);
    }

    #[test]
    fn multi_byte_content_does_not_panic() {
        let spans = vec![Span::raw("日本語のテキストと emoji 🎉🎉🎉 mixed in")];
        for width in 1..20 {
            let lines = wrap_spans(spans.clone(), width);
            assert!(!lines.is_empty());
        }
    }

    #[test]
    fn width_zero_does_not_panic_or_loop_forever() {
        let spans = vec![Span::raw("hello")];
        let lines = wrap_spans(spans, 0);
        assert!(!lines.is_empty());
        let reconstructed: String = texts(&lines).concat();
        assert_eq!(reconstructed, "hello");
    }

    #[test]
    fn empty_input_is_handled() {
        let lines = wrap_spans(Vec::new(), 10);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].is_empty());
    }

    #[test]
    fn empty_span_content_is_handled() {
        let lines = wrap_spans(vec![Span::raw("")], 10);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].is_empty());
    }
}
