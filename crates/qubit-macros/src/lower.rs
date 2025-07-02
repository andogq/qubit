use syn::{Expr, Ident, Path, Type, Visibility, parse_quote};

use super::{
    analyse::{HandlerReturn, Implementation, Model},
    parse::HandlerKind,
};

pub fn lower(model: Model) -> Ir {
    // Must be a collision-free ident to use as a generic within the handler
    let inner_ctx_ty: Type = parse_quote! { __internal_AppCtx };

    // Identifier to reference ctx instance.
    let ctx_ident: Ident = parse_quote! { ctx };

    Ir {
        name: model.name,
        rpc_name: model.rpc_name.clone(),
        visibility: model.visibility,

        // Use the user's ctx, or Qubit's ctx if not provided.
        ctx_ty: model.ctx_ty.as_ref().unwrap_or(&inner_ctx_ty).clone(),
        inner_ctx_ty,

        // Set to `None` if the inputs list is empty, so no parsing takes place.
        parse_params: (!model.inputs.is_empty()).then(|| model.inputs.clone()),
        // Only provide handler params if ctx is used.
        handler_params: model
            .ctx_ty
            .is_some()
            .then(||
            // Prepend the ctx param
            std::iter::once(ctx_ident)
                    // All other inputs follow.
                .chain(model.inputs.iter().map(|(ident, _)| ident.clone()))
                .collect())
            .unwrap_or_default(),

        handler_return_ty: match model.return_ty {
            HandlerReturn::Return(ty) => ty,
            HandlerReturn::Stream(ty) => ty,
        },

        implementation: model.implementation,

        // TODO: Link to where these are defined.
        register_method: match model.kind {
            HandlerKind::Query => parse_quote!(query),
            HandlerKind::Mutation => parse_quote!(mutation),
            HandlerKind::Subscription => parse_quote!(subscription),
        },
        register_params: {
            let name = model.rpc_name;

            let params = match model.kind {
                HandlerKind::Query | HandlerKind::Mutation => &[name] as &[String],
                HandlerKind::Subscription => &[
                    name.clone(),
                    format!("{name}_notif"),
                    format!("{name}_unsub"),
                ],
            };

            params
                .iter()
                .map(|lit| Expr::Lit(parse_quote!(#lit)))
                .collect()
        },

        qubit_types: {
            let ident: Ident = match model.kind {
                HandlerKind::Query => parse_quote!(Query),
                HandlerKind::Mutation => parse_quote!(Mutation),
                HandlerKind::Subscription => parse_quote!(Subscription),
            };

            vec![parse_quote!(::qubit::ty::util::QubitType::#ident)]
        },

        handler_kind_str: match model.kind {
            HandlerKind::Query => "Query".to_string(),
            HandlerKind::Mutation => "Mutation".to_string(),
            HandlerKind::Subscription => "Subscription".to_string(),
        },
    }
}

#[derive(Clone, Debug)]
pub struct Ir {
    /// Handler name.
    pub name: Ident,
    /// Name of the RPC method.
    pub rpc_name: String,
    /// Visibility provided by the user.
    pub visibility: Visibility,

    /// Type to use as the ctx.
    pub ctx_ty: Type,
    /// Name of the generic type for the inner context.
    pub inner_ctx_ty: Type,

    /// Parameters to parse and pass to the handler. If [`None`], no parsing will be performed at
    /// all.
    // TODO: See if this can be a vec directly (impacts generating param parsing)
    pub parse_params: Option<Vec<(Ident, Type)>>,
    /// Parameters to call the handler with.
    pub handler_params: Vec<Ident>,

    /// Return type of the handler.
    pub handler_return_ty: Type,

    /// Implementation of the handler.
    pub implementation: Implementation,

    /// RPC builder method responsible for registering the handler.
    pub register_method: Ident,
    /// Parameters to pass to the register method (excluding the implementation itself).
    pub register_params: Vec<Expr>,

    /// Builtin Qubit types that are depended on.
    pub qubit_types: Vec<Path>,

    /// String representation of the handler kind.
    pub handler_kind_str: String,
}

#[cfg(test)]
mod test {
    use crate::analyse::ModelAssertion;

    use super::*;

    use proc_macro2::Span;
    use rstest::*;
    use syn::ItemFn;

    #[derive(Clone)]
    struct IrAssertion {
        name: Ident,
        rpc_name: String,
        ctx_ty: Type,
        parse_params: Option<Vec<(Ident, Type)>>,
        handler_params: Vec<Ident>,
        register_method: Ident,
        register_params: Vec<Expr>,
        qubit_types: Vec<Path>,
        handler_kind_ts_type: String,
    }

    impl IrAssertion {
        fn new(
            name: Ident,
            rpc_name: impl ToString,
            register_method: Ident,
            register_params: Vec<Expr>,
            qubit_type: Path,
            handler_kind_ts_type: String,
        ) -> Self {
            Self {
                name,
                rpc_name: rpc_name.to_string(),
                ctx_ty: parse_quote! { __internal_AppCtx },
                parse_params: None,
                handler_params: Vec::new(),
                register_method,
                register_params,
                qubit_types: vec![qubit_type],
                handler_kind_ts_type,
            }
        }

        fn query(name: impl AsRef<str>, rpc_name: impl AsRef<str>) -> Self {
            let rpc_name = rpc_name.as_ref();
            Self::new(
                Ident::new(name.as_ref(), Span::call_site()),
                rpc_name,
                parse_quote!(query),
                vec![parse_quote!(#rpc_name)],
                parse_quote!(::qubit::ty::util::QubitType::Query),
                "Query".to_string(),
            )
        }

        fn mutation(name: impl AsRef<str>, rpc_name: impl AsRef<str>) -> Self {
            let rpc_name = rpc_name.as_ref();
            Self::new(
                Ident::new(name.as_ref(), Span::call_site()),
                rpc_name,
                parse_quote!(mutation),
                vec![parse_quote!(#rpc_name)],
                parse_quote!(::qubit::ty::util::QubitType::Mutation),
                "Mutation".to_string(),
            )
        }

        fn subscription(name: impl AsRef<str>, rpc_name: impl AsRef<str>) -> Self {
            let rpc_name = rpc_name.as_ref();
            Self::new(
                Ident::new(name.as_ref(), Span::call_site()),
                rpc_name,
                parse_quote!(subscription),
                [
                    rpc_name.to_string(),
                    format!("{rpc_name}_notif"),
                    format!("{rpc_name}_unsub"),
                ]
                .into_iter()
                .map(|lit| Expr::Lit(parse_quote!(#lit)))
                .collect(),
                parse_quote!(::qubit::ty::util::QubitType::Subscription),
                "Subscription".to_string(),
            )
        }

        fn with_ctx_ty(mut self, ctx_ty: Type) -> Self {
            self.ctx_ty = ctx_ty;
            self
        }

        fn with_parse_params(
            mut self,
            parse_params: impl IntoIterator<Item = (Ident, Type)>,
        ) -> Self {
            self.parse_params = Some(parse_params.into_iter().collect());
            self
        }

        fn with_handler_params(mut self, handler_params: impl IntoIterator<Item = Ident>) -> Self {
            self.handler_params = handler_params.into_iter().collect();
            self
        }
    }

    #[rstest]
    #[case::simple_query(
        ModelAssertion::query(parse_quote!(my_handler)),
        IrAssertion::query("my_handler", "my_handler")
    )]
    #[case::simple_mutation(
        ModelAssertion::mutation(parse_quote!(my_handler)),
        IrAssertion::mutation("my_handler", "my_handler")
    )]
    #[case::simple_subscription(
        ModelAssertion::subscription(parse_quote!(my_handler)),
        IrAssertion::subscription("my_handler", "my_handler")
    )]
    #[case::user_ctx(
        ModelAssertion::query(parse_quote!(my_handler))
            .with_ctx_ty(Some(parse_quote!(MyCtx))),
        IrAssertion::query("my_handler", "my_handler")
            .with_ctx_ty(parse_quote!(MyCtx))
            .with_handler_params([parse_quote!(ctx)])
    )]
    #[case::user_params(
        ModelAssertion::query(parse_quote!(my_handler))
            .with_ctx_ty(Some(parse_quote!(MyCtx)))
            .with_inputs([(parse_quote!(param_a), parse_quote!(usize)), (parse_quote!(param_b), parse_quote!(String))]),
        IrAssertion::query("my_handler", "my_handler")
            .with_ctx_ty(parse_quote!(MyCtx))
            .with_parse_params([(parse_quote!(param_a), parse_quote!(usize)), (parse_quote!(param_b), parse_quote!(String))])
            .with_handler_params([parse_quote!(ctx), parse_quote!(param_a), parse_quote!(param_b)])
    )]
    #[case::rename_handler(
        ModelAssertion::query(parse_quote!(my_handler))
            .with_rpc_name("other_name"),
        IrAssertion::query("my_handler", "other_name")
    )]
    fn valid(#[case] model: ModelAssertion, #[case] expected: IrAssertion) {
        let name = model.name;
        let implementation = {
            let item: ItemFn = parse_quote!(fn #name() {});
            item.into()
        };
        let model = Model {
            name,
            rpc_name: model.rpc_name,
            kind: model.kind,
            visibility: model.visibility,
            ctx_ty: model.ctx_ty,
            inputs: model.inputs,
            return_ty: model.return_ty,
            implementation,
        };

        let ir = lower(model);

        assert_eq!(ir.name, expected.name);
        assert_eq!(ir.rpc_name, expected.rpc_name);
        assert_eq!(ir.ctx_ty, expected.ctx_ty);
        assert_eq!(ir.parse_params, expected.parse_params);
        assert_eq!(ir.handler_params, expected.handler_params);
        assert_eq!(ir.register_method, expected.register_method);
        assert_eq!(ir.register_params, expected.register_params);
        assert_eq!(ir.qubit_types, expected.qubit_types);
        assert_eq!(ir.handler_kind_str, expected.handler_kind_ts_type);
    }
}
