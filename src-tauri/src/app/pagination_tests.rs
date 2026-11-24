use super::ListError;
use super::ListQuery;
use super::Page;
use super::UnvalidatedListQuery;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;

#[test]
fn list_query_accepts_its_valid_boundaries() {
    assert_eq!(ListQuery::try_new(1, 0).map(ListQuery::limit), Ok(1));
    assert_eq!(
        ListQuery::try_new(ListQuery::MAX_LIMIT, usize::MAX).map(ListQuery::offset),
        Ok(usize::MAX)
    );
}

#[test]
fn an_invalid_limit_is_reported_as_a_violation_of_the_limit() {
    for (limit, code) in [(0, "too_small"), (ListQuery::MAX_LIMIT + 1, "too_large")] {
        let error = ListQuery::try_from(UnvalidatedListQuery { limit, offset: 0 })
            .expect_err("invalid limit");
        assert!(matches!(
            ListError::from(error).describe(),
            ErrorInfo::InvalidInput(violations)
                if violations.first.field == "limit" && violations.first.reason.code == code
        ));
    }
}

#[test]
fn a_storage_failure_is_described_by_the_port() {
    for (port, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(ListError::from(port).describe(), expected);
    }
}

#[test]
fn only_the_lookahead_row_indicates_a_next_page() {
    let query = ListQuery::try_new(2, 7).expect("valid query");
    let rows = [3, 1, 2];
    let cases = [(0, 0, None), (1, 1, None), (2, 2, None), (3, 2, Some(9))];

    for (fetched, kept, next_offset) in cases {
        let page = Page::from_lookahead(query, rows[..fetched].to_vec()).expect("valid page");
        assert_eq!(page.items(), &rows[..kept], "fetched {fetched} rows");
        assert_eq!(page.next_offset(), next_offset, "fetched {fetched} rows");
    }
}

#[test]
fn usize_max_is_a_valid_continuation_and_final_offset() {
    let query = ListQuery::try_new(1, usize::MAX - 1).expect("valid query");
    let page = Page::from_lookahead(query, vec![1, 2]).expect("offset fits exactly");
    assert_eq!(page.next_offset(), Some(usize::MAX));

    let query = ListQuery::try_new(1, usize::MAX).expect("valid query");
    let last = Page::from_lookahead(query, vec![2]).expect("final page needs no addition");
    assert_eq!(last.items(), [2]);
    assert_eq!(last.next_offset(), None);
}
