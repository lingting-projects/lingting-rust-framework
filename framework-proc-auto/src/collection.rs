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
    definition: TokenStream,
    specta_enabled: bool,
) -> TokenStream {
    let type_name_fn = format_ident!("__proc_auto_type_name_{ident}");
    let definition_fn = format_ident!("__proc_auto_definition_{ident}");
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
    let ts_metadata = if specta_enabled {
        quote!(::framework_proc_core::TsTypeMetadata::register(#register_fn))
    } else {
        quote!(::framework_proc_core::TsTypeMetadata::none())
    };

    quote! {
        #[doc(hidden)]
        fn #type_name_fn() -> &'static str {
            #metadata_type_name
        }

        #[doc(hidden)]
        fn #definition_fn() -> &'static str {
            stringify!(#definition)
        }

        #registration

        ::framework_proc_core::push_type_metadata! {
            ::framework_proc_core::TypeMetadata::new(#kind, #type_name_fn)
                .with_ts(#ts_metadata)
                .with_rust(
                    ::framework_proc_core::RustTypeMetadata::new()
                        .with_derives(&[#(#derives),*])
                        .with_attributes(&[#(#attributes),*])
                        .with_definition(#definition_fn)
                )
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
    _: TokenStream,
    _: bool,
) -> TokenStream {
    TokenStream::new()
}
