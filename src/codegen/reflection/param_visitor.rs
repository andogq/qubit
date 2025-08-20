use ts_rs::{TS, TypeVisitor};

use crate::{handler::ts::TsTypeTuple, reflection::ty::CodegenType};

/// Visits parameters in a [`TsTypeTuple`], and registers them with the associated parameter name
/// to the handler.
pub enum ParamVisitor {
    /// Visitor is in a good state.
    Ok {
        /// Handler to register parameters against.
        params: Vec<(&'static str, CodegenType)>,
        /// Remaining parameter names.
        param_names: &'static [&'static str],
    },
    /// A parameter type was visited without any more parameter names. Tracks how many more
    /// parameters were expected.
    MissingNames(usize),
}

impl ParamVisitor {
    /// Visit the parameters in the provided [`TsTypeTuple`], match each type with a value from
    /// `param_names`, and push the parameter to the handler.
    pub fn visit<Params>(
        param_names: &'static [&'static str],
    ) -> Result<Vec<(&'static str, CodegenType)>, ParamVisitorError>
    where
        Params: TsTypeTuple,
    {
        let mut visitor = Self::Ok {
            params: Vec::new(),
            param_names,
        };

        Params::visit_tys(&mut visitor);

        match visitor {
            ParamVisitor::Ok {
                params,
                param_names,
            } => {
                if param_names.is_empty() {
                    Ok(params)
                } else {
                    Err(ParamVisitorError::MissingTypes(param_names.len()))
                }
            }
            ParamVisitor::MissingNames(missed) => Err(ParamVisitorError::MissingNames(missed)),
        }
    }
}

impl TypeVisitor for ParamVisitor {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        match self {
            ParamVisitor::Ok {
                param_names,
                params,
            } => {
                if param_names.is_empty() {
                    *self = ParamVisitor::MissingNames(1);
                    return;
                }

                let param_name = &param_names[0];
                *param_names = &param_names[1..];

                params.push((param_name, CodegenType::from_type::<T>()));
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
    use super::*;

    #[test]
    fn no_params() {
        let params = ParamVisitor::visit::<()>(&[]).unwrap();
        assert_eq!(params, &[]);
    }

    #[test]
    fn single_param() {
        let params = ParamVisitor::visit::<(u32,)>(&["param_a"]).unwrap();
        assert_eq!(params, &[("param_a", CodegenType::from_type::<u32>())]);
    }

    #[test]
    fn multiple_params() {
        let params =
            ParamVisitor::visit::<(u32, bool, String)>(&["param_a", "some_boolean", "cool_string"])
                .unwrap();
        assert_eq!(
            params,
            &[
                ("param_a", CodegenType::from_type::<u32>()),
                ("some_boolean", CodegenType::from_type::<bool>()),
                ("cool_string", CodegenType::from_type::<String>()),
            ]
        );
    }

    #[test]
    fn missing_names() {
        let err = ParamVisitor::visit::<(u32, bool, String)>(&[]).unwrap_err();
        assert!(matches!(err, ParamVisitorError::MissingNames(3)));
    }

    #[test]
    fn missing_some_names() {
        let err = ParamVisitor::visit::<(u32, bool, String)>(&["param_a"]).unwrap_err();
        assert!(matches!(err, ParamVisitorError::MissingNames(2)));
    }

    #[test]
    fn missing_types() {
        let err =
            ParamVisitor::visit::<()>(&["param_a", "some_boolean", "cool_string"]).unwrap_err();
        assert!(matches!(err, ParamVisitorError::MissingTypes(3)));
    }

    #[test]
    fn missing_some_types() {
        let err =
            ParamVisitor::visit::<(u32,)>(&["param_a", "some_boolean", "cool_string"]).unwrap_err();
        assert!(matches!(err, ParamVisitorError::MissingTypes(2)));
    }
}
