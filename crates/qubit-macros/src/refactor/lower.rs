use syn::{Expr, Ident, Path, Type, parse_quote};

use super::{
    analyse::{Implementation, Model},
    parse::HandlerKind,
};

pub fn lower(model: Model) -> Ir {
    // Must be a collision-free ident to use as a generic within the handler
    let inner_ctx_ty: Type = parse_quote! { __internal_AppCtx };

    // Identifier to reference ctx instance.
    let ctx_ident: Ident = parse_quote! { ctx };

    Ir {
        // Use the user's ctx, or Qubit's ctx if not provided.
        ctx_ty: model.ctx_ty.as_ref().unwrap_or(&inner_ctx_ty).clone(),

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

        implementation: model.implementation,

        // TODO: Link to where these are defined.
        register_method: match model.kind {
            HandlerKind::Query => parse_quote!(query),
            HandlerKind::Mutation => parse_quote!(mutation),
            HandlerKind::Subscription => parse_quote!(subscription),
        },
        register_params: {
            let name = model.name.to_string();

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
    }
}

#[derive(Clone, Debug)]
pub struct Ir {
    /// Type to use as the ctx.
    pub ctx_ty: Type,

    /// Parameters to parse and pass to the handler. If [`None`], no parsing will be performed at
    /// all.
    pub parse_params: Option<Vec<(Ident, Type)>>,
    /// Parameters to call the handler with.
    pub handler_params: Vec<Ident>,

    /// Implementation of the handler.
    pub implementation: Implementation,

    /// RPC builder method responsible for registering the handler.
    pub register_method: Ident,
    /// Parameters to pass to the register method (excluding the implementation itself).
    pub register_params: Vec<Expr>,

    /// Builtin Qubit types that are depended on.
    pub qubit_types: Vec<Path>,
}

#[cfg(test)]
mod test {
    use crate::refactor::analyse::ModelAssertion;

    use super::*;

    use rstest::*;
    use syn::ItemFn;

    #[derive(Clone)]
    struct IrAssertion {
        ctx_ty: Type,
        parse_params: Option<Vec<(Ident, Type)>>,
        handler_params: Vec<Ident>,
        register_method: Ident,
        register_params: Vec<Expr>,
        qubit_types: Vec<Path>,
    }

    impl IrAssertion {
        fn new(register_method: Ident, register_params: Vec<Expr>, qubit_type: Path) -> Self {
            Self {
                ctx_ty: parse_quote! { __internal_AppCtx },
                parse_params: None,
                handler_params: Vec::new(),
                register_method,
                register_params,
                qubit_types: vec![qubit_type],
            }
        }

        fn query(name: impl AsRef<str>) -> Self {
            let name = name.as_ref();
            Self::new(
                parse_quote!(query),
                vec![parse_quote!(#name)],
                parse_quote!(::qubit::ty::util::QubitType::Query),
            )
        }

        fn mutation(name: impl AsRef<str>) -> Self {
            let name = name.as_ref();
            Self::new(
                parse_quote!(mutation),
                vec![parse_quote!(#name)],
                parse_quote!(::qubit::ty::util::QubitType::Mutation),
            )
        }

        fn subscription(name: impl AsRef<str>) -> Self {
            let name = name.as_ref();
            Self::new(
                parse_quote!(subscription),
                [
                    name.to_string(),
                    format!("{name}_notif"),
                    format!("{name}_unsub"),
                ]
                .into_iter()
                .map(|lit| Expr::Lit(parse_quote!(#lit)))
                .collect(),
                parse_quote!(::qubit::ty::util::QubitType::Subscription),
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
        IrAssertion::query("my_handler")
    )]
    #[case::simple_mutation(
        ModelAssertion::mutation(parse_quote!(my_handler)),
        IrAssertion::mutation("my_handler")
    )]
    #[case::simple_subscription(
        ModelAssertion::subscription(parse_quote!(my_handler)),
        IrAssertion::subscription("my_handler")
    )]
    #[case::user_ctx(
        ModelAssertion::query(parse_quote!(my_handler))
            .with_ctx_ty(Some(parse_quote!(MyCtx))),
        IrAssertion::query("my_handler")
            .with_ctx_ty(parse_quote!(MyCtx))
            .with_handler_params([parse_quote!(ctx)])
    )]
    #[case::user_params(
        ModelAssertion::query(parse_quote!(my_handler))
            .with_ctx_ty(Some(parse_quote!(MyCtx)))
            .with_inputs([(parse_quote!(param_a), parse_quote!(usize)), (parse_quote!(param_b), parse_quote!(String))]),
        IrAssertion::query("my_handler")
            .with_ctx_ty(parse_quote!(MyCtx))
            .with_parse_params([(parse_quote!(param_a), parse_quote!(usize)), (parse_quote!(param_b), parse_quote!(String))])
            .with_handler_params([parse_quote!(ctx), parse_quote!(param_a), parse_quote!(param_b)])
    )]
    fn valid(#[case] model: ModelAssertion, #[case] expected: IrAssertion) {
        let name = model.name;
        let implementation = {
            let item: ItemFn = parse_quote!(fn #name() {});
            item.into()
        };
        let model = Model {
            name,
            kind: model.kind,
            visibility: model.visibility,
            ctx_ty: model.ctx_ty,
            inputs: model.inputs,
            return_ty: model.return_ty,
            implementation,
        };

        let ir = lower(model);

        assert_eq!(ir.ctx_ty, expected.ctx_ty);
        assert_eq!(ir.parse_params, expected.parse_params);
        assert_eq!(ir.handler_params, expected.handler_params);
        assert_eq!(ir.register_method, expected.register_method);
        assert_eq!(ir.register_params, expected.register_params);
        assert_eq!(ir.qubit_types, expected.qubit_types);
    }
}
