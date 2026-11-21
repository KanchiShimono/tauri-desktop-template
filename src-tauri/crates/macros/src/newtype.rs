use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Error;
use syn::Field;
use syn::Fields;
use syn::Meta;
use syn::Token;
use syn::punctuated::Punctuated;

pub(crate) fn field<'a>(input: &'a DeriveInput, derive_name: &str) -> Result<&'a Field, Error> {
    let unsupported = || {
        Error::new_spanned(
            &input.ident,
            format!(
                "`{derive_name}` can only be derived for a non-generic tuple struct with a single field"
            ),
        )
    };

    if !input.generics.params.is_empty() {
        return Err(unsupported());
    }
    let Data::Struct(data) = &input.data else {
        return Err(unsupported());
    };
    let Fields::Unnamed(fields) = &data.fields else {
        return Err(unsupported());
    };
    let mut fields = fields.unnamed.iter();
    let (Some(field), None) = (fields.next(), fields.next()) else {
        return Err(unsupported());
    };

    Ok(field)
}

pub(crate) fn permits_lint(attr: &Attribute, lint_name: &str) -> bool {
    let path = attr.path();
    if path.is_ident("deprecated") {
        return lint_name == "deprecated";
    }
    if !path.is_ident("allow") && !path.is_ident("expect") {
        return false;
    }
    attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .is_ok_and(|lints| lints.iter().any(|lint| lint.path().is_ident(lint_name)))
}

#[cfg(test)]
#[path = "newtype_tests.rs"]
mod tests;
