pub(crate) mod assignee;
pub(crate) mod todo;

fn first_line_break_or_control_character(text: &str) -> Option<char> {
    text.chars()
        .find(|&ch| ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}'))
}
