use domain::assignee::AssigneeNameError;

use crate::assignee;
use crate::error::ErrorInfo;

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
            assignee::describe_invalid_name(&error),
            ErrorInfo::InvalidInput(violations)
                if violations.first.field == "name" && violations.first.reason.code == code
        ));
    }
}
