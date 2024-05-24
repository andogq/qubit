use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, Error, FnArg, ItemFn, Pat, Result, ReturnType, Type, TypeImplTrait};

mod options;

pub use options::*;

/// Generates the implementation for [`qubit::Handler`] for the provided handler function. The
/// [`HandlerKind`] is required alter how the handler is applied to the router. This could be
/// induced based on the return type of the handler (whether it retrusn a [`futures::Stream`]) or
/// not), but that might cause problems.
pub fn generate_handler(handler: ItemFn, options: HandlerOptions) -> Result<TokenStream> {
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
    let function_ident = options.name.unwrap_or(handler.sig.ident);
    let function_name_str = function_ident.to_string();
    let function_visibility = handler.vis;

    // Extract out the return type
    let (return_type, stream_item) = match handler.sig.output.clone() {
        ReturnType::Default => (quote! { () }, false),
        ReturnType::Type(_, ty) => match *ty {
            Type::ImplTrait(TypeImplTrait { bounds, .. }) => {
                (quote! { <dyn #bounds as futures::Stream>::Item }, true)
            }
            ref return_type => (return_type.to_token_stream(), false),
        },
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

    let (register_impl, signature) = match options.kind.unwrap_or(HandlerKind::Query) {
        HandlerKind::Query => (
            quote! {
                rpc_builder.query(#function_name_str, |ctx, params| async move {
                    #parse_params

                    // Run the handler
                    handler(ctx, #(#param_names,)*).await
                })
            },
            quote! {
                format!("({}) => Promise<{}>", parameters.join(", "), <#return_type as ts_rs::TS>::name())
            },
        ),
        HandlerKind::Subscription => {
            let notification_name = format!("{function_name_str}_notif");
            let unsubscribe_name = format!("{function_name_str}_unsub");

            (
                quote! {
                    rpc_builder.subscription(#function_name_str, #notification_name, #unsubscribe_name, |ctx, params| async move {
                        #parse_params

                        // Run the handler
                        handler(ctx, #(#param_names,)*).await
                    })
                },
                {
                    if !stream_item {
                        return Err(syn::Error::new(
                            span,
                            "subscriptions must have a return type",
                        ));
                    };

                    quote! {
                        format!("({}) => Stream<{}>", parameters.join(", "), <#return_type as ts_rs::TS>::name())
                    }
                },
            )
        }
    };

    Ok(quote! {
        #[allow(non_camel_case_types)]
        #function_visibility struct #function_ident;
        impl<__internal_AppCtx> qubit::Handler<__internal_AppCtx> for #function_ident
            where #ctx_ty: qubit::FromContext<__internal_AppCtx>,
                __internal_AppCtx: 'static + Send + Sync + Clone
        {
            fn get_type() -> qubit::HandlerType {
                let parameters = [
                    #((#param_name_strs, <#param_tys as ts_rs::TS>::name())),*
                ]
                    .into_iter()
                    .map(|(param, ty): (&str, String)| {
                        format!("{param}: {ty}")
                    })
                    .collect::<Vec<_>>();

                qubit::HandlerType {
                    name: #function_name_str.to_string(),
                    signature: #signature,
                }
            }

            fn register(rpc_builder: qubit::RpcBuilder<__internal_AppCtx>) -> qubit::RpcBuilder<__internal_AppCtx> {
                #handler_fn

                #register_impl
            }

            fn add_dependencies(dependencies: &mut std::collections::BTreeMap<std::string::String, std::string::String>) {
                // Add dependencies for the parameters
                #(<#param_tys as qubit::TypeDependencies>::get_deps(dependencies);)*

                // Add dependencies for the return type
                <#return_type as qubit::TypeDependencies>::get_deps(dependencies);
            }
        }
    })
}
