use macros::From;
use macros::Id;
use macros::Into;

#[deprecated]
#[derive(Id)]
struct LegacyId(u32);

#[deprecated]
type LegacyNumber = u32;

#[allow(deprecated, reason = "This wrapper retains the legacy representation.")]
#[derive(From, Into)]
struct Number(LegacyNumber);

#[derive(Id)]
struct RowId(
    #[expect(deprecated, reason = "This ID retains the legacy representation.")] LegacyNumber,
);

#[test]
#[allow(
    deprecated,
    reason = "Exercise the deprecated ID's generated conversions."
)]
fn converts_a_deprecated_wrapper() {
    assert_eq!(u32::from(LegacyId::from(7)), 7);
}

#[test]
fn converts_wrappers_of_a_deprecated_type() {
    assert_eq!(u32::from(Number::from(5)), 5);
    assert_eq!("7".parse::<RowId>().map(u32::from), Ok(7));
}

#[test]
fn honors_the_warnings_group_on_the_wrapper_and_field() {
    #[allow(warnings, reason = "This wrapper retains the legacy representation.")]
    #[derive(Into)]
    struct Wrapper(LegacyNumber);

    #[derive(From)]
    struct Field(#[expect(warnings, reason = "This field uses the legacy type.")] LegacyNumber);

    assert_eq!(u32::from(Wrapper(5)), 5);
    assert_eq!(Field::from(7).0, 7);
}

#[test]
#[forbid(deprecated)]
fn allowing_warnings_respects_a_forbidden_deprecated_lint() {
    #[allow(
        warnings,
        reason = "Exercise the warnings group inside a forbid scope."
    )]
    #[derive(Id)]
    struct Number(u32);

    assert_eq!("7".parse::<Number>().map(u32::from), Ok(7));
}
