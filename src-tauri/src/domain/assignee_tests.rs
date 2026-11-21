use jiff::SignedDuration;
use jiff::Timestamp;
use uuid::Uuid;

use super::Active;
use super::Assignee;
use super::AssigneeByLifecycle;
use super::AssigneeId;
use super::AssigneeName;
use super::AssigneeNameError;
use super::Deleted;

#[test]
fn name_trims_only_surrounding_whitespace() {
    let name = "\t\u{2003}Ada  Lovelace\u{00a0}\n"
        .parse::<AssigneeName>()
        .expect("name should be valid");

    assert_eq!(name.as_ref(), "Ada  Lovelace");
}

#[test]
fn name_rejects_blank_text() {
    assert_eq!(
        " \t\u{2003}\n".parse::<AssigneeName>(),
        Err(AssigneeNameError::Blank)
    );
}

#[test]
fn name_rejects_embedded_controls_and_unicode_line_separators() {
    for ch in [
        '\n', '\r', '\t', '\0', '\u{1b}', '\u{85}', '\u{2028}', '\u{2029}',
    ] {
        assert_eq!(
            format!("Ada{ch}Lee").parse::<AssigneeName>(),
            Err(AssigneeNameError::LineBreakOrControlCharacter(ch))
        );
    }
}

#[test]
fn name_limit_counts_unicode_scalars_after_trimming() {
    let name = "e\u{0301}".repeat(25);

    assert!(format!("\u{2003}{name}\t").parse::<AssigneeName>().is_ok());
    assert_eq!(
        format!("{name}x").parse::<AssigneeName>(),
        Err(AssigneeNameError::TooLong)
    );
}

fn new_assignee(created_at: Timestamp) -> Assignee<Active> {
    Assignee::new(
        AssigneeId::from(Uuid::from_u128(42)),
        "Ada Byron".parse().expect("name should be valid"),
        created_at,
    )
}

#[test]
fn new_assignee_is_active_and_renaming_only_changes_the_name() {
    let created_at = Timestamp::UNIX_EPOCH + SignedDuration::from_hours(1);
    let assignee = new_assignee(created_at);
    assert_eq!(assignee.lifecycle(), Active);
    assert_eq!(assignee.name().as_ref(), "Ada Byron");

    let renamed = assignee.rename("Ada Lovelace".parse().expect("name should be valid"));

    assert_eq!(renamed.id(), AssigneeId::from(Uuid::from_u128(42)));
    assert_eq!(renamed.name().as_ref(), "Ada Lovelace");
    assert_eq!(renamed.created_at(), created_at);
}

#[test]
fn deletion_and_restoration_preserve_assignee_details_across_lifecycle_conversions() {
    let created_at = Timestamp::UNIX_EPOCH + SignedDuration::from_hours(2);
    let deleted_at = created_at + SignedDuration::from_hours(1);
    let original = new_assignee(created_at);
    let stored = original.clone().into_any();
    let AssigneeByLifecycle::Active(active) = AssigneeByLifecycle::from(stored) else {
        panic!("assignee should be active");
    };
    assert_eq!(active, original);

    let deleted: Assignee<Deleted> = active.delete(deleted_at);
    let stored = deleted.into_any();
    let AssigneeByLifecycle::Deleted(deleted) = AssigneeByLifecycle::from(stored) else {
        panic!("assignee should be deleted");
    };
    assert_eq!(deleted.id(), original.id());
    assert_eq!(deleted.name(), original.name());
    assert_eq!(deleted.created_at(), created_at);
    assert_eq!(deleted.lifecycle().deleted_at(), deleted_at);
    assert_eq!(deleted.restore(), original);
}
