use proc_macro2::Span;
use syn::{meta, parse_macro_input, spanned::Spanned, Error, Item};

use crate::handler::{Handler, HandlerOptions};

pub fn handler(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Extract information from the attribute
    let options = {
        let mut options_builder = HandlerOptions::builder();

        let attribute_parser = meta::parser(|meta| options_builder.parse(meta));

        parse_macro_input!(attr with attribute_parser);

        let Some(options) = options_builder.build() else {
            // Produce a compiler error
            // TODO: Make it a better erro
            return Error::new(
                Span::call_site(),
                "handler type must be provided (`query`, `mutation`, or `subscription`)",
            )
            .into_compile_error()
            .into();
        };

        options
    };

    // Attempt to match as a function
    syn::parse::<Item>(input)
        .and_then(|item| {
            if let Item::Fn(handler) = item {
                let handler = Handler::parse(handler, options)?;
                Ok(handler.into())
            } else {
                Err(Error::new(item.span(), "handlers must be a method"))
            }
        })
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
