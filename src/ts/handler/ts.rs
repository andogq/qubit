//! Utilities for representing TypeScript types at runtime.

use std::{any::TypeId, ops::Deref};

use derive_more::Deref;
use ts_rs::TS;

/// Common components of [`TsType`].
#[derive(Clone, Debug)]
pub struct TsTypeCommon {
    /// TypeScript name of the type. Could be the primitive (`number`, `string`), or a
    /// user-defined type.
    pub name: String,
}

/// User-defined type.
#[derive(Clone, Debug, Deref)]
pub struct TsTypeUser {
    #[deref]
    common: TsTypeCommon,

    /// Rust type that this refers to. The same Rust type will correspond to the same
    /// TypeScript type (with the exception of numbers like [`i32`] and [`u32`] which are both
    ///  `number`).
    pub id: std::any::TypeId,
    /// Declaration of this type.
    pub declaration: String,
}

/// Type information to represent a type in TypeScript.
#[derive(Clone, Debug)]
pub enum TsType {
    /// Built-in TypeScript type.
    Primitive(TsTypeCommon),
    /// User-defined TypeScript type.
    User(TsTypeUser),
}

impl TsType {
    /// Determine if the type is primitive.
    pub fn is_primitive(&self) -> bool {
        matches!(self, Self::Primitive(_))
    }

    /// Determine if the type is user-defined.
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User(_))
    }

    /// Produce type information for the given Rust type.
    pub fn from_type<T: 'static + TS + ?Sized>() -> Self {
        let common = TsTypeCommon { name: T::name() };

        // Determine whether the type is primitive or not based on whether the output path is defined.
        match T::output_path() {
            Some(_) => Self::User(TsTypeUser {
                common,
                id: TypeId::of::<T>(),
                declaration: T::decl(),
            }),
            None => Self::Primitive(common),
        }
    }
}

impl Deref for TsType {
    type Target = TsTypeCommon;

    fn deref(&self) -> &Self::Target {
        match self {
            TsType::Primitive(ts_type_common) => ts_type_common,
            TsType::User(ts_type_user) => ts_type_user,
        }
    }
}

/// Tuple of [`TsType`] types.
pub trait TsTypeTuple {
    /// Produce all of the [`TsType`] for each of the types in the tuple, in order.
    fn get_ts_types() -> Vec<TsType>;
}

macro_rules! impl_ts_type_tuple {
    (impl [$($params:ident,)*]) => {
        impl<$($params: 'static + TS,)*> TsTypeTuple for ($($params,)*) {
            fn get_ts_types() -> Vec<TsType> {
                vec![$(TsType::from_type::<$params>(),)*]
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
mod test {
    use super::*;

    #[test]
    fn valid_primitive() {
        let ts_type = TsType::from_type::<u32>();
        assert_eq!(ts_type.name, "number");
        assert!(ts_type.is_primitive());
    }

    #[test]
    fn valid_user_defined() {
        #[derive(TS)]
        struct MyType;

        let ts_type = TsType::from_type::<MyType>();
        assert_eq!(ts_type.name, "MyType");
        assert!(ts_type.is_user());
    }

    mod ts_tupe_tuple {
        use super::*;

        #[test]
        fn empty() {
            let types = <()>::get_ts_types();
            assert!(types.is_empty());
        }

        #[test]
        fn single() {
            let types = <(u32,)>::get_ts_types();
            assert_eq!(types.len(), 1);
            assert_eq!(types[0].name, "number");
        }

        #[test]
        fn multiple() {
            let types = <(u32, bool, String)>::get_ts_types();
            assert_eq!(types.len(), 3);
            assert_eq!(types[0].name, "number");
            assert_eq!(types[1].name, "boolean");
            assert_eq!(types[2].name, "string");
        }
    }
}
