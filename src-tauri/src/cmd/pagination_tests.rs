use super::NextOffsetOutOfRange;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;

#[test]
fn a_next_offset_beyond_the_wire_format_is_an_internal_error() {
    let error = u32::try_from(u64::MAX)
        .map_err(NextOffsetOutOfRange::from)
        .expect_err("out of range");

    assert_eq!(error.describe(), ErrorInfo::Internal);
}
