mod analyse;
mod codegen;
mod lower;
mod parse;

use self::{analyse::analyse, codegen::codegen, lower::lower, parse::parse};
use proc_macro::TokenStream;
use syn::Error;

/// See [`qubit::builder::handler`] for more information.
#[proc_macro_attribute]
pub fn handler(attrs: TokenStream, item: TokenStream) -> TokenStream {
    match handler_inner(attrs.into(), item.into()) {
        Ok(ts) => ts,
        Err(e) => e.into_compile_error(),
    }
    .into()
}

fn handler_inner(
    attrs: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream, Error> {
    let ast = parse(attrs, item)?;
    let model = analyse(ast)?;
    let ir = lower(model);
    Ok(codegen(ir))
}
