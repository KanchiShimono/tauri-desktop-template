use syn::DeriveInput;

#[test]
fn rejects_unsupported_types_with_the_requested_derive_name() {
    let inputs: [DeriveInput; 9] = [
        syn::parse_quote! { struct Generic<T>(T); },
        syn::parse_quote! { struct Borrowed<'a>(&'a str); },
        syn::parse_quote! { struct Buffer<const N: usize>([u8; N]); },
        syn::parse_quote! { enum Choice { Only(u32) } },
        syn::parse_quote! { union Bits { value: u32 } },
        syn::parse_quote! { struct Named { value: u32 } },
        syn::parse_quote! { struct Unit; },
        syn::parse_quote! { struct Empty(); },
        syn::parse_quote! { struct Pair(u32, u32); },
    ];

    for input in inputs {
        for derive_name in ["From", "Id", "Into"] {
            let Err(error) = super::field(&input, derive_name) else {
                panic!("{derive_name} should reject {}", input.ident);
            };
            assert_eq!(
                error.to_string(),
                format!(
                    "`{derive_name}` can only be derived for a non-generic tuple struct with a single field"
                )
            );
        }
    }
}
