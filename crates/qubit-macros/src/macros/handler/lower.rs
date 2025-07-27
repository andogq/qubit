use quote::quote;
use syn::{Expr, Ident, ItemFn, parse_quote};

use super::{analyse::Model, parse::HandlerKind};

pub fn lower(model: Model) -> Ir {
    Ir {
        name: model.name,
        kind: {
            let variant = match model.kind {
                HandlerKind::Query => quote!(Query),
                HandlerKind::Mutation => quote!(Mutation),
                HandlerKind::Subscription => quote!(Subscription),
            };

            parse_quote!(::qubit::__private::HandlerKind::#variant)
        },
        rpc_name: model.rpc_name,
        param_names: model
            .param_names
            .into_iter()
            .map(|param| param.to_string())
            .collect(),
        handler: model.handler,
    }
}

pub struct Ir {
    pub name: Ident,
    pub kind: Expr,
    pub rpc_name: String,
    pub param_names: Vec<String>,
    pub handler: ItemFn,
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::super::analyse::ModelAssertion;

    use super::*;

    #[derive(Clone)]
    struct IrAssertion {
        name: Ident,
        kind: Expr,
        rpc_name: String,
        param_names: Vec<String>,
    }

    impl IrAssertion {
        fn new(name: Ident, kind: Expr) -> Self {
            Self {
                rpc_name: name.to_string(),
                name,
                kind,
                param_names: Vec::new(),
            }
        }

        fn query(name: Ident) -> Self {
            Self::new(name, parse_quote!(::qubit::__private::HandlerKind::Query))
        }

        fn mutation(name: Ident) -> Self {
            Self::new(
                name,
                parse_quote!(::qubit::__private::HandlerKind::Mutation),
            )
        }

        fn subscription(name: Ident) -> Self {
            Self::new(
                name,
                parse_quote!(::qubit::__private::HandlerKind::Subscription),
            )
        }

        fn with_rpc_name(mut self, rpc_name: impl ToString) -> Self {
            self.rpc_name = rpc_name.to_string();
            self
        }

        fn with_param_names(
            mut self,
            param_names: impl IntoIterator<Item = impl ToString>,
        ) -> Self {
            self.param_names = param_names
                .into_iter()
                .map(|param_name| param_name.to_string())
                .collect();
            self
        }
    }

    #[rstest]
    #[case::simple_query(
        ModelAssertion::query(parse_quote!(my_handler)),
        IrAssertion::query(parse_quote!(my_handler)),
    )]
    #[case::simple_mutation(
        ModelAssertion::mutation(parse_quote!(my_handler)),
        IrAssertion::mutation(parse_quote!(my_handler)),
    )]
    #[case::simple_subscription(
        ModelAssertion::subscription(parse_quote!(my_handler)),
        IrAssertion::subscription(parse_quote!(my_handler)),
    )]
    #[case::single_param(
        ModelAssertion::query(parse_quote!(my_handler))
            .with_param_names([parse_quote!(param_a)]),
        IrAssertion::query(parse_quote!(my_handler))
            .with_param_names(["param_a"]),
    )]
    #[case::multi_param(
        ModelAssertion::query(parse_quote!(my_handler))
            .with_param_names([parse_quote!(param_a), parse_quote!(param_b)]),
        IrAssertion::query(parse_quote!(my_handler))
            .with_param_names(["param_a", "param_b"]),
    )]
    #[case::with_rename(
        ModelAssertion::query(parse_quote!(my_handler))
            .with_rpc_name("other_name"),
        IrAssertion::query(parse_quote!(my_handler))
            .with_rpc_name("other_name"),
    )]
    fn valid(#[case] model: ModelAssertion, #[case] expected: IrAssertion) {
        let name = model.name;
        let ir = lower(Model {
            rpc_name: model.rpc_name,
            kind: model.kind,
            param_names: model.param_names,
            handler: parse_quote!(fn #name() {}),
            name,
        });

        assert_eq!(ir.name, expected.name);
        assert_eq!(ir.kind, expected.kind);
        assert_eq!(ir.rpc_name, expected.rpc_name);
        assert_eq!(ir.param_names, expected.param_names);
    }
}
