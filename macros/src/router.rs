use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Brace,
    ItemFn, Result, Token, Visibility,
};

struct RouterBlock {
    // ident: Ident,
    // brace_token: Brace,
    handlers: Vec<ItemFn>,
}

impl Parse for RouterBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let content = input;

        Ok(RouterBlock {
            // ident: input.parse()?,
            // brace_token: braced!(content in input),
            handlers: {
                let mut handlers = Vec::new();
                while !content.is_empty() {
                    handlers.push(content.parse()?);
                }
                handlers
            },
        })
    }
}

pub fn entry(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let router = parse_macro_input!(tokens as RouterBlock);

    // match router::entry(input).into() {
    //     Err(err) => err.to_compile_error().into(),
    //     Ok(result) => result,
    // }
    // .into()

    // let router_struct = router.ident;
    let stripped_handlers = router
        .handlers
        .into_iter()
        .map(|ItemFn { sig, block, .. }| ItemFn {
            attrs: Vec::new(),
            vis: Visibility::Inherited,
            sig,
            block,
        })
        .collect::<Vec<_>>();

    quote! {
        pub struct UserRouter;

        impl UserRouter {
            #(#stripped_handlers)*
        }
    }
    .into()
}
