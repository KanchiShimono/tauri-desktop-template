use crate::app::error::ErrorInfo;
use crate::domain::assignee::AssigneeNameError;

#[test]
fn name_errors_name_their_field_and_public_code() {
    for (error, code) in [
        (AssigneeNameError::Blank, "blank"),
        (
            AssigneeNameError::LineBreakOrControlCharacter('\n'),
            "invalid_character",
        ),
        (AssigneeNameError::TooLong, "too_long"),
    ] {
        assert!(matches!(
            super::describe_invalid_name(&error),
            ErrorInfo::InvalidInput(violations)
                if violations.first.field == "name" && violations.first.reason.code == code
        ));
    }
}
