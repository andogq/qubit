use proc_macro2::Span;
use syn::{
    Attribute, Block, Error, FnArg, Ident, ItemFn, Pat, PatIdent, ReturnType, Signature, Token,
    Type, TypeImplTrait, Visibility, parse_quote, punctuated::Punctuated, spanned::Spanned,
};

use super::parse::{Ast, HandlerKind};

pub fn analyse(ast: Ast) -> Result<Model, AnalyseError> {
    // Determine the name of the handler.
    let name = ast
        .attrs
        .name
        .as_ref()
        .unwrap_or(&ast.handler.sig.ident)
        .clone();

    // Assert that the handler is an async function.
    // TODO: Could this be relaxed?
    if ast.handler.sig.asyncness.is_none() {
        return Err(AnalyseError::ExpectedAsyncHandler(ast.handler.span()));
    }

    let kind = ast.attrs.kind;

    let visibility = ast.handler.vis.clone();

    // Process all of the inputs from the signature.
    let mut inputs = process_inputs(ast.handler.sig.inputs.iter())?;

    // Assume the first parameter is the ctx.
    let ctx_ty = (!inputs.is_empty()).then(|| inputs.remove(0).1);

    // TODO: This complex analysis doesn't need to take place. This can be handled by trait
    // implementations that this code expands into.
    let return_ty = process_return_ty(&ast.handler.sig.output, ast.attrs.kind)?;

    let implementation = ast.handler.into();

    Ok(Model {
        name,
        kind,
        visibility,
        ctx_ty,
        inputs,
        return_ty,
        implementation,
    })
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum AnalyseError {
    #[error("handlers must be async")]
    ExpectedAsyncHandler(Span),
    #[error(transparent)]
    Input(#[from] InputError),
    #[error(transparent)]
    ReturnTy(#[from] ReturnTyError),
}

impl From<AnalyseError> for Error {
    fn from(err: AnalyseError) -> Self {
        match err {
            AnalyseError::ExpectedAsyncHandler(span) => Error::new(span, err.to_string()),
            AnalyseError::Input(input_error) => input_error.into(),
            AnalyseError::ReturnTy(return_ty_error) => return_ty_error.into(),
        }
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum InputError {
    #[error("handlers cannot take `self` parameter")]
    SelfParameter(Span),
    #[error("destructured parameters are not supported in handlers")]
    Destructured(Span),
}

impl From<InputError> for Error {
    fn from(err: InputError) -> Self {
        Error::new(
            match err {
                InputError::SelfParameter(span) => span,
                InputError::Destructured(span) => span,
            },
            err.to_string(),
        )
    }
}

fn process_inputs<'a>(
    inputs: impl Iterator<Item = &'a FnArg>,
) -> Result<Vec<(Ident, Type)>, InputError> {
    inputs
        .map(|arg| {
            let arg = match arg {
                FnArg::Typed(arg) => arg,
                FnArg::Receiver(receiver) => {
                    return Err(InputError::SelfParameter(receiver.span()));
                }
            };

            let Pat::Ident(PatIdent { ref ident, .. }) = *arg.pat else {
                return Err(InputError::Destructured(arg.pat.span()));
            };

            Ok((ident.clone(), (*arg.ty).clone()))
        })
        .collect()
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum ReturnTyError {
    #[error("handler isn't a subscription, but a stream was returned")]
    InvalidStream(Span),
    #[error("a stream must be returned from a subscription")]
    ExpectedStream(Span),
}

impl From<ReturnTyError> for Error {
    fn from(err: ReturnTyError) -> Self {
        Error::new(
            match err {
                ReturnTyError::InvalidStream(span) => span,
                ReturnTyError::ExpectedStream(span) => span,
            },
            err.to_string(),
        )
    }
}

fn process_return_ty(
    return_ty: &ReturnType,
    handler_kind: HandlerKind,
) -> Result<HandlerReturn, ReturnTyError> {
    let handler_return = match return_ty {
        ReturnType::Default => HandlerReturn::Return(parse_quote! { () }),
        ReturnType::Type(_, ty) => match &**ty {
            // BUG: Assuming that any trait implementation is a stream, which definitely isn't
            // the case.
            Type::ImplTrait(TypeImplTrait { bounds, .. }) => {
                HandlerReturn::Stream(parse_quote! { <dyn #bounds as ::futures::Stream>::Item })
            }
            // All other return types will be treated as a regular return type.
            return_type => HandlerReturn::Return(return_type.clone()),
        },
    };

    match (&handler_return, handler_kind) {
        // Valid case, return type matches with handler annotation
        (HandlerReturn::Stream(_), HandlerKind::Subscription)
        | (HandlerReturn::Return(_), HandlerKind::Query | HandlerKind::Mutation) => {
            Ok(handler_return)
        }

        // Mismatches
        (HandlerReturn::Stream(_), HandlerKind::Query | HandlerKind::Mutation) => {
            Err(ReturnTyError::InvalidStream(return_ty.span()))
        }
        (HandlerReturn::Return(_), HandlerKind::Subscription) => {
            Err(ReturnTyError::ExpectedStream(return_ty.span()))
        }
    }
}

#[derive(Clone, Debug)]
pub struct Model {
    /// Handler name.
    pub name: Ident,

    /// Kind of the handler.
    pub kind: HandlerKind,

    /// Visibility provided by the user.
    pub visibility: Visibility,

    /// Context type of the handler.
    pub ctx_ty: Option<Type>,

    /// Handler parameters. Currently does not support any kind of destructuring.
    pub inputs: Vec<(Ident, Type)>,

    /// Return type of the handler.
    pub return_ty: HandlerReturn,

    /// Implementation of the handler.
    pub implementation: Implementation,
}

/// All relevant components of a handler implementation. Where possible the original components of
/// the handler should be re-used, to ensure that any additional attributes are retained.
#[derive(Clone, Debug)]
pub struct Implementation {
    /// Function body.
    pub block: Block,
    /// Attributes attached to the function.
    pub attrs: Vec<Attribute>,

    /// Optional async keyword attached to the function.
    pub asyncness: Option<Token![async]>,
    /// Input parameters for the function.
    pub inputs: Punctuated<FnArg, Token![,]>,
    /// Return type of the function.
    pub output: ReturnType,
}

impl From<ItemFn> for Implementation {
    fn from(item: ItemFn) -> Self {
        Self {
            block: *item.block,
            attrs: item.attrs,

            asyncness: item.sig.asyncness,
            inputs: item.sig.inputs,
            output: item.sig.output,
        }
    }
}

// TODO: Work out if this is even required, and get rid of it if not.
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum HandlerReturn {
    /// [`Type`] returned directly.
    Return(Type),

    /// Stream containing [`Type`] items.
    Stream(Type),
}

#[cfg(test)]
pub use test::ModelAssertion;

#[cfg(test)]
mod test {
    use super::*;

    use rstest::*;
    use syn::Signature;

    use crate::refactor::parse::Attributes;

    #[derive(Clone)]
    pub struct ModelAssertion {
        pub name: Ident,
        pub kind: HandlerKind,
        pub visibility: Visibility,
        pub ctx_ty: Option<Type>,
        pub inputs: Vec<(Ident, Type)>,
        pub return_ty: HandlerReturn,
    }

    impl ModelAssertion {
        pub fn new(name: Ident, kind: HandlerKind) -> Self {
            Self {
                name,
                kind,
                visibility: Visibility::Inherited,
                ctx_ty: None,
                inputs: Vec::new(),
                return_ty: HandlerReturn::Return(parse_quote!(())),
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

        pub fn with_visibility(mut self, visibility: Visibility) -> Self {
            self.visibility = visibility;
            self
        }

        pub fn with_ctx_ty(mut self, ctx_ty: Option<Type>) -> Self {
            self.ctx_ty = ctx_ty;
            self
        }

        pub fn with_inputs(mut self, inputs: impl IntoIterator<Item = (Ident, Type)>) -> Self {
            self.inputs = inputs.into_iter().collect();
            self
        }

        pub fn with_return_ty(mut self, return_ty: HandlerReturn) -> Self {
            self.return_ty = return_ty;
            self
        }
    }

    mod analyse {

        use super::*;

        #[rstest]
        #[case::simple_query(
            Attributes::query(),
            parse_quote!(),
            parse_quote!(async fn my_handler()),
            ModelAssertion::query(parse_quote!(my_handler))
        )]
        #[case::simple_mutation(
            Attributes::mutation(),
            parse_quote!(),
            parse_quote!(async fn my_handler()),
            ModelAssertion::mutation(parse_quote!(my_handler))
        )]
        #[case::rename(
            Attributes::query().with_name("other_name"),
            parse_quote!(),
            parse_quote!(async fn my_handler()),
            ModelAssertion::query(parse_quote!(other_name))
        )]
        #[case::visibility_pub(
            Attributes::query(),
            parse_quote!(pub),
            parse_quote!(async fn my_handler()),
            ModelAssertion::query(parse_quote!(my_handler))
                .with_visibility(parse_quote!(pub))
        )]
        #[case::visibility_complex(
            Attributes::query(),
            parse_quote!(pub(in crate::some::path)),
            parse_quote!(async fn my_handler()),
            ModelAssertion::query(parse_quote!(my_handler))
                .with_visibility(parse_quote!(pub(in crate::some::path)))
        )]
        #[case::single_parameter(
            Attributes::query(),
            parse_quote!(),
            parse_quote!(async fn my_handler(ctx: usize)),
            ModelAssertion::query(parse_quote!(my_handler))
                .with_ctx_ty(Some(parse_quote!(usize)))
        )]
        #[case::multi_parameter(
            Attributes::query(),
            parse_quote!(),
            parse_quote!(async fn my_handler(ctx: usize, param_a: String, param_b: bool)),
            ModelAssertion::query(parse_quote!(my_handler))
                .with_ctx_ty(Some(parse_quote!(usize)))
                .with_inputs([
                    (parse_quote!(param_a), parse_quote!(String)),
                    (parse_quote!(param_b), parse_quote!(bool)),
                ])
        )]
        #[case::return_ty(
            Attributes::query(),
            parse_quote!(),
            parse_quote!(async fn my_handler() -> usize),
            ModelAssertion::query(parse_quote!(my_handler))
                .with_return_ty(HandlerReturn::Return(parse_quote!(usize)))
        )]
        #[case::simple_subscription(
            Attributes::subscription(),
            parse_quote!(),
            parse_quote!(async fn my_handler() -> impl Stream<Item = usize>),
            ModelAssertion::subscription(parse_quote!(my_handler))
                .with_return_ty(HandlerReturn::Stream(parse_quote!(<dyn Stream<Item = usize> as ::futures::Stream>::Item)))
        )]
        #[case::subscription_with_params(
            Attributes::subscription(),
            parse_quote!(),
            parse_quote!(async fn my_handler(ctx: usize, param_a: String, param_b: bool) -> impl Stream<Item = usize>),
            ModelAssertion::subscription(parse_quote!(my_handler))
                .with_ctx_ty(Some(parse_quote!(usize)))
                .with_inputs([
                    (parse_quote!(param_a), parse_quote!(String)),
                    (parse_quote!(param_b), parse_quote!(bool)),
                ])
                .with_return_ty(HandlerReturn::Stream(parse_quote!(<dyn Stream<Item = usize> as ::futures::Stream>::Item)))
        )]
        #[case::query_everything(
            Attributes::query().with_name("other_name"),
            parse_quote!(pub(in crate::some::path)),
            parse_quote!(async fn my_handler(ctx: usize, param_a: String, param_b: bool) -> f64),
            ModelAssertion::query(parse_quote!(other_name))
                .with_visibility(parse_quote!(pub(in crate::some::path)))
                .with_ctx_ty(Some(parse_quote!(usize)))
                .with_inputs([
                    (parse_quote!(param_a), parse_quote!(String)),
                    (parse_quote!(param_b), parse_quote!(bool)),
                ])
                .with_return_ty(HandlerReturn::Return(parse_quote!(f64)))
        )]
        #[case::mutation_everything(
            Attributes::mutation().with_name("other_name"),
            parse_quote!(pub(in crate::some::path)),
            parse_quote!(async fn my_handler(ctx: usize, param_a: String, param_b: bool) -> f64),
            ModelAssertion::mutation(parse_quote!(other_name))
                .with_visibility(parse_quote!(pub(in crate::some::path)))
                .with_ctx_ty(Some(parse_quote!(usize)))
                .with_inputs([
                    (parse_quote!(param_a), parse_quote!(String)),
                    (parse_quote!(param_b), parse_quote!(bool)),
                ])
                .with_return_ty(HandlerReturn::Return(parse_quote!(f64)))
        )]
        #[case::subscription_everything(
            Attributes::subscription().with_name("other_name"),
            parse_quote!(pub(in crate::some::path)),
            parse_quote!(async fn my_handler(ctx: usize, param_a: String, param_b: bool) -> impl Stream<Item = f64>),
            ModelAssertion::subscription(parse_quote!(other_name))
                .with_visibility(parse_quote!(pub(in crate::some::path)))
                .with_ctx_ty(Some(parse_quote!(usize)))
                .with_inputs([
                    (parse_quote!(param_a), parse_quote!(String)),
                    (parse_quote!(param_b), parse_quote!(bool)),
                ])
                .with_return_ty(HandlerReturn::Stream(parse_quote!(<dyn Stream<Item = f64> as ::futures::Stream>::Item)))
        )]
        fn valid(
            #[case] attrs: Attributes,
            #[case] visibility: Visibility,
            #[case] signature: Signature,
            #[case] expected: ModelAssertion,
        ) {
            let model = analyse(Ast::new(
                attrs,
                parse_quote!(#visibility #signature { todo!() }),
            ))
            .unwrap();

            assert_eq!(model.name, expected.name);
            assert_eq!(model.kind, expected.kind);
            assert_eq!(model.visibility, expected.visibility);
            assert_eq!(model.ctx_ty, expected.ctx_ty);
            assert_eq!(model.inputs, expected.inputs);
            assert_eq!(model.return_ty, expected.return_ty);
        }

        #[rstest]
        #[case::not_async(
            Attributes::query(),
            parse_quote!(fn my_handler()),
            |e| matches!(e, AnalyseError::ExpectedAsyncHandler(_)),
        )]
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
        #[case::query_return_stream(
            Attributes::query(),
            parse_quote!(async fn my_handler() -> impl Stream<Item = usize>),
            |e| matches!(e, AnalyseError::ReturnTy(ReturnTyError::InvalidStream(_))),
        )]
        #[case::subscription_return_non_stream(
            Attributes::subscription(),
            parse_quote!(async fn my_handler() -> usize),
            |e| matches!(e, AnalyseError::ReturnTy(ReturnTyError::ExpectedStream(_))),
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
        #[case::single(&[parse_quote!(n: usize)], &[(parse_quote!(n), parse_quote!(usize))])]
        #[case::multiple(
            &[parse_quote!(n: usize), parse_quote!(name: String), parse_quote!(thing: bool)],
            &[(parse_quote!(n), parse_quote!(usize)), (parse_quote!(name), parse_quote!(String)), (parse_quote!(thing), parse_quote!(bool))]
        )]
        #[case::type_path(&[parse_quote!(value: some_crate::path::Type)], &[(parse_quote!(value), parse_quote!(some_crate::path::Type))])]
        fn valid<'a>(
            #[case] inputs: impl IntoIterator<Item = &'a FnArg>,
            #[case] expected: &[(Ident, Type)],
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

    mod process_return_ty {
        use super::*;

        #[rstest]
        #[case::query_simple_return(parse_quote!(-> usize), HandlerKind::Query, HandlerReturn::Return(parse_quote!(usize)))]
        #[case::mutation_simple_return(parse_quote!(-> usize), HandlerKind::Mutation, HandlerReturn::Return(parse_quote!(usize)))]
        #[case::default_return(parse_quote!(), HandlerKind::Query, HandlerReturn::Return(parse_quote!(())))]
        #[case::stream(
            parse_quote!(-> impl Stream<Item = usize>),
            HandlerKind::Subscription,
            HandlerReturn::Stream(parse_quote!(<dyn Stream<Item = usize> as ::futures::Stream>::Item))
        )]
        fn valid(
            #[case] return_ty: ReturnType,
            #[case] handler_kind: HandlerKind,
            #[case] expected: HandlerReturn,
        ) {
            let return_ty = process_return_ty(&return_ty, handler_kind).unwrap();
            assert_eq!(return_ty, expected);
        }

        #[rstest]
        #[case::subscription_non_stream(parse_quote!(-> usize), HandlerKind::Subscription, |e| matches!(e, ReturnTyError::ExpectedStream(_)))]
        #[case::query_stream(parse_quote!(-> impl Stream<Item = usize>), HandlerKind::Query, |e| matches!(e, ReturnTyError::InvalidStream(_)))]
        #[case::mutation_stream(parse_quote!(-> impl Stream<Item = usize>), HandlerKind::Mutation, |e| matches!(e, ReturnTyError::InvalidStream(_)))]
        // TODO: This test case should be removed at some point, as general traits should be
        // allowed if they pass type checking.
        #[case::query_trait(parse_quote!(-> impl SomeTrait), HandlerKind::Query, |e| matches!(e, ReturnTyError::InvalidStream(_)))]
        fn fail(
            #[case] return_ty: ReturnType,
            #[case] handler_kind: HandlerKind,
            #[case] err_check: fn(ReturnTyError) -> bool,
        ) {
            let err = process_return_ty(&return_ty, handler_kind).unwrap_err();
            assert!(err_check(err));
        }
    }
}
