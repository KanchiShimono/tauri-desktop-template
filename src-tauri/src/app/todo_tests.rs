use super::TodoFieldError;
use crate::domain::todo::TodoDescriptionError;
use crate::domain::todo::TodoTitleError;

#[test]
fn field_errors_name_their_field_and_public_code() {
    for (error, field, code) in [
        (
            TodoFieldError::Title(TodoTitleError::Blank),
            "title",
            "blank",
        ),
        (
            TodoFieldError::Title(TodoTitleError::LineBreakOrControlCharacter('\n')),
            "title",
            "invalid_character",
        ),
        (
            TodoFieldError::Title(TodoTitleError::TooLong),
            "title",
            "too_long",
        ),
        (
            TodoFieldError::Description(TodoDescriptionError::Blank),
            "description",
            "blank",
        ),
        (
            TodoFieldError::Description(TodoDescriptionError::InvalidControlCharacter),
            "description",
            "invalid_character",
        ),
        (
            TodoFieldError::Description(TodoDescriptionError::TooLong),
            "description",
            "too_long",
        ),
    ] {
        let violation = error.describe();
        assert_eq!((violation.field, violation.reason.code), (field, code));
    }
}
