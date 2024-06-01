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
