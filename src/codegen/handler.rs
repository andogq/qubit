use ts_rs::{TS, TypeVisitor};

use crate::{__private::HandlerKind, handler::ts::TsTypeTuple};

pub trait HandlerBuilder {
    type Output;

    fn new(name: &'static str, kind: HandlerKind) -> Self;
    fn push_param<T: TS + 'static + ?Sized>(&mut self, param_name: &'static str);
    fn returning<T: TS + 'static + ?Sized>(self) -> Self::Output;
}

/// Visits parameters in a [`TsTypeTuple`], and registers them with the associated parameter name
/// to the handler.
pub enum ParamVisitor<B> {
    /// Visitor is in a good state.
    Ok {
        /// Handler to register parameters against.
        handler: B,
        /// Remaining parameter names.
        param_names: &'static [&'static str],
    },
    /// A parameter type was visited without any more parameter names. Tracks how many more
    /// parameters were expected.
    MissingNames(usize),
}

impl<B> ParamVisitor<B>
where
    B: HandlerBuilder,
{
    /// Visit the parameters in the provided [`TsTypeTuple`], match each type with a value from
    /// `param_names`, and push the parameter to the handler.
    pub fn visit<Params>(
        handler: B,
        param_names: &'static [&'static str],
    ) -> Result<B, ParamVisitorError>
    where
        Params: TsTypeTuple,
    {
        let mut visitor = Self::Ok {
            handler,
            param_names,
        };

        Params::visit_tys(&mut visitor);

        match visitor {
            ParamVisitor::Ok {
                handler,
                param_names,
            } => {
                if param_names.is_empty() {
                    Ok(handler)
                } else {
                    Err(ParamVisitorError::MissingTypes(param_names.len()))
                }
            }
            ParamVisitor::MissingNames(missed) => Err(ParamVisitorError::MissingNames(missed)),
        }
    }
}

impl<B> TypeVisitor for ParamVisitor<B>
where
    B: HandlerBuilder,
{
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        match self {
            ParamVisitor::Ok {
                param_names,
                handler: target_handler,
            } => {
                if param_names.is_empty() {
                    *self = ParamVisitor::MissingNames(1);
                    return;
                }

                let param_name = &param_names[0];
                *param_names = &param_names[1..];

                target_handler.push_param::<T>(param_name);
            }
            ParamVisitor::MissingNames(missing) => {
                *missing += 1;
            }
        }
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum ParamVisitorError {
    #[error("expected {0} more parameter names")]
    MissingNames(usize),
    #[error("expected {0} more parameter types")]
    MissingTypes(usize),
}

#[cfg(test)]
pub mod test {
    use std::any::TypeId;

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    pub struct AssertHandlerBuilder {
        pub name: &'static str,
        pub kind: HandlerKind,
        pub params: Vec<(&'static str, TypeId)>,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct AssertHandler {
        pub name: &'static str,
        pub kind: HandlerKind,
        pub params: Vec<(&'static str, TypeId)>,
        pub return_ty: TypeId,
    }

    impl HandlerBuilder for AssertHandlerBuilder {
        type Output = AssertHandler;

        fn new(name: &'static str, kind: HandlerKind) -> Self {
            Self {
                name,
                kind,
                params: Vec::new(),
            }
        }

        fn push_param<T: TS + 'static + ?Sized>(&mut self, param_name: &'static str) {
            self.params.push((param_name, TypeId::of::<T>()));
        }

        fn returning<T: TS + 'static + ?Sized>(self) -> Self::Output {
            AssertHandler {
                name: self.name,
                kind: self.kind,
                params: self.params,
                return_ty: TypeId::of::<T>(),
            }
        }
    }

    mod param_visitor {
        use super::*;

        fn param_builder() -> AssertHandlerBuilder {
            AssertHandlerBuilder::new("", HandlerKind::Query)
        }

        #[test]
        fn no_params() {
            let params = ParamVisitor::visit::<()>(param_builder(), &[])
                .unwrap()
                .params;
            assert_eq!(params, &[]);
        }

        #[test]
        fn single_param() {
            let params = ParamVisitor::visit::<(u32,)>(param_builder(), &["param_a"])
                .unwrap()
                .params;
            assert_eq!(params, &[("param_a", TypeId::of::<u32>())]);
        }

        #[test]
        fn multiple_params() {
            let params = ParamVisitor::visit::<(u32, bool, String)>(
                param_builder(),
                &["param_a", "some_boolean", "cool_string"],
            )
            .unwrap()
            .params;
            assert_eq!(
                params,
                &[
                    ("param_a", TypeId::of::<u32>()),
                    ("some_boolean", TypeId::of::<bool>()),
                    ("cool_string", TypeId::of::<String>()),
                ]
            );
        }

        #[test]
        fn missing_names() {
            let Err(err) = ParamVisitor::visit::<(u32, bool, String)>(param_builder(), &[]) else {
                panic!("expected error");
            };

            assert!(matches!(err, ParamVisitorError::MissingNames(3)));
        }

        #[test]
        fn missing_some_names() {
            let Err(err) =
                ParamVisitor::visit::<(u32, bool, String)>(param_builder(), &["param_a"])
            else {
                panic!("expected error");
            };

            assert!(matches!(err, ParamVisitorError::MissingNames(2)));
        }

        #[test]
        fn missing_types() {
            let Err(err) = ParamVisitor::visit::<()>(
                param_builder(),
                &["param_a", "some_boolean", "cool_string"],
            ) else {
                panic!("expected error");
            };

            assert!(matches!(err, ParamVisitorError::MissingTypes(3)));
        }

        #[test]
        fn missing_some_types() {
            let Err(err) = ParamVisitor::visit::<(u32,)>(
                param_builder(),
                &["param_a", "some_boolean", "cool_string"],
            ) else {
                panic!("expected error");
            };

            assert!(matches!(err, ParamVisitorError::MissingTypes(2)));
        }
    }
}
