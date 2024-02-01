use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    meta, parse_macro_input, spanned::Spanned, Error, FnArg, Item, ItemFn, Pat, Result, ReturnType,
};

enum HandlerKind {
    Query,
    Subscription,
}

fn generate_handler(handler: ItemFn, kind: HandlerKind) -> Result<TokenStream> {
    // Handlers must be async
    if handler.sig.asyncness.is_none() {
        return Err(Error::new_spanned(handler, "RPC handlers must be async"));
    }

    let handler_fn = {
        let mut f = handler.clone();
        f.sig.ident = Ident::new("handler", Span::call_site());
        f
    };

    let function_name_str = handler.sig.ident.to_string();
    let function_ident = handler.sig.ident.clone();

    let return_type = match handler.sig.output.clone() {
        ReturnType::Default => quote!("void"),
        ReturnType::Type(_, ty) => {
            quote!((<#ty as ts_rs::TS>::name(), ts_rs::Dependency::from_ty::<#ty>()))
        }
    };

    let (param_names, param_tys): (Vec<_>, Vec<_>) = handler
        .sig
        .inputs
        .iter()
        .map(|param| match param {
            FnArg::Typed(arg) => match arg.pat.as_ref() {
                Pat::Ident(ident) => Ok((ident.clone(), arg.ty.clone())),
                Pat::Struct(_) | Pat::Tuple(_) | Pat::TupleStruct(_) => Err(Error::new(
                    arg.span(),
                    "destructured arguments are not currently supported",
                )),
                _ => Err(Error::new(
                    arg.span(),
                    "unable to process this argument type",
                )),
            },
            FnArg::Receiver(_) => Err(Error::new(
                param.span(),
                "handlers cannot have `self` as a parameter",
            )),
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .unzip();

    let get_type_impl = {
        let parameters = handler
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

        quote!(
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
        )
    };

    let run_handler_body = quote!(
        // Parse the parameters from the request
        let (#(#param_names,)*) = params.parse::<(#(#param_tys,)*)>().unwrap();

        // Run the handler
        let result = handler(#(#param_names,)*).await;

        // Serialise the resulte
        serde_json::to_value(result).unwrap()
    );

    Ok(quote! {
        #[allow(non_camel_case_types)]
        struct #function_ident;
        impl rs_ts_api::Handler for #function_ident {
            fn get_type() -> rs_ts_api::HandlerType {
                #get_type_impl
            }

            fn register(mut router: jsonrpsee::RpcModule<()>) -> jsonrpsee::RpcModule<()> {
                #handler_fn

                router.register_async_method(#function_name_str, |params, _ctx| async move {
                    #run_handler_body
                })
                .unwrap();

                router
            }
        }
    })
}

#[proc_macro_attribute]
pub fn handler(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Extract information from the attribute
    let kind = {
        let mut kind = HandlerKind::Query;

        let attribute_parser = meta::parser(|meta| {
            if meta.path.is_ident("query") {
                kind = HandlerKind::Query;
                Ok(())
            } else if meta.path.is_ident("subscription") {
                kind = HandlerKind::Subscription;
                Ok(())
            } else {
                Err(meta.error("unsupported handler property"))
            }
        });

        parse_macro_input!(attr with attribute_parser);

        kind
    };

    // Attempt to match as a function
    syn::parse::<Item>(input)
        .and_then(|item| {
            if let Item::Fn(handler) = item {
                generate_handler(handler, kind)
            } else {
                Err(Error::new(item.span(), "handlers must be a method"))
            }
        })
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
