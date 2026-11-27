use std::error::Error;

use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::error::FieldViolation;
use crate::error::PublicReason;
use crate::error::Violations;
use crate::pagination::ListQuery;
use crate::pagination::Page;
use crate::port::PortError;

#[test]
fn resource_and_conflict_errors_expose_their_specific_reason() {
    for (info, code, message) in [
        (
            ErrorInfo::NotFound(PublicReason {
                code: "todo_not_found",
                message: "The Todo was not found.".into(),
            }),
            "todo_not_found",
            "The Todo was not found.",
        ),
        (
            ErrorInfo::Conflict(PublicReason {
                code: "todo_archived",
                message: "The Todo is archived.".into(),
            }),
            "todo_archived",
            "The Todo is archived.",
        ),
    ] {
        assert_eq!(info.code(), code);
        assert_eq!(info.message(), message);
    }
}

#[test]
fn a_continuation_overflow_is_internal_and_retains_its_typed_cause() {
    let query = ListQuery::try_new(2, usize::MAX - 1).expect("valid query");
    let overflow = Page::from_lookahead(query, vec![3, 1, 2]).expect_err("offset exceeds usize");
    let error = PortError::from(overflow);

    assert_eq!(error.describe(), ErrorInfo::Internal);
    assert_eq!(
        error.source().and_then(|source| source.downcast_ref()),
        Some(&overflow)
    );
}

#[test]
fn a_storage_failure_is_unavailable_when_busy_and_internal_otherwise() {
    for (error, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(error.describe(), expected);
    }
}

#[test]
fn errors_without_a_specific_reason_expose_a_fixed_code_and_message() {
    let invalid = ErrorInfo::InvalidInput(Violations::one(FieldViolation::new(
        "title",
        "blank",
        "Title must contain a non-whitespace character.".into(),
    )));
    for (info, code) in [
        (invalid, "invalid_input"),
        (ErrorInfo::Unavailable, "temporarily_unavailable"),
        (ErrorInfo::Internal, "internal"),
    ] {
        assert_eq!(info.code(), code);
        assert!(!info.message().is_empty());
    }
}
