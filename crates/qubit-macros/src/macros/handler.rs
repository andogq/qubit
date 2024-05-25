use syn::{meta, parse_macro_input, spanned::Spanned, Error, Item};

use crate::handler::{Handler, HandlerOptions};

pub fn handler(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Extract information from the attribute
    let options = {
        let mut options = HandlerOptions::default();

        let attribute_parser = meta::parser(|meta| options.parse(meta));

        parse_macro_input!(attr with attribute_parser);

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
