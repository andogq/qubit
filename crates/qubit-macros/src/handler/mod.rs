use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
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
    /// Get the equivalent TS type for this return type.
    fn ts_container(&self) -> String {
        match self {
            Self::Stream(_) => "Stream".to_string(),
            Self::Return(_) => "Promise".to_string(),
        }
    }
}

impl ToTokens for HandlerReturn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Stream(ty) | Self::Return(ty) => ty.to_tokens(tokens),
        }
    }
}

/// All relevant information about the handler extracted from the macro.
pub struct Handler {
    /// Visibility of the handler.
    visibility: Visibility,

    /// Name of the handler.
    name: Ident,

    /// Type of the context used in the handler.
    ctx_ty: Option<Type>,

    /// Inputs for the handler. Currently does not support any kind of destructuring.
    inputs: Vec<(Ident, Type)>,

    /// Return type of the handler.
    return_type: HandlerReturn,

    /// The actual handler implementation.
    implementation: ItemFn,
}

impl Handler {
    /// Parse a handler from an [`ItemFn`] and some options. This will return [`syn::Error`]s if
    /// parsing cannot take place.
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

        let ctx_ty = if inputs.is_empty() {
            None
        } else {
            Some(inputs.remove(0).1)
        };

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

    /// Produce a list of parameter names as idents that this handler accepts.
    fn parameter_names(&self) -> Vec<Ident> {
        self.inputs.iter().map(|(name, _)| name).cloned().collect()
    }

    /// Produce a list of parameter names as strings that this handler accepts.
    fn parameter_names_str(&self) -> Vec<String> {
        self.parameter_names()
            .iter()
            .map(|name| name.to_string())
            .collect()
    }

    /// Produce a list of parameter types that this handler accepts.
    fn parameter_tys(&self) -> Vec<Type> {
        self.inputs.iter().map(|(_, ty)| ty).cloned().collect()
    }

    /// Produce a token stream that will generate the TS signature of this handler.
    fn get_signature(&self) -> TokenStream {
        let return_type = &self.return_type;
        let container = return_type.ts_container();

        let param_names_str = self.parameter_names_str();
        let param_tys = self.parameter_tys();

        quote! {
            {
                let parameters = [
                    #((#param_names_str, <#param_tys as ts_rs::TS>::name())),*
                ]
                    .into_iter()
                    .map(|(param, ty): (&str, String)| {
                        format!("{param}: {ty}")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("({}) => {}<{}>", parameters, #container, <#return_type as ts_rs::TS>::name())
            }
        }
    }
}

impl From<Handler> for TokenStream {
    fn from(handler: Handler) -> Self {
        // Generate the signature
        let param_names = handler.parameter_names();
        let param_tys = handler.parameter_tys();
        let signature = handler.get_signature();

        // Extract required elements from handler
        let Handler {
            visibility,
            name,
            ctx_ty,
            inputs,
            return_type,
            implementation,
        } = handler;

        let handler_name_str = name.to_string();

        // Must be a collision-free ident to use as a generic within the handler
        let inner_ctx_ty: Type = parse_quote! { __internal_AppCtx };

        // Record whether the handler needs a ctx passed to it
        let ctx_required = ctx_ty.is_some();

        // Use the ctx type, or default back to the app ctx type if none is provided
        let ctx_ty = ctx_ty.unwrap_or_else(|| inner_ctx_ty.clone());

        let register_impl = {
            // Define idents in one place, so they will be checked by the compiler
            let ctx_ident = quote! { ctx };
            let params_ident = quote! { params };

            // Generate the parameter parsing implementation
            let parse_params = (!inputs.is_empty()).then(|| {
                quote! {
                    let (#(#param_names,)*) = #params_ident.parse::<(#(#param_tys,)*)>().unwrap();
                }
            });

            let handler_call = if ctx_required {
                quote! { handler(#ctx_ident, #(#param_names,)*).await }
            } else {
                quote! { handler().await }
            };

            // Body of the handler registration implementation
            let register_inner = quote! {
                #parse_params

                #handler_call
            };

            match &return_type {
                HandlerReturn::Return(_) => {
                    quote! {
                        rpc_builder.query(#handler_name_str, |#ctx_ident: #ctx_ty, #params_ident| async move {
                            #register_inner
                        })
                    }
                }
                HandlerReturn::Stream(_) => {
                    let notification_name = format!("{handler_name_str}_notif");
                    let unsubscribe_name = format!("{handler_name_str}_unsub");

                    quote! {
                        rpc_builder.subscription(
                            #handler_name_str,
                            #notification_name,
                            #unsubscribe_name,
                            |#ctx_ident: #ctx_ty, #params_ident| async move {
                                #register_inner
                            }
                        )
                    }
                }
            }
        };

        // Generate implementation of the `qubit_types` method.
        let qubit_types = if let HandlerReturn::Stream(_) = return_type {
            quote! { ::std::vec![::qubit::ty::util::QubitType::Stream] }
        } else {
            quote! { ::std::vec![] }
        };

        quote! {
            #[allow(non_camel_case_types)]
            #visibility struct #name;
            impl<#inner_ctx_ty> qubit::Handler<#inner_ctx_ty> for #name
                where #inner_ctx_ty: 'static + Send + Sync + Clone,
                     #ctx_ty: ::qubit::FromContext<#inner_ctx_ty>,
            {
                fn get_type() -> qubit::HandlerType {
                    qubit::HandlerType {
                        name: #handler_name_str.to_string(),
                        signature: #signature,
                    }
                }

                fn register(rpc_builder: qubit::RpcBuilder<#inner_ctx_ty>) -> qubit::RpcBuilder<#inner_ctx_ty> {
                    #implementation

                    #register_impl
                }

                fn export_all_dependencies_to(out_dir: &::std::path::Path) -> ::std::result::Result<::std::vec::Vec<::ts_rs::Dependency>, ::ts_rs::ExportError> {
                    // Export the return type
                    let mut dependencies = ::qubit::ty::util::export_with_dependencies::<#return_type>(out_dir)?;

                    // Export each of the parameters
                    #(dependencies.extend(::qubit::ty::util::export_with_dependencies::<#param_tys>(out_dir)?);)*

                    ::std::result::Result::Ok(dependencies)
                }

                fn qubit_types() -> ::std::vec::Vec<::qubit::ty::util::QubitType> {
                    #qubit_types
                }
            }
        }
    }
}
