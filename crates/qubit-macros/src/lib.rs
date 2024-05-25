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

/// Derive [`qubit::TypeDependencies`] implementation for the attached struct. Will check to see if
/// the struct has been added before, and if not it will add it's own inline definition, and
/// recurse to add the types of any nested types.
#[proc_macro_derive(TypeDependencies)]
pub fn derive_type_dependencies(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_type_dependencies(input)
}
