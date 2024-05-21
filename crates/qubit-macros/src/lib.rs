use handler::{generate_handler, HandlerKind};
use quote::quote;
use syn::{meta, parse_macro_input, spanned::Spanned, Error, Item};

mod handler;

/// See [`qubit::builder::handler`] for more information.
#[proc_macro_attribute]
pub fn handler(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Extract information from the attribute
    let kind = {
        let mut kind = HandlerKind::Query;

        let attribute_parser = meta::parser(|meta| kind.parse(meta));

        parse_macro_input!(attr with attribute_parser);

        kind
    };

    // Attempt to match as a function
    syn::parse::<Item>(input)
        .and_then(|item| {
            if let Item::Fn(handler) = item {
                generate_handler(handler, kind)
            } else {
                Err(Error::new(item.span(), "handlers must be a method"))
            }
        })
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn exported_type(
    _attrs: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let s = syn::parse::<Item>(input).unwrap();

    let (target_struct, fields) = match s {
        Item::Struct(ref s) => (
            s.ident.clone(),
            s.fields.iter().map(|field| field.ty.clone()),
        ),
        _ => unimplemented!(),
    };

    quote! {
        #s

        impl qubit::TypeDependencies for #target_struct {
            fn get_deps(dependencies: &mut std::collections::BTreeMap<std::string::String, std::string::String>) {
                // Short circuit if this type has already been added
                if dependencies.contains_key(&<Self as ts_rs::TS>::name()) {
                    return;
                }

                // Insert this type
                dependencies.insert(<Self as ts_rs::TS>::name(), <Self as ts_rs::TS>::inline());

                // Insert field types
                #(<#fields as qubit::TypeDependencies>::get_deps(dependencies);)*
            }
        }
    }
    .into()
}
