#![forbid(deprecated)]

use std::num::ParseIntError;

use macros::Id;

#[derive(Id)]
struct RowId(u32);

#[test]
fn converts_from_and_into_the_inner_value() {
    assert_eq!(u32::from(RowId::from(7)), 7);
}

#[test]
fn parses_like_the_inner_type() {
    for text in ["7", "seven"] {
        assert_eq!(text.parse::<RowId>().map(u32::from), text.parse::<u32>());
    }
}

mod public_id {
    use macros::Id;

    #[derive(Id)]
    struct Inner(u32);

    #[derive(Id)]
    pub struct PublicId(Inner);
}

#[test]
fn public_id_preserves_the_parse_error_of_its_private_inner_type() {
    use public_id::PublicId;

    for text in ["7", "seven"] {
        let parsed: Result<PublicId, ParseIntError> = text.parse();
        assert_eq!(
            parsed.map(|id| id.to_string()),
            text.parse::<u32>().map(|value| value.to_string())
        );
    }
}

#[test]
fn the_generated_error_parameter_does_not_shadow_field_types() {
    type __IdParseError = u32;
    macro_rules! number {
        () => {
            __IdParseError
        };
    }

    #[derive(Id)]
    struct Direct(__IdParseError);
    #[derive(Id)]
    struct FromMacro(number!());

    assert_eq!("7".parse::<Direct>().map(u32::from), Ok(7));
    assert_eq!("7".parse::<FromMacro>().map(u32::from), Ok(7));
}

#[test]
fn display_honors_format_specs_like_the_inner_value() {
    assert_eq!(format!("{:>3}", RowId::from(7)), "  7");
}

#[test]
fn derives_with_common_parameter_names_in_scope() {
    const VALUE: u32 = 7;
    const TEXT: &str = "7";
    use TEXT as formatter;
    use VALUE as value;

    #[derive(Id)]
    struct Number(u32);

    assert_eq!(Number::from(value).to_string(), formatter);
    assert_eq!(formatter.parse::<Number>().map(u32::from), Ok(value));
}
