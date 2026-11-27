use std::io;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;

use app::error::ErrorInfo;
use app::error::FieldViolation;
use app::error::PublicReason;
use app::error::Violations;
use app::port::PortError;
use app::todo::TodoNotFound;
use serde_json::Value;
use serde_json::json;
use tracing::subscriber;
use tracing_subscriber::fmt::MakeWriter;
use uuid::Uuid;

use super::CommandError;

fn wire(info: &ErrorInfo) -> Value {
    serde_json::to_value(CommandError::from(info)).expect("serializable")
}

#[derive(Clone, Default)]
struct Logs(Arc<Mutex<Vec<u8>>>);

impl Logs {
    fn capture<T>(f: impl FnOnce() -> T) -> (T, String) {
        let logs = Self::default();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(logs.clone())
            .finish();
        let value = subscriber::with_default(subscriber, f);
        let bytes = logs.0.lock().expect("logs lock").clone();
        (value, String::from_utf8(bytes).expect("UTF-8 logs"))
    }
}

impl Write for Logs {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("logs lock").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl MakeWriter<'_> for Logs {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

#[test]
fn invalid_input_lists_every_violation_in_order() {
    let info = ErrorInfo::InvalidInput(Violations {
        first: FieldViolation::new("title", "blank", "Title is blank.".into()),
        rest: vec![FieldViolation::new(
            "description",
            "too_long",
            "Description is too long.".into(),
        )],
    });

    assert_eq!(
        wire(&info),
        json!({
            "kind": "invalid_input",
            "code": "invalid_input",
            "message": "The input is invalid.",
            "violations": [
                { "field": "title", "code": "blank", "message": "Title is blank." },
                { "field": "description", "code": "too_long", "message": "Description is too long." },
            ],
        })
    );
}

#[test]
fn other_failures_carry_their_kind_code_and_message() {
    let reason = |code| PublicReason {
        code,
        message: "Reason.".into(),
    };
    for (info, expected) in [
        (
            ErrorInfo::NotFound(reason("todo_not_found")),
            json!({ "kind": "not_found", "code": "todo_not_found", "message": "Reason." }),
        ),
        (
            ErrorInfo::Conflict(reason("todo_archived")),
            json!({ "kind": "conflict", "code": "todo_archived", "message": "Reason." }),
        ),
        (
            ErrorInfo::Unavailable,
            json!({
                "kind": "unavailable",
                "code": "temporarily_unavailable",
                "message": "The service is temporarily unavailable.",
            }),
        ),
        (
            ErrorInfo::Internal,
            json!({ "kind": "internal", "code": "internal", "message": "An internal error occurred." }),
        ),
    ] {
        assert_eq!(wire(&info), expected);
    }
}

#[test]
fn an_unexpected_failure_is_logged_with_its_cause_and_reported_without_it() {
    for (error, level, cause, info) in [
        (
            PortError::Backend("disk I/O error".into()),
            "ERROR",
            "disk I/O error",
            ErrorInfo::Internal,
        ),
        (
            PortError::Busy("database is locked".into()),
            "WARN",
            "database is locked",
            ErrorInfo::Unavailable,
        ),
    ] {
        let (reported, logs) = Logs::capture(|| super::report(error));

        assert_eq!(reported, (&info).into());
        let reported = serde_json::to_string(&reported).expect("serializable");
        assert!(!reported.contains(cause), "{reported}");
        assert!(logs.contains(level) && logs.contains(cause), "{logs}");
    }
}

#[test]
fn an_expected_failure_is_not_logged() {
    let error = TodoNotFound(Uuid::nil().into());

    let (reported, logs) = Logs::capture(|| super::report(error));

    assert!(matches!(reported, CommandError::NotFound { .. }));
    assert_eq!(logs, "");
}
