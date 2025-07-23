use proc_macro2::TokenStream;
use syn::Error;

mod analyse;
mod codegen;
mod lower;
mod parse;

use self::{analyse::analyse, codegen::codegen, lower::lower, parse::parse};

pub fn handler(attrs: TokenStream, item: TokenStream) -> Result<TokenStream, Error> {
    let ast = parse(attrs, item)?;
    let model = analyse(ast)?;
    let ir = lower(model);
    Ok(codegen(ir))
}
