use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    meta::ParseNestedMeta, parse_quote, spanned::Spanned, Error, FnArg, ItemFn, Pat, Result,
    ReturnType,
};

/// Handlers can have different variations depending on how they interact with the client.
pub enum HandlerKind {
    /// Query handlers support the standard request/response pattern.
    Query,

    /// Subscriptions have an initial request, and returns a stream of responses that the client
    /// will continue to consume.
    Subscription,
}

impl HandlerKind {
    /// Attempt to parse the handler kind from [`ParseNestedMeta`].
    pub fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
        if meta.path.is_ident("query") {
            *self = Self::Query;
            Ok(())
        } else if meta.path.is_ident("subscription") {
            *self = Self::Subscription;
            Ok(())
        } else {
            Err(meta.error("unsupported handler property"))
        }
    }
}

/// Generates the implementation for [`rs_ts_api::Handler`] for the provided handler function. The
/// [`HandlerKind`] is required alter how the handler is applied to the router. This could be
/// induced based on the return type of the handler (whether it retrusn a [`futures::Stream`]) or
/// not), but that might cause problems.
pub fn generate_handler(handler: ItemFn, kind: HandlerKind) -> Result<TokenStream> {
    let span = handler.span().clone();

    // Handlers must be async
    if handler.sig.asyncness.is_none() {
        return Err(Error::new_spanned(handler, "RPC handlers must be async"));
    }

    // Clone the function implementation, in order to use it as the handler
    let handler_fn = {
        let mut f = handler.clone();
        f.sig.ident = Ident::new("handler", Span::call_site());
        f
    };

    // Extract out the function name
    let function_name_str = handler.sig.ident.to_string();
    let function_ident = handler.sig.ident;

    // Extract out the return type
    let return_type = match handler.sig.output.clone() {
        ReturnType::Default => quote!("void"),
        ReturnType::Type(_, ty) => {
            quote!(<#ty as ts_rs::TS>::name())
        }
    };

    let mut inputs = handler.sig.inputs.iter();

    let ctx_ty = if let Some(FnArg::Typed(arg)) = inputs.next() {
        arg.ty.clone()
    } else {
        return Err(syn::Error::new(span, "ctx type must be provided"));
    };

    // Process parameters, to get the idents, string version of the idents, and the type
    let ((param_names, param_name_strs), param_tys): ((Vec<_>, Vec<_>), Vec<_>) = inputs
        .map(|param| match param {
            FnArg::Typed(arg) => match arg.pat.as_ref() {
                Pat::Ident(ident) => Ok(((ident.clone(), ident.ident.to_string()), arg.ty.clone())),
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

    let parse_params = (!param_names.is_empty()).then(|| {
        quote! {
            let (#(#param_names,)*) = params.parse::<(#(#param_tys,)*)>().unwrap();
        }
    });

    let register_impl = match kind {
        HandlerKind::Query => quote! {
            rpc_builder.query(#function_name_str, |app_ctx, params| async move {
                #parse_params

                // Convert app_ctx to ctx
                let ctx = <#ctx_ty as rs_ts_api::Context<__internal_AppCtx>>::from_app_ctx(app_ctx).unwrap();

                // Run the handler
                let result = handler(ctx, #(#param_names,)*).await;

                // Serialise the resulte
                serde_json::to_value(result).unwrap()
            })
        },
        HandlerKind::Subscription => {
            let notification_name = format!("{function_name_str}_notif");
            let unsubscribe_name = format!("{function_name_str}_unsub");

            quote! {
                rpc_builder.subscription(#function_name_str, #notification_name, #unsubscribe_name, |app_ctx, params| async move {
                    #parse_params

                    // Convert app_ctx to ctx

                    // Run the handler
                    handler(ctx, #(#param_names,)*)
                })
            }
        }
    };

    let return_ty = match handler.sig.output.clone() {
        ReturnType::Default => parse_quote!(()),
        ReturnType::Type(_, ty) => ty,
    };

    Ok(quote! {
        #[allow(non_camel_case_types)]
        struct #function_ident;
        impl<__internal_AppCtx> rs_ts_api::Handler<__internal_AppCtx> for #function_ident
            where #ctx_ty: rs_ts_api::Context<__internal_AppCtx>,
                __internal_AppCtx: 'static + Send + Sync + Clone
        {
            fn get_type() -> rs_ts_api::HandlerType {
                let parameters = [
                    #((#param_name_strs, <#param_tys as ts_rs::TS>::name())),*
                ]
                    .into_iter()
                    .map(|(param, ty): (&str, String)| {
                        format!("{param}: {ty}")
                    })
                    .collect::<Vec<_>>();

                rs_ts_api::HandlerType {
                    name: #function_name_str.to_string(),
                    signature: format!("({}) => Promise<{}>", parameters.join(", "), #return_type),
                }
            }

            fn register(rpc_builder: rs_ts_api::RpcBuilder<__internal_AppCtx>) -> rs_ts_api::RpcBuilder<__internal_AppCtx> {
                #handler_fn

                #register_impl
            }

            fn add_dependencies(dependencies: &mut std::collections::BTreeMap<std::string::String, std::string::String>) {
                // Add dependencies for the parameters
                #(<#param_tys as rs_ts_api::TypeDependencies>::get_deps(dependencies);)*

                // Add dependencies for the return type
                <#return_ty as rs_ts_api::TypeDependencies>::get_deps(dependencies);
            }
        }
    })
}
