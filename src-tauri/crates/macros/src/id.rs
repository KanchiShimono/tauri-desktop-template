use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use syn::Type;

use crate::from;
use crate::into;

pub(crate) fn expand(name: &Ident, inner: &Type) -> TokenStream {
    let from_impl = from::expand(name, inner);
    let (into_impl, target) = into::expand_with_target(name, inner);
    // Keep the inferred error parameter from shadowing the wrapper or its field type.
    // The Into target already resolves field macros outside this generic impl.
    let type_names = quote!(#name #target).to_string();
    let mut error_name = String::from("__IdParseError");
    while type_names.contains(&error_name) {
        error_name.push('_');
    }
    let parse_error = Ident::new(&error_name, Span::mixed_site());

    quote! {
        #from_impl
        #into_impl

        // Expose the error without naming a private field type in the associated type.
        #[automatically_derived]
        impl<#parse_error> ::core::str::FromStr for #name
        where
            #target: ::core::str::FromStr<Err = #parse_error>,
        {
            type Err = #parse_error;

            fn from_str(__value: &str) -> ::core::result::Result<Self, Self::Err> {
                ::core::str::FromStr::from_str(__value).map(Self)
            }
        }

        #[automatically_derived]
        impl ::core::fmt::Display for #name {
            fn fmt(&self, __formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.0, __formatter)
            }
        }
    }
}
