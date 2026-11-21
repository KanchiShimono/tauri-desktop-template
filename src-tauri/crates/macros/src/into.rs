use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::Ident;
use syn::Item;
use syn::Path;
use syn::Type;
use syn::visit_mut;
use syn::visit_mut::VisitMut;

pub(crate) fn expand(name: &Ident, inner: &Type) -> TokenStream {
    expand_with_target(name, inner).0
}

pub(crate) fn expand_with_target(name: &Ident, inner: &Type) -> (TokenStream, TokenStream) {
    let mut resolved = inner.clone();
    let mut resolver = ResolveSelf {
        wrapper: name,
        needs_wrapper_scope: false,
    };
    resolver.visit_type_mut(&mut resolved);

    let (field_impl, target) = if resolver.needs_wrapper_scope {
        // Macros can supply `Self`, and local items can shadow the wrapper's name.
        // In either case let rustc resolve the field type with the wrapper as `Self`.
        let field_trait = format_ident!("__{}NewtypeField", name, span = Span::mixed_site());
        let field_impl = quote! {
            trait #field_trait {
                type Inner;
            }

            #[automatically_derived]
            impl #field_trait for #name {
                type Inner = #inner;
            }
        };
        (Some(field_impl), quote!(<#name as #field_trait>::Inner))
    } else {
        (None, quote!(#resolved))
    };

    let impls = quote! {
        #field_impl

        #[automatically_derived]
        impl ::core::convert::From<#name> for #target {
            fn from(__value: #name) -> Self {
                __value.0
            }
        }
    };
    (impls, target)
}

struct ResolveSelf<'a> {
    wrapper: &'a Ident,
    needs_wrapper_scope: bool,
}

impl VisitMut for ResolveSelf<'_> {
    fn visit_path_mut(&mut self, path: &mut Path) {
        if let Some(first) = path.segments.first_mut()
            && first.ident == "Self"
        {
            first.ident = self.wrapper.clone();
        }
        visit_mut::visit_path_mut(self, path);
    }

    fn visit_item_mut(&mut self, _: &mut Item) {
        self.needs_wrapper_scope = true;
    }

    fn visit_token_stream_mut(&mut self, _: &mut TokenStream) {
        self.needs_wrapper_scope = true;
    }
}
