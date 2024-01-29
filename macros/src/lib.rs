use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Error, FnArg, Item, ItemFn, Pat, Result, ReturnType};

fn generate_signature(f: ItemFn) -> Result<TokenStream> {
    let original_impl = f.clone();

    let function_name = f.sig.ident.to_string();
    let parameters = f
        .sig
        .inputs
        .into_iter()
        .filter_map(|input| match input {
            FnArg::Typed(pat) => Some(pat),
            _ => None,
        })
        .map(|pat| {
            if let Pat::Ident(ident) = *pat.pat {
                let param_name = ident.ident.to_string();
                let param_type = pat.ty;

                Ok(quote! {
                    (#param_name, <#param_type as ts_rs::TS>::inline())
                })
            } else {
                Err(Error::new(pat.span(), "unsupported parameter type"))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let return_type = match f.sig.output {
        ReturnType::Default => quote!("void"),
        ReturnType::Type(_, ty) => quote!(<#ty as ts_rs::TS>::inline()),
    };

    let test_fn = format_ident!("export_bindings_{}", function_name);

    Ok(quote! {
        #[cfg(test)]
        #[test]
        fn #test_fn() {
            let parameters = [#(#parameters),*]
                .into_iter()
                .map(|(param, ty)| {
                    format!("{param}: {ty}")
                })
                .collect::<Vec<_>>()
                .join(",");

            println!("const {}: ({}) => {};", #function_name, parameters, #return_type);
        }

        #original_impl
    })
}

#[proc_macro_attribute]
pub fn handler(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match entry(input) {
        Err(err) => err.to_compile_error().into(),
        Ok(result) => result,
    }
    .into()
}

fn entry(input: proc_macro::TokenStream) -> Result<TokenStream> {
    let input = syn::parse::<Item>(input)?;
    match input {
        Item::Fn(f) => generate_signature(f).into(),
        _ => Err(Error::new(input.span(), "unsupported item")),
    }
}
