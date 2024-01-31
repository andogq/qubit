use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, Error, FnArg, Item, ItemFn, Pat, Result, ReturnType};

fn generate_signature(f: ItemFn) -> Result<TokenStream> {
    // Handlers must be async
    if f.sig.asyncness.is_none() {
        return Err(Error::new_spanned(f, "RPC handlers must be async"));
    }

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
                    (#param_name, <#param_type as ts_rs::TS>::name(), ts_rs::Dependency::from_ty::<#param_type>())
                })
            } else {
                Err(Error::new(pat.span(), "unsupported parameter type"))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let return_type = match f.sig.output.clone() {
        ReturnType::Default => quote!("void"),
        ReturnType::Type(_, ty) => {
            quote!((<#ty as ts_rs::TS>::name(), ts_rs::Dependency::from_ty::<#ty>()))
        }
    };

    let (param_names, param_tys): (Vec<_>, Vec<_>) = f
        .sig
        .inputs
        .iter()
        .filter_map(|param| {
            if let FnArg::Typed(arg) = param {
                Some((arg.pat.clone(), arg.ty.clone()))
            } else {
                None
            }
        })
        .unzip();

    Ok(quote! {
        #[allow(non_camel_case_types)]
        struct #function_ident;
        impl rs_ts_api::Handler for #function_ident {
            fn get_type() -> rs_ts_api::HandlerType {
                let (parameters, mut dependencies): (std::vec::Vec<_>, std::vec::Vec<_>) = [#(#parameters),*]
                    .into_iter()
                    .map(|(param, ty, dependency)| {
                        (format!("{param}: {ty}"), dependency)
                    })
                    .unzip();

                let (return_type, return_dependency) = #return_type;

                dependencies.push(return_dependency);

                rs_ts_api::HandlerType {
                    name: #function_name_str.to_string(),
                    signature: format!("({}) => {}", parameters.join(", "), return_type),
                    dependencies: dependencies.into_iter().flatten().collect(),
                }
            }

            fn register(mut router: jsonrpsee::RpcModule<()>) -> jsonrpsee::RpcModule<()> {
                #handler_fn

                router.register_async_method(#function_name_str, |params, _ctx| async move {
                    // Parse the parameters from the request
                    let (#(#param_names,)*) = params.parse::<(#(#param_tys,)*)>().unwrap();

                    // Run the handler
                    let result = handler(#(#param_names,)*).await;

                    // Serialise the resulte
                    serde_json::to_value(result).unwrap()
                })
                .unwrap();

                router
            }
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
