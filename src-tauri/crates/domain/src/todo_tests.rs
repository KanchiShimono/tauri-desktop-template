use jiff::SignedDuration;
use jiff::Timestamp;
use uuid::Uuid;

use crate::assignee;
use crate::assignee::Assignee;
use crate::assignee::AssigneeId;
use crate::todo;
use crate::todo::CompletedAt;
use crate::todo::Done;
use crate::todo::DueAt;
use crate::todo::InProgress;
use crate::todo::NotStarted;
use crate::todo::StartedAt;
use crate::todo::Status;
use crate::todo::StatusKind;
use crate::todo::Todo;
use crate::todo::TodoByLifecycle;
use crate::todo::TodoByStatus;
use crate::todo::TodoByUndeleted;
use crate::todo::TodoDescription;
use crate::todo::TodoDescriptionError;
use crate::todo::TodoId;
use crate::todo::TodoTitle;
use crate::todo::TodoTitleError;
use crate::todo::Undeleted;

#[test]
fn title_trims_only_surrounding_whitespace() {
    let title = "\u{2003}Review  invoice\t"
        .parse::<TodoTitle>()
        .expect("title should be valid");

    assert_eq!(title.as_ref(), "Review  invoice");
}

#[test]
fn title_rejects_blank_text() {
    assert_eq!(
        "\u{2003}\t\n".parse::<TodoTitle>(),
        Err(TodoTitleError::Blank)
    );
}

#[test]
fn title_reports_the_forbidden_line_break_or_control_character() {
    for ch in ['\n', '\0', '\u{2028}', '\u{2029}'] {
        assert_eq!(
            format!("a{ch}b").parse::<TodoTitle>(),
            Err(TodoTitleError::LineBreakOrControlCharacter(ch))
        );
    }
}

#[test]
fn title_limit_counts_unicode_scalars_after_trimming() {
    let title = "e\u{0301}".repeat(50);

    assert!(format!("\u{2003}{title}\t").parse::<TodoTitle>().is_ok());
    assert_eq!(
        format!("{title}x").parse::<TodoTitle>(),
        Err(TodoTitleError::TooLong)
    );
}

#[test]
fn description_preserves_formatting_and_normalizes_line_endings() {
    let description = "\r\n  Prepare\r\tRun\u{0085}\u{2028}Review\u{2029}  \r\n"
        .parse::<TodoDescription>()
        .expect("description should be valid");

    assert_eq!(description.as_ref(), "\n  Prepare\n\tRun\n\nReview\n  \n");
}

#[test]
fn description_rejects_blank_text_and_forbidden_control_characters() {
    for (input, error) in [
        ("\u{2003}\t\n", TodoDescriptionError::Blank),
        ("a\0b", TodoDescriptionError::InvalidControlCharacter),
        ("a\u{000b}b", TodoDescriptionError::InvalidControlCharacter),
    ] {
        assert_eq!(input.parse::<TodoDescription>(), Err(error));
    }
}

#[test]
fn description_limit_counts_unicode_scalars_after_normalization() {
    let body = "e\u{0301}".repeat(999);
    let at_limit = format!("\t{body}\r\n");

    assert!(at_limit.parse::<TodoDescription>().is_ok());
    assert_eq!(
        format!("{at_limit} ").parse::<TodoDescription>(),
        Err(TodoDescriptionError::TooLong)
    );
}

#[test]
fn optional_description_is_absent_only_when_missing_or_blank() {
    for input in [None, Some("\u{2003}\t\n")] {
        assert_eq!(TodoDescription::try_from_optional(input), Ok(None));
    }

    let present = "Check the totals";
    assert_eq!(
        TodoDescription::try_from_optional(Some(present)),
        present.parse().map(Some)
    );
    assert_eq!(
        TodoDescription::try_from_optional(Some("a\u{000b}b")),
        Err(TodoDescriptionError::InvalidControlCharacter)
    );
}

fn at(hours: i64) -> Timestamp {
    Timestamp::UNIX_EPOCH + SignedDuration::from_hours(hours)
}

fn new_todo() -> Todo<todo::Active, NotStarted> {
    Todo::new(
        TodoId::from(Uuid::from_u128(1)),
        "Review invoice".parse().expect("title should be valid"),
        None,
        None,
        None,
        at(0),
    )
}

fn new_assignee(id: u128) -> Assignee<assignee::Active> {
    Assignee::new(
        AssigneeId::from(Uuid::from_u128(id)),
        "Ada".parse().expect("name should be valid"),
        at(0),
    )
}

#[test]
fn todo_timestamps_round_trip_one_nanosecond_before_the_epoch() {
    let instant = Timestamp::UNIX_EPOCH - SignedDuration::from_nanos(1);

    assert_eq!(Timestamp::from(StartedAt::from(instant)), instant);
    assert_eq!(Timestamp::from(CompletedAt::from(instant)), instant);
    assert_eq!(Timestamp::from(DueAt::from(instant)), instant);
}

#[test]
fn new_todo_keeps_the_given_values() {
    let id = TodoId::from(Uuid::from_u128(1));
    let assignee_id = AssigneeId::from(Uuid::from_u128(2));
    let created_at = at(0);
    let due_at = DueAt::from(at(25));

    let todo = Todo::new(
        id,
        "Review invoice".parse().expect("title should be valid"),
        Some(
            "Check the totals"
                .parse()
                .expect("description should be valid"),
        ),
        Some(assignee_id),
        Some(due_at),
        created_at,
    );

    assert_eq!(todo.id(), id);
    assert_eq!(todo.title().as_ref(), "Review invoice");
    assert_eq!(
        todo.description().map(AsRef::as_ref),
        Some("Check the totals")
    );
    assert_eq!(todo.assignee_id(), Some(assignee_id));
    assert_eq!(todo.due_at(), Some(due_at));
    assert_eq!(todo.created_at(), created_at);
}

#[test]
fn an_unassigned_todo_can_be_assigned_reassigned_and_cleared() {
    let todo = new_todo();
    assert_eq!(todo.assignee_id(), None);

    let unassigned = todo.change_due_at(Some(at(4).into()));
    let assignee = new_assignee(2);
    let assigned = unassigned.clone().reassign(&assignee);
    assert_eq!(assigned.assignee_id(), Some(assignee.id()));

    let successor = new_assignee(3);
    let reassigned = assigned.reassign(&successor);
    assert_eq!(reassigned.assignee_id(), Some(successor.id()));
    assert_eq!(
        reassigned
            .clone()
            .unassign_for_deleted_assignee(&assignee.delete(at(2))),
        reassigned
    );

    let cleared = reassigned.unassign();
    assert_eq!(cleared, unassigned);
    assert_eq!(cleared.unassign(), unassigned);
}

#[test]
fn completing_a_todo_keeps_its_corrected_start_and_deadline() {
    let due_at = DueAt::from(at(3));
    let started_at = StartedAt::from(at(1));
    let corrected_start = StartedAt::from(at(1) + SignedDuration::from_nanos(1));
    let completed_at = CompletedAt::from(at(2));
    let todo = new_todo()
        .reassign(&new_assignee(2))
        .change_due_at(Some(due_at));
    let in_progress = todo.clone().start(started_at);
    assert_eq!(in_progress.status().started_at(), started_at);
    assert_eq!(in_progress.due_at(), Some(due_at));

    let corrected = in_progress.change_started_at(corrected_start);
    assert_eq!(corrected, todo.start(corrected_start));
    let done = corrected.complete(completed_at);
    assert_eq!(done.status().started_at(), corrected_start);
    assert_eq!(done.status().completed_at(), completed_at);
    assert_eq!(done.due_at(), Some(due_at));
}

#[test]
fn editing_either_time_on_a_completed_todo_preserves_the_other() {
    let todo = new_todo()
        .change_description(Some("Check the totals".parse().expect("valid description")))
        .reassign(&new_assignee(2))
        .change_due_at(Some(at(9).into()));
    let started_at = StartedAt::from(at(3));
    let completed_at = CompletedAt::from(at(5));
    let done = todo.clone().start(started_at).complete(completed_at);

    let earlier_completion = CompletedAt::from(at(2));
    assert_eq!(
        done.clone().change_completed_at(earlier_completion),
        todo.clone().start(started_at).complete(earlier_completion)
    );

    let later_start = StartedAt::from(at(6));
    let corrected = done.change_started_at(later_start);
    let expected_in_progress = todo.start(later_start);
    assert_eq!(
        corrected,
        expected_in_progress.clone().complete(completed_at)
    );

    let corrected_times = corrected.status();
    let same_column = corrected.change_status(StatusKind::Done, at(7));
    assert_eq!(same_column.status(), Status::Done(corrected_times));
    let reopened = same_column.change_status(StatusKind::InProgress, at(8));
    assert_eq!(reopened.into_any(), expected_in_progress.into_any());
}

#[test]
fn every_column_move_preserves_the_todo_and_records_only_the_expected_times() {
    let todo = new_todo()
        .change_description(Some("Check the totals".parse().expect("valid description")))
        .change_due_at(Some(at(8).into()))
        .reassign(&new_assignee(2));
    let TodoByLifecycle::Active(original) = TodoByLifecycle::from(todo.into_any()) else {
        panic!("new TODO should be active");
    };
    let mut todo = original.clone();
    let ready = Status::NotStarted(NotStarted);
    let working = |hour| {
        Status::InProgress(InProgress {
            started_at: at(hour).into(),
        })
    };
    let done = |start, end| {
        Status::Done(Done {
            started_at: at(start).into(),
            completed_at: at(end).into(),
        })
    };

    for (target, hour, expected) in [
        (StatusKind::NotStarted, 1, ready),
        (StatusKind::Done, 2, done(2, 2)),
        (StatusKind::Done, 3, done(2, 2)),
        (StatusKind::InProgress, 4, working(2)),
        (StatusKind::InProgress, 5, working(2)),
        (StatusKind::NotStarted, 6, ready),
        (StatusKind::InProgress, 8, working(8)),
        (StatusKind::Done, 7, done(8, 7)),
        (StatusKind::NotStarted, 9, ready),
    ] {
        let from = todo.status();
        todo = todo.change_status(target, at(hour));
        assert_eq!(
            StatusKind::from(todo.status()),
            target,
            "{from:?} -> {target:?}"
        );
        assert_eq!(
            todo,
            Todo {
                status: expected,
                ..original.clone()
            },
            "{from:?} -> {target:?}"
        );
    }
}

#[test]
fn archiving_records_the_time_and_unarchiving_restores_the_todo() {
    let todo = new_todo()
        .start(at(1).into())
        .change_due_at(Some(at(3).into()));

    let archived = todo.clone().archive(at(2));
    assert_eq!(archived.lifecycle().archived_at(), at(2));

    assert_eq!(archived.unarchive(), todo);
}

#[test]
fn deleting_records_the_time_and_restoring_returns_the_todo_to_its_previous_lifecycle() {
    let assignee = new_assignee(2);
    let active = new_todo().reassign(&assignee).start(at(1).into());
    let archived = active.clone().archive(at(2));

    let deleted = active.clone().delete(at(3));
    assert_eq!(deleted.lifecycle().deleted_at(), at(3));
    assert_eq!(deleted.restore(), active);

    let deleted = archived.clone().delete(at(3));
    assert_eq!(deleted.lifecycle().previous(), archived.lifecycle());
    assert_eq!(
        deleted.clone().into_any().assignee_id(),
        Some(assignee.id())
    );
    let restored = deleted.restore();
    assert_eq!(restored, archived);
    assert_eq!(restored.unarchive(), active);
}

#[test]
fn deleting_an_assignee_clears_the_assignment_in_every_lifecycle_and_after_restoration() {
    let assignee = new_assignee(2);
    let unassigned = new_todo()
        .change_due_at(Some(at(8).into()))
        .start(at(1).into())
        .complete(at(2).into());
    let assigned = unassigned.clone().reassign(&assignee);
    let deleted_assignee = assignee.delete(at(6));
    let cleared = assigned
        .clone()
        .unassign_for_deleted_assignee(&deleted_assignee);
    assert_eq!(cleared, unassigned);
    assert_eq!(
        cleared.unassign_for_deleted_assignee(&deleted_assignee),
        unassigned
    );

    let archived = assigned.clone().archive(at(3));
    let unassigned_archived = unassigned.clone().archive(at(3));
    assert_eq!(
        archived
            .clone()
            .unassign_for_deleted_assignee(&deleted_assignee),
        unassigned_archived
    );
    assert_eq!(
        assigned
            .delete(at(4))
            .unassign_for_deleted_assignee(&deleted_assignee),
        unassigned.delete(at(4))
    );

    let cleared = archived
        .delete(at(4))
        .unassign_for_deleted_assignee(&deleted_assignee);
    assert_eq!(cleared.lifecycle().deleted_at(), at(4));
    assert_eq!(cleared.restore(), unassigned_archived);
}

#[test]
fn editing_replaces_the_targeted_fields() {
    let assignee = new_assignee(3);
    let due_at = DueAt::from(at(7));

    let todo = new_todo().reassign(&assignee);
    assert_eq!(todo.due_at(), None);

    let todo = todo
        .rename("Pay invoice".parse().expect("title should be valid"))
        .change_description(Some(
            "Before Friday"
                .parse()
                .expect("description should be valid"),
        ))
        .change_due_at(Some(due_at));

    assert_eq!(todo.title().as_ref(), "Pay invoice");
    assert_eq!(todo.description().map(AsRef::as_ref), Some("Before Friday"));
    assert_eq!(todo.assignee_id(), Some(assignee.id()));
    assert_eq!(todo.due_at(), Some(due_at));

    let todo = todo.change_description(None);
    assert_eq!(todo.description(), None);
    assert_eq!(todo.due_at(), Some(due_at));
    let done = todo.start(at(1).into()).complete(at(2).into());
    let recorded_times = done.status();
    let rescheduled_due_at = DueAt::from(Timestamp::from(due_at) + SignedDuration::from_millis(1));
    let rescheduled = done.change_due_at(Some(rescheduled_due_at));
    assert_eq!(rescheduled.due_at(), Some(rescheduled_due_at));
    assert_eq!(rescheduled.status(), recorded_times);
    let cleared = rescheduled.change_due_at(None);
    assert_eq!(cleared.due_at(), None);
    assert_eq!(cleared.status(), recorded_times);
}

#[test]
fn undeleted_todos_widen_and_narrow_without_losing_their_status_or_archive_time() {
    let active = new_todo().change_status(StatusKind::InProgress, at(1));
    let archived = active.clone().archive(at(2));

    let stored = Todo::<Undeleted, Status>::from(active.clone());
    assert_eq!(
        TodoByUndeleted::from(stored),
        TodoByUndeleted::Active(active)
    );
    let stored = Todo::<Undeleted, Status>::from(archived.clone());
    assert_eq!(
        TodoByUndeleted::from(stored),
        TodoByUndeleted::Archived(archived)
    );
}

#[test]
fn narrowing_by_lifecycle_picks_the_matching_state_and_loses_nothing() {
    let active = new_todo().into_any();
    let TodoByLifecycle::Active(narrowed) = TodoByLifecycle::from(active.clone()) else {
        panic!("todo should be active");
    };
    assert_eq!(narrowed.into_any(), active);

    let archived = new_todo().archive(at(1)).into_any();
    let TodoByLifecycle::Archived(narrowed) = TodoByLifecycle::from(archived.clone()) else {
        panic!("todo should be archived");
    };
    assert_eq!(narrowed.into_any(), archived);

    for (deleted, previous) in [
        (new_todo().delete(at(2)).into_any(), active),
        (new_todo().archive(at(1)).delete(at(2)).into_any(), archived),
    ] {
        let TodoByLifecycle::Deleted(narrowed) = TodoByLifecycle::from(deleted.clone()) else {
            panic!("todo should be deleted");
        };
        assert_eq!(narrowed.clone().into_any(), deleted);
        assert_eq!(narrowed.restore().into_any(), previous);
    }
}

#[test]
fn narrowing_by_status_picks_the_matching_state_and_loses_nothing() {
    let not_started = new_todo().into_any();
    let TodoByStatus::NotStarted(narrowed) = TodoByStatus::from(not_started.clone()) else {
        panic!("todo should not be started");
    };
    assert_eq!(narrowed.into_any(), not_started);

    let in_progress = new_todo().start(at(1).into()).into_any();
    let TodoByStatus::InProgress(narrowed) = TodoByStatus::from(in_progress.clone()) else {
        panic!("todo should be in progress");
    };
    assert_eq!(narrowed.into_any(), in_progress);

    let done = new_todo()
        .start(at(1).into())
        .complete(at(2).into())
        .into_any();
    let TodoByStatus::Done(narrowed) = TodoByStatus::from(done.clone()) else {
        panic!("todo should be done");
    };
    assert_eq!(narrowed.into_any(), done);
}

#[test]
fn narrowing_by_lifecycle_and_status_in_either_order_recovers_the_original_todo() {
    let original = new_todo().start(at(1).into()).archive(at(2));
    let any = original.clone().into_any();

    let TodoByLifecycle::Archived(archived) = TodoByLifecycle::from(any.clone()) else {
        panic!("todo should be archived");
    };
    let TodoByStatus::InProgress(narrowed) = TodoByStatus::from(archived) else {
        panic!("todo should be in progress");
    };
    assert_eq!(narrowed, original);

    let TodoByStatus::InProgress(in_progress) = TodoByStatus::from(any) else {
        panic!("todo should be in progress");
    };
    let TodoByLifecycle::Archived(narrowed) = TodoByLifecycle::from(in_progress) else {
        panic!("todo should be archived");
    };
    assert_eq!(narrowed, original);
}
