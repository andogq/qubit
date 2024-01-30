mod router;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Error, FnArg, Item, ItemFn, Pat, Result, ReturnType};

#[proc_macro]
pub fn router(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    router::entry(tokens)
}

fn generate_signature(f: ItemFn) -> Result<TokenStream> {
    let handler_fn = {
        let mut f = f.clone();
        f.sig.ident = Ident::new("handler", Span::call_site());
        f
    };

    let function_name_str = f.sig.ident.to_string();
    let function_ident = f.sig.ident.clone();

    let parameters = f
        .sig
        .inputs
        .clone()
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(pat) => Some(pat),
            _ => None,
        })
        .map(|pat| {
            if let Pat::Ident(ident) = *pat.pat.clone() {
                let param_name = ident.ident.to_string();
                let param_type = &pat.ty;

                Ok(quote! {
                    (#param_name, <#param_type as ts_rs::TS>::inline())
                })
            } else {
                Err(Error::new(pat.span(), "unsupported parameter type"))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let return_type = match f.sig.output.clone() {
        ReturnType::Default => quote!("void"),
        ReturnType::Type(_, ty) => quote!(<#ty as ts_rs::TS>::inline()),
    };

    let test_fn = format_ident!("export_bindings_{}", function_name_str);

    let params = f
        .sig
        .inputs
        .iter()
        .filter_map(|param| {
            if let FnArg::Typed(arg) = param {
                Some(arg.ty.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let ReturnType::Type(_, return_ty) = f.sig.output else {
        panic!();
    };

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

            println!("const {}: ({}) => {};", #function_name_str, parameters, #return_type);
        }

        fn #function_ident(mut router: jsonrpsee::RpcModule<()>) -> jsonrpsee::RpcModule<()> {
            #handler_fn

            router.register_async_method(#function_name_str, |params, _ctx| async move {
                rs_ts_api::handler::Handler::<(#(#params,)*), #return_ty>::call(&handler, params.parse::<serde_json::Value>().unwrap())
            });

            router
        }
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
