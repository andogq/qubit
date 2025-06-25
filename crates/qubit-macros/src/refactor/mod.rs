mod parse;

use self::parse::parse;
use proc_macro::TokenStream;
use syn::{Error, ItemFn, Meta, Token, punctuated::Punctuated};

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

    todo!()
}
