use std::{
    any::TypeId,
    collections::{BTreeMap, BTreeSet},
};

use ts_rs::{TS, TypeVisitor};

use crate::reflection::ty::CodegenType;

/// Utility for tracking custom types which other types may depend on.
pub struct DependentTypes {
    /// Tracks all types that have already been analysed, to prevent duplicated types or loops.
    pub visited_types: BTreeSet<TypeId>,
    /// Collection of captured definitions, and the type that they stemmed from.
    pub definitions: BTreeMap<TypeId, (CodegenType, String)>,
}

impl DependentTypes {
    /// Create a new instance.
    pub(crate) fn new() -> Self {
        Self {
            visited_types: BTreeSet::new(),
            definitions: BTreeMap::new(),
        }
    }
}

impl Default for DependentTypes {
    fn default() -> Self {
        Self::new()
    }
}

/// Utilise [`TS`] to perform the actual type walking, and just track what has and hasn't been
/// visited.
impl TypeVisitor for DependentTypes {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        let type_id = TypeId::of::<T>();
        let type_id_no_generics = TypeId::of::<T::WithoutGenerics>();

        let visit_dependencies = !self.visited_types.contains(&type_id_no_generics);
        let visit_generics = !self.visited_types.contains(&type_id);

        // Don't bother processing if this type has already been captured.
        if visit_dependencies {
            self.visited_types.insert(type_id_no_generics);

            // Pass the type to the backend, if it's a user type.
            if T::output_path().is_some() {
                self.definitions.insert(
                    TypeId::of::<T::WithoutGenerics>(),
                    CodegenType::from_type_with_definition::<T::WithoutGenerics>(),
                );
            }

            // Process dependent types
            T::visit_dependencies(self);
        }

        if visit_generics {
            self.visited_types.insert(type_id);

            // Process all generic typGes.
            T::visit_generics(self);
        }
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use serde::Deserialize;

    use super::*;

    fn assert_set<T: Ord>(set: &BTreeSet<T>, expected: &[T]) {
        assert_eq!(set.len(), expected.len());
        for value in expected {
            assert!(set.contains(value));
        }
    }
    fn assert_map<K: Ord, V: Debug + PartialEq>(set: &BTreeMap<K, V>, expected: &[(K, V)]) {
        assert_eq!(set.len(), expected.len());
        for (key, value) in expected {
            assert_eq!(set.get(key).unwrap(), value);
        }
    }

    #[test]
    fn visit_unit() {
        let mut types = DependentTypes::new();
        types.visit::<()>();
        assert_set(&types.visited_types, &[TypeId::of::<()>()]);
        assert_map(&types.definitions, &[]);
    }

    #[test]
    fn visit_primitive() {
        let mut types = DependentTypes::new();
        types.visit::<u32>();
        assert_set(&types.visited_types, &[TypeId::of::<u32>()]);
        assert_map(&types.definitions, &[]);
    }

    #[test]
    fn custom_ty() {
        #[derive(TS, Clone, Deserialize)]
        struct MyType;

        let mut types = DependentTypes::new();
        types.visit::<MyType>();
        assert_set(&types.visited_types, &[TypeId::of::<MyType>()]);
        assert_map(
            &types.definitions,
            &[(
                TypeId::of::<MyType>(),
                CodegenType::from_type_with_definition::<MyType>(),
            )],
        );
    }

    #[test]
    fn custom_ty_in_generic() {
        #[derive(TS, Clone, Deserialize)]
        struct MyType;

        let mut types = DependentTypes::new();
        types.visit::<Vec<MyType>>();
        assert_set(
            &types.visited_types,
            &[
                TypeId::of::<Vec<MyType>>(),
                TypeId::of::<Vec<ts_rs::Dummy>>(),
                TypeId::of::<MyType>(),
            ],
        );
        assert_map(
            &types.definitions,
            &[(
                TypeId::of::<MyType>(),
                CodegenType::from_type_with_definition::<MyType>(),
            )],
        );
    }

    #[test]
    fn custom_ty_in_option() {
        // NOTE: ts-rs treats `Option` as a special case, and doesn't consider it a part of the
        // generic.

        #[derive(TS, Clone, Deserialize)]
        struct MyType;

        let mut types = DependentTypes::new();
        types.visit::<Option<MyType>>();
        assert_set(
            &types.visited_types,
            &[TypeId::of::<Option<MyType>>(), TypeId::of::<MyType>()],
        );
        assert_map(
            &types.definitions,
            &[(
                TypeId::of::<MyType>(),
                CodegenType::from_type_with_definition::<MyType>(),
            )],
        );
    }

    #[test]
    fn custom_ty_with_generic() {
        #[derive(TS, Clone, Deserialize)]
        struct MyType<T>(T);
        #[derive(TS, Clone, Deserialize)]
        struct MyInnerType;

        let mut types = DependentTypes::new();
        types.visit::<MyType<MyInnerType>>();
        assert_set(
            &types.visited_types,
            &[
                TypeId::of::<MyType<MyInnerType>>(),
                TypeId::of::<MyType<ts_rs::Dummy>>(),
                TypeId::of::<MyInnerType>(),
            ],
        );
        assert_map(
            &types.definitions,
            &[
                (
                    TypeId::of::<MyType<ts_rs::Dummy>>(),
                    CodegenType::from_type_with_definition::<MyType<ts_rs::Dummy>>(),
                ),
                (
                    TypeId::of::<MyInnerType>(),
                    CodegenType::from_type_with_definition::<MyInnerType>(),
                ),
            ],
        );
    }
}
