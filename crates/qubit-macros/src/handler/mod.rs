use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, spanned::Spanned, Error, FnArg, ItemFn, Pat, Result, ReturnType, Type,
    TypeImplTrait, Visibility,
};

mod options;

pub use options::*;

/// Kind of return value from a handler.
enum HandlerReturn {
    /// Handler returns a stream of the provided type.
    Stream(Type),

    /// Handle returns a single instance of the provided type.
    Return(Type),
}

impl HandlerReturn {
    pub fn ty(&self) -> Type {
        match self {
            Self::Stream(ty) | Self::Return(ty) => ty.clone(),
        }
    }
}

/// All relevant information about the handler extracted from the macro.
struct Handler {
    /// Visibility of the handler.
    visibility: Visibility,

    /// Name of the handler.
    name: Ident,

    /// Type of the context used in the handler.
    ctx_ty: Type,

    /// Inputs for the handler. Currently does not support any kind of destructuring.
    inputs: Vec<(Ident, Type)>,

    /// Return type of the handler.
    return_type: HandlerReturn,

    /// The actual handler implementation.
    implementation: ItemFn,
}

impl Handler {
    pub fn parse(handler: ItemFn, options: HandlerOptions) -> Result<Self> {
        let span = handler.span();

        // TODO: Could this be relaxed?
        if handler.sig.asyncness.is_none() {
            return Err(Error::new(span, "handlers must be async"));
        }

        let implementation = {
            // Create the implementation by cloning the original function, and changing the
            // name to be `handler`.
            let mut implementation = handler.clone();
            implementation.sig.ident = Ident::new("handler", Span::call_site());
            implementation
        };

        let mut inputs = handler
            .sig
            .inputs
            .into_iter()
            .map(|arg| {
                let FnArg::Typed(arg) = arg else {
                    return Err(Error::new(span, "handlers cannot take `self` parameter"));
                };

                let Pat::Ident(ident) = *arg.pat else {
                    return Err(Error::new(
                        span,
                        "destructured parameters are not supported in handlers",
                    ));
                };

                let ty = *arg.ty;

                Ok((ident.ident, ty))
            })
            .collect::<Result<Vec<_>>>()?;

        // TODO: Remove this restriction, allow handlers to accept no parameters
        if inputs.is_empty() {
            return Err(Error::new(
                span,
                "handlers must accept atleast one argument (the ctx)",
            ));
        }

        let (_, ctx_ty) = inputs.remove(0);

        Ok(Self {
            implementation,
            visibility: handler.vis,
            name: options.name.unwrap_or(handler.sig.ident),
            ctx_ty,
            inputs,
            return_type: {
                let return_type = match handler.sig.output {
                    ReturnType::Default => HandlerReturn::Return(parse_quote! { () }),
                    ReturnType::Type(_, ty) => match *ty {
                        // BUG: Assuming that any trait implementation is a stream, which definitely isn't
                        // the case.
                        Type::ImplTrait(TypeImplTrait { bounds, .. }) => HandlerReturn::Stream(
                            parse_quote! { <dyn #bounds as futures::Stream>::Item },
                        ),
                        // All other return types will be treated as a regular return type.
                        return_type => HandlerReturn::Return(return_type),
                    },
                };

                match (&return_type, options.kind) {
                    // Valid case, return type matches with handler annotation
                    (HandlerReturn::Stream(_), Some(HandlerKind::Subscription))
                    | (HandlerReturn::Return(_), Some(HandlerKind::Query) | None) => return_type,

                    // Mismatches
                    (HandlerReturn::Stream(_), Some(HandlerKind::Query) | None) => {
                        return Err(Error::new(
                            span,
                            "handler indicated to be a query, however a stream was returned",
                        ));
                    }
                    (HandlerReturn::Return(_), Some(HandlerKind::Subscription)) => {
                        return Err(Error::new(
                            span,
                            "handler indicated to be a subscription, however a stream was not returned",
                        ));
                    }
                }
            },
        })
    }
}

/// Generates the implementation for [`qubit::Handler`] for the provided handler function. The
/// [`HandlerKind`] is required alter how the handler is applied to the router. This could be
/// induced based on the return type of the handler (whether it retrusn a [`futures::Stream`]) or
/// not), but that might cause problems.
pub fn generate_handler(handler: ItemFn, options: HandlerOptions) -> Result<TokenStream> {
    let handler = Handler::parse(handler, options)?;

    let handler_impl = handler.implementation;
    let handler_name = handler.name;
    let handler_name_str = handler_name.to_string();
    let (param_names, param_tys): (Vec<_>, Vec<_>) = handler.inputs.iter().cloned().unzip();
    let param_names_str = param_names
        .iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    let visibility = handler.visibility;
    let ctx_ty = handler.ctx_ty;
    let return_type = handler.return_type.ty();

    // Generate the parameter parsing implementation
    let parse_params = (!handler.inputs.is_empty()).then(|| {
        quote! {
            let (#(#param_names,)*) = params.parse::<(#(#param_tys,)*)>().unwrap();
        }
    });

    let (register_impl, signature) = match handler.return_type {
        HandlerReturn::Return(return_type) => {
            (
                quote! {
                    rpc_builder.query(#handler_name_str, |ctx, params| async move {
                        #parse_params

                        // Run the handler
                        handler(ctx, #(#param_names,)*).await
                    })
                },
                quote! {
                    format!("({}) => Promise<{}>", parameters.join(", "), <#return_type as ts_rs::TS>::name())
                },
            )
        }
        HandlerReturn::Stream(return_type) => {
            let notification_name = format!("{handler_name_str}_notif");
            let unsubscribe_name = format!("{handler_name_str}_unsub");

            (
                quote! {
                    rpc_builder.subscription(#handler_name_str, #notification_name, #unsubscribe_name, |ctx, params| async move {
                        #parse_params

                        // Run the handler
                        handler(ctx, #(#param_names,)*).await
                    })
                },
                quote! {
                    format!("({}) => Stream<{}>", parameters.join(", "), <#return_type as ts_rs::TS>::name())
                },
            )
        }
    };

    Ok(quote! {
        #[allow(non_camel_case_types)]
        #visibility struct #handler_name;
        impl<__internal_AppCtx> qubit::Handler<__internal_AppCtx> for #handler_name
            where #ctx_ty: qubit::FromContext<__internal_AppCtx>,
                __internal_AppCtx: 'static + Send + Sync + Clone
        {
            fn get_type() -> qubit::HandlerType {
                let parameters = [
                    #((#param_names_str, <#param_tys as ts_rs::TS>::name())),*
                ]
                    .into_iter()
                    .map(|(param, ty): (&str, String)| {
                        format!("{param}: {ty}")
                    })
                    .collect::<Vec<_>>();

                qubit::HandlerType {
                    name: #handler_name_str.to_string(),
                    signature: #signature,
                }
            }

            fn register(rpc_builder: qubit::RpcBuilder<__internal_AppCtx>) -> qubit::RpcBuilder<__internal_AppCtx> {
                #handler_impl

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
