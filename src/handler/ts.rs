//! Utilities for representing TypeScript types at runtime.

use ts_rs::{TS, TypeVisitor};

/// Tuple of [`TS`] types.
pub trait TsTypeTuple {
    fn visit_tys(visitor: &mut impl TypeVisitor);
}

macro_rules! impl_ts_type_tuple {
    (impl [$($params:ident,)*]) => {
        impl<$($params: 'static + TS,)*> TsTypeTuple for ($($params,)*) {
            fn visit_tys(#[allow(unused)] visitor: &mut impl TypeVisitor) {
                $(visitor.visit::<$params>();)*
            }
        }
    };

    (recurse []) => {};

    (recurse [$param:ident, $($params:ident,)*]) => {
        impl_ts_type_tuple!($($params),*);
    };

    ($($params:ident),* $(,)?) => {
        impl_ts_type_tuple!(impl [$($params,)*]);
        impl_ts_type_tuple!(recurse [$($params,)*]);
    };
}

impl_ts_type_tuple!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);

#[cfg(test)]
pub use self::test::TypeCollector;

#[cfg(test)]
mod test {
    use super::*;

    pub struct TypeCollector(Vec<(String, Option<String>)>);

    impl TypeCollector {
        pub fn collect_names<T: TsTypeTuple>() -> Vec<String> {
            TypeCollector::collect::<T>()
                .into_iter()
                .map(|(name, _)| name)
                .collect()
        }

        pub fn collect<T: TsTypeTuple>() -> Vec<(String, Option<String>)> {
            let mut types = Self(Vec::new());
            T::visit_tys(&mut types);
            types.0
        }
    }

    impl TypeVisitor for TypeCollector {
        fn visit<T: TS + 'static + ?Sized>(&mut self) {
            self.0
                .push((T::name(), T::output_path().map(|_| T::decl())));
        }
    }

    #[test]
    fn empty() {
        assert_eq!(TypeCollector::collect_names::<()>(), &[] as &[&str]);
    }

    #[test]
    fn single() {
        assert_eq!(TypeCollector::collect_names::<(u32,)>(), &["number"]);
    }

    #[test]
    fn multiple() {
        assert_eq!(
            TypeCollector::collect_names::<(u32, bool, String)>(),
            ["number", "boolean", "string"]
        );
    }
}
