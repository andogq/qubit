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

/// Mark a type to be exported to TypeScript.
///
/// See [`ts_rs::TS`] for available attributes.
#[proc_macro_attribute]
pub fn ts(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item: proc_macro2::TokenStream = item.into();
    let attr: proc_macro2::TokenStream = attr.into();

    let ts_rs_path = quote::quote!(::qubit::__private::ts_rs);
    let ts_rs_path_str = ts_rs_path.to_string();

    let attr = {
        let crate_attr = quote::quote!(crate = #ts_rs_path_str);

        // Append any user-provided arguments
        if attr.is_empty() {
            crate_attr
        } else {
            quote::quote!(#crate_attr, #attr)
        }
    };

    quote::quote! {
        #[derive(#ts_rs_path::TS)]
        #[ts(#attr)]
        #item
    }
    .into()
}
