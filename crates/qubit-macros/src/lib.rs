mod macros;

use proc_macro::TokenStream;

/// See [`qubit::builder::handler`] for more information.
#[proc_macro_attribute]
pub fn handler(attrs: TokenStream, item: TokenStream) -> TokenStream {
    match macros::handler(attrs.into(), item.into()) {
        Ok(ts) => ts,
        Err(e) => e.into_compile_error(),
    }
    .into()
}

/// Mark a type to be exported to TypeScript.
///
/// See [`ts_rs::TS`] for available attributes.
#[proc_macro_attribute]
pub fn ts(attr: TokenStream, item: TokenStream) -> TokenStream {
    macros::ts(attr.into(), item.into()).into()
}
