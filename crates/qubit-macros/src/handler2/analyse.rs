use syn::{Error, FnArg, Ident, ItemFn, Pat, PatIdent, Receiver};

use super::parse::{Ast, HandlerKind};

pub fn analyse(ast: Ast) -> Result<Model, AnalyseError> {
    Ok(Model {
        name: ast.handler.sig.ident.clone(),
        rpc_name: ast
            .attrs
            .name
            .unwrap_or_else(|| ast.handler.sig.ident.to_string()),
        kind: ast.attrs.kind,
        param_names: process_inputs(ast.handler.sig.inputs.iter())?,
        handler: ast.handler,
    })
}

#[derive(Clone, Debug)]
pub struct Model {
    /// Identifier of the handler.
    pub name: Ident,

    /// Identifier of the RPC method.
    pub rpc_name: String,

    /// Kind of the handler.
    pub kind: HandlerKind,

    /// Name of all the parameters (excluding the `ctx`).
    pub param_names: Vec<Ident>,

    /// The actual handler implementation.
    pub handler: ItemFn,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum AnalyseError {
    #[error(transparent)]
    Input(#[from] InputError),
}

impl From<AnalyseError> for Error {
    fn from(err: AnalyseError) -> Self {
        match err {
            AnalyseError::Input(input_error) => input_error.into(),
        }
    }
}
/// From a collection of [`FnArg`]s, extract the parameter names (excluding the `ctx` parameter).
fn process_inputs<'a>(inputs: impl Iterator<Item = &'a FnArg>) -> Result<Vec<Ident>, InputError> {
    let mut inputs = inputs
        .map(|arg| {
            let arg = match arg {
                FnArg::Typed(arg) => arg,
                FnArg::Receiver(receiver) => {
                    return Err(InputError::SelfParameter(receiver.clone()));
                }
            };

            let Pat::Ident(PatIdent { ref ident, .. }) = *arg.pat else {
                return Err(InputError::Destructured(arg.pat.clone()));
            };

            Ok(ident.clone())
        })
        .collect::<Result<Vec<_>, _>>()?;

    if !inputs.is_empty() {
        inputs.remove(0);
    }

    Ok(inputs)
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum InputError {
    #[error("handlers cannot take `self` parameter")]
    SelfParameter(Receiver),
    #[error("destructured parameters are not supported in handlers")]
    Destructured(Box<Pat>),
}

impl From<InputError> for Error {
    fn from(err: InputError) -> Self {
        match &err {
            InputError::SelfParameter(receiver) => Error::new_spanned(receiver, err.to_string()),
            InputError::Destructured(pat) => Error::new_spanned(pat, err.to_string()),
        }
    }
}

#[cfg(test)]
pub use test::ModelAssertion;

#[cfg(test)]
mod test {
    use super::*;

    use rstest::*;
    use syn::parse_quote;

    use super::super::parse::Attributes;

    #[derive(Clone)]
    pub struct ModelAssertion {
        pub name: Ident,
        pub rpc_name: String,
        pub kind: HandlerKind,
        pub param_names: Vec<Ident>,
    }

    impl ModelAssertion {
        pub fn new(name: Ident, kind: HandlerKind) -> Self {
            Self {
                rpc_name: name.to_string(),
                name,
                kind,
                param_names: Vec::new(),
            }
        }

        pub fn query(name: Ident) -> Self {
            Self::new(name, HandlerKind::Query)
        }

        pub fn mutation(name: Ident) -> Self {
            Self::new(name, HandlerKind::Mutation)
        }

        pub fn subscription(name: Ident) -> Self {
            Self::new(name, HandlerKind::Subscription)
        }

        pub fn with_rpc_name(mut self, rpc_name: impl ToString) -> Self {
            self.rpc_name = rpc_name.to_string();
            self
        }

        pub fn with_param_names(mut self, param_names: impl IntoIterator<Item = Ident>) -> Self {
            self.param_names = param_names.into_iter().collect();
            self
        }
    }

    mod analyse {
        use syn::Signature;

        use super::*;

        #[rstest]
        #[case::simple_query(
            Attributes::query(),
            parse_quote!(async fn my_handler()),
            ModelAssertion::query(parse_quote!(my_handler))
        )]
        #[case::simple_mutation(
            Attributes::mutation(),
            parse_quote!(async fn my_handler()),
            ModelAssertion::mutation(parse_quote!(my_handler))
        )]
        #[case::simple_subscription(
            Attributes::subscription(),
            parse_quote!(async fn my_handler()),
            ModelAssertion::subscription(parse_quote!(my_handler))
        )]
        #[case::rename(
            Attributes::query().with_name("other_name"),
            parse_quote!(async fn my_handler()),
            ModelAssertion::query(parse_quote!(my_handler))
                .with_rpc_name("other_name")
        )]
        #[case::visibility_pub(
            Attributes::query(),
            parse_quote!(async fn my_handler()),
            ModelAssertion::query(parse_quote!(my_handler))
        )]
        #[case::visibility_complex(
            Attributes::query(),
            parse_quote!(async fn my_handler()),
            ModelAssertion::query(parse_quote!(my_handler))
        )]
        #[case::ctx_only(
            Attributes::query(),
            parse_quote!(async fn my_handler(ctx: Ctx)),
            ModelAssertion::query(parse_quote!(my_handler))
        )]
        #[case::single_param(
            Attributes::query(),
            parse_quote!(async fn my_handler(ctx: Ctx, param_a: String)),
            ModelAssertion::query(parse_quote!(my_handler))
                .with_param_names([parse_quote!(param_a)])
        )]
        #[case::multi_param(
            Attributes::query(),
            parse_quote!(async fn my_handler(ctx: Ctx, param_a: String, param_b: bool, param_c: usize)),
            ModelAssertion::query(parse_quote!(my_handler))
                .with_param_names([parse_quote!(param_a), parse_quote!(param_b), parse_quote!(param_c)])
        )]
        #[case::return_value(
            Attributes::query(),
            parse_quote!(async fn my_handler() -> usize),
            ModelAssertion::query(parse_quote!(my_handler))
        )]
        #[case::query_everything(
            Attributes::query().with_name("other_name"),
            parse_quote!(async fn my_handler(ctx: Ctx, param_a: String, param_b: bool, param_c: usize) -> usize),
            ModelAssertion::query(parse_quote!(my_handler))
                .with_rpc_name("other_name")
                .with_param_names([parse_quote!(param_a), parse_quote!(param_b), parse_quote!(param_c)])
        )]
        #[case::mutation_everything(
            Attributes::mutation().with_name("other_name"),
            parse_quote!(async fn my_handler(ctx: Ctx, param_a: String, param_b: bool, param_c: usize) -> usize),
            ModelAssertion::mutation(parse_quote!(my_handler))
                .with_rpc_name("other_name")
                .with_param_names([parse_quote!(param_a), parse_quote!(param_b), parse_quote!(param_c)])
        )]
        #[case::subscription_everything(
            Attributes::subscription().with_name("other_name"),
            parse_quote!(async fn my_handler(ctx: Ctx, param_a: String, param_b: bool, param_c: usize) -> usize),
            ModelAssertion::subscription(parse_quote!(my_handler))
                .with_rpc_name("other_name")
                .with_param_names([parse_quote!(param_a), parse_quote!(param_b), parse_quote!(param_c)])
        )]
        fn valid(
            #[case] attrs: Attributes,
            #[case] signature: Signature,
            #[case] expected: ModelAssertion,
        ) {
            let model = analyse(Ast::new(attrs, parse_quote!(#signature { todo!() }))).unwrap();

            assert_eq!(model.name, expected.name);
            assert_eq!(model.rpc_name, expected.rpc_name);
            assert_eq!(model.kind, expected.kind);
            assert_eq!(model.param_names, expected.param_names);
        }

        #[rstest]
        #[case::self_param(
            Attributes::query(),
            parse_quote!(async fn my_handler(self)),
            |e| matches!(e, AnalyseError::Input(InputError::SelfParameter(_))),
        )]
        #[case::destructured_param(
            Attributes::query(),
            parse_quote!(async fn my_handler(SomeType { a, b }: SomeType)),
            |e| matches!(e, AnalyseError::Input(InputError::Destructured(_))),
        )]
        fn invalid(
            #[case] attrs: Attributes,
            #[case] signature: Signature,
            #[case] err_check: fn(AnalyseError) -> bool,
        ) {
            let err = analyse(Ast::new(attrs, parse_quote!(#signature { todo!() }))).unwrap_err();
            assert!(err_check(err));
        }
    }

    mod process_inputs {
        use super::*;

        #[rstest]
        #[case::empty(&[], &[])]
        #[case::ctx(&[parse_quote!(ctx: Ctx)], &[])]
        #[case::single(&[parse_quote!(ctx: Ctx), parse_quote!(n: usize)], &[parse_quote!(n)])]
        #[case::multiple(
            &[parse_quote!(ctx: Ctx), parse_quote!(n: usize), parse_quote!(name: String), parse_quote!(thing: bool)],
            &[parse_quote!(n), parse_quote!(name), parse_quote!(thing)]
        )]
        #[case::type_path(&[parse_quote!(ctx: Ctx), parse_quote!(value: some_crate::path::Type)], &[parse_quote!(value)])]
        fn valid<'a>(
            #[case] inputs: impl IntoIterator<Item = &'a FnArg>,
            #[case] expected: &[Ident],
        ) {
            let inputs = process_inputs(inputs.into_iter()).unwrap();
            assert_eq!(inputs, expected);
        }

        #[rstest]
        #[case::reject_self(&[parse_quote!(self)], |e| matches!(e, InputError::SelfParameter(_)))]
        #[case::reject_self_after_input(&[parse_quote!(n: usize), parse_quote!(self)], |e| matches!(e, InputError::SelfParameter(_)))]
        #[case::reject_wildcard(&[parse_quote!(_: usize)], |e| matches!(e, InputError::Destructured(_)))]
        #[case::reject_destructuring(&[parse_quote!(SomeType { a, b }: SomeType)], |e| matches!(e, InputError::Destructured(_)))]
        fn fail<'a>(
            #[case] inputs: impl IntoIterator<Item = &'a FnArg>,
            #[case] err_check: fn(InputError) -> bool,
        ) {
            let err = process_inputs(inputs.into_iter()).unwrap_err();
            assert!(err_check(err));
        }
    }
}
