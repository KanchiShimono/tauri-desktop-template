use thiserror::Error;
use tokio::runtime::Builder;

use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::port::PortError;
use crate::port::UnitOfWork;

#[derive(Debug)]
enum Action {
    Commit,
    Rollback,
}

struct TestUnitOfWork<'a> {
    called: &'a mut Option<Action>,
    result: Result<(), PortError>,
}

impl UnitOfWork for TestUnitOfWork<'_> {
    async fn commit(self) -> Result<(), PortError> {
        *self.called = Some(Action::Commit);
        self.result
    }

    async fn rollback(self) -> Result<(), PortError> {
        *self.called = Some(Action::Rollback);
        self.result
    }
}

#[derive(Debug, Error)]
enum OperationError {
    #[error("TODO is archived")]
    Archived,
    #[error(transparent)]
    Port(#[from] PortError),
}

fn finalize(
    operation: Result<u8, OperationError>,
    result: Result<(), PortError>,
) -> (Option<Action>, Result<u8, OperationError>) {
    let mut called = None;
    let work = TestUnitOfWork {
        called: &mut called,
        result,
    };
    let outcome = Builder::new_current_thread()
        .build()
        .expect("build test runtime")
        .block_on(work.finalize(operation));
    (called, outcome)
}

#[test]
fn successful_work_commits_and_propagates_a_commit_failure() {
    assert!(matches!(
        finalize(Ok(42), Ok(())),
        (Some(Action::Commit), Ok(42))
    ));

    let busy = PortError::Busy("cannot commit".into());
    assert!(matches!(
        finalize(Ok(42), Err(busy)),
        (Some(Action::Commit), Err(OperationError::Port(PortError::Busy(cause))))
            if cause.to_string() == "cannot commit"
    ));
}

#[test]
fn failed_work_rolls_back_and_preserves_both_errors_if_rollback_fails() {
    assert!(matches!(
        finalize(Err(OperationError::Archived), Ok(())),
        (Some(Action::Rollback), Err(OperationError::Archived))
    ));

    let busy = PortError::Busy("cannot roll back".into());
    let outcome = finalize(Err(OperationError::Archived), Err(busy));
    let (Some(Action::Rollback), Err(OperationError::Port(port))) = outcome else {
        panic!("expected a rollback failure, got {outcome:?}");
    };
    assert_eq!(port.describe(), ErrorInfo::Internal);
    let PortError::Backend(failure) = port else {
        panic!("expected a backend error, got {port:?}");
    };
    assert!(
        failure
            .to_string()
            .contains(&OperationError::Archived.to_string())
    );
    let cause = failure
        .source()
        .expect("rollback error should be the source");
    assert!(matches!(
        cause.downcast_ref::<PortError>(),
        Some(PortError::Busy(detail)) if detail.to_string() == "cannot roll back"
    ));
}
