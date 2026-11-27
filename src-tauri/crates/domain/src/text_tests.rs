use crate::text::NonEmptyText;
use crate::text::NonEmptyTextError;

#[test]
fn non_empty_text_preserves_whitespace() {
    for input in ["  valid text\u{00a0}\t", " \t\n"] {
        let text = input.parse::<NonEmptyText>().expect("text should be valid");

        assert_eq!(text.as_ref(), input);
        assert_eq!(String::from(text), input);
    }
}

#[test]
fn non_empty_text_rejects_empty_text() {
    assert_eq!("".parse::<NonEmptyText>(), Err(NonEmptyTextError::Empty));
}

#[test]
fn non_empty_text_rejects_nul() {
    assert_eq!(
        "before\0after".parse::<NonEmptyText>(),
        Err(NonEmptyTextError::ContainsNul)
    );
}
