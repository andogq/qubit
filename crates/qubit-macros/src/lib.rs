use quote::quote;
use syn::Item;

mod handler;
mod macros;

/// See [`qubit::builder::handler`] for more information.
#[proc_macro_attribute]
pub fn handler(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    macros::handler(attr, input)
}

#[proc_macro_derive(TypeDependencies)]
pub fn derive_type_dependencies(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let s = syn::parse::<Item>(input).unwrap();

    let (target_struct, fields) = match s {
        Item::Struct(ref s) => (
            s.ident.clone(),
            s.fields.iter().map(|field| field.ty.clone()),
        ),
        _ => unimplemented!(),
    };

    quote! {
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
