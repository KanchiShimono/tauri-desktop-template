use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use syn::Type;

pub(crate) fn expand(name: &Ident, inner: &Type) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl ::core::convert::From<#inner> for #name {
            fn from(__value: #inner) -> Self {
                Self(__value)
            }
        }
    }
}
