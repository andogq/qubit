use syn::Error;

mod analyse;
mod codegen;
mod lower;
mod parse;

use self::{analyse::analyse, codegen::codegen, lower::lower, parse::parse};

pub fn handler_inner(
    attrs: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream, Error> {
    let ast = parse(attrs, item)?;
    let model = analyse(ast)?;
    let ir = lower(model);
    Ok(codegen(ir))
}
