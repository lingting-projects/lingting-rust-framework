use proc_macro2::TokenStream;

#[cfg(feature = "collect")]
use quote::{format_ident, quote};

#[cfg(feature = "collect")]
pub fn expand(
    ident: &syn::Ident,
    generics: &syn::Generics,
    kind: TokenStream,
    derives: Vec<syn::LitStr>,
    attributes: Vec<syn::LitStr>,
    specta_enabled: bool,
) -> TokenStream {
    let type_name_fn = format_ident!("__proc_auto_type_name_{ident}");
    let register_fn = format_ident!("__proc_auto_register_{ident}");
    let type_arguments = generics.params.iter().map(|_| quote!(()));
    let registered_type = if generics.params.is_empty() {
        quote!(#ident)
    } else {
        quote!(#ident<#(#type_arguments),*>)
    };
    let metadata_type_name = if generics.params.is_empty() {
        quote!(::std::any::type_name::<#ident>())
    } else {
        quote!(concat!(module_path!(), "::", stringify!(#ident)))
    };
    let registration = if specta_enabled {
        quote! {
            #[doc(hidden)]
            fn #register_fn(types: ::specta::Types) -> ::specta::Types {
                types.register::<#registered_type>()
            }
        }
    } else {
        TokenStream::new()
    };
    let register_value = if specta_enabled {
        quote!(Some(#register_fn))
    } else {
        quote!(None)
    };

    quote! {
        #[doc(hidden)]
        fn #type_name_fn() -> &'static str {
            #metadata_type_name
        }

        #registration

        ::framework_proc_core::push_type_metadata! {
            ::framework_proc_core::TypeMetadata {
                kind: #kind,
                type_name: #type_name_fn,
                derives: &[#(#derives),*],
                attributes: &[#(#attributes),*],
                register: #register_value,
            }
        }
    }
}

#[cfg(not(feature = "collect"))]
pub fn expand(
    _: &syn::Ident,
    _: &syn::Generics,
    _: TokenStream,
    _: Vec<syn::LitStr>,
    _: Vec<syn::LitStr>,
    _: bool,
) -> TokenStream {
    TokenStream::new()
}
