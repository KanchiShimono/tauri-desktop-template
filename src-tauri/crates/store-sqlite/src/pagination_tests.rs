use app::pagination::ListQuery;

use crate::pagination::LimitOffset;

#[test]
fn the_limit_gains_a_lookahead_row_and_an_unrepresentable_offset_saturates() {
    let query = ListQuery::try_new(1, usize::MAX).expect("valid query");
    let LimitOffset { limit, offset } = query.into();
    assert_eq!((limit, offset), (2, i64::MAX));
}
