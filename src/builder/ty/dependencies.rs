use std::collections::{BTreeMap, HashMap};

pub trait TypeDependencies {
    #[allow(unused_variables)]
    fn get_deps(dependencies: &mut BTreeMap<String, String>) {}
}

macro_rules! impl_type_dependencies {
    ($($t:ident<$($generic:ident),*>),*) => {
        $(impl<$($generic),*> TypeDependencies for $t<$($generic),*>
            where $($generic: ts_rs::TS + crate::TypeDependencies),*
            {
            fn get_deps(dependencies: &mut BTreeMap<String, String>) {
                $(impl_type_dependencies!(generic: $generic, dependencies);)*
            }
        })*
    };

    ($($t:ty),*) => {
        $(impl TypeDependencies for $t {})*
    };

    (tuple: $t:ident) => {
        impl<$t> TypeDependencies for ($t,)
            where $t: ts_rs::TS + crate::TypeDependencies,
        {
            fn get_deps(dependencies: &mut BTreeMap<String, String>) {
                impl_type_dependencies!(generic: $t, dependencies);
            }
        }
    };

    (tuple: $t:ident $(, $t_other:ident)*) => {
        impl<$t, $($t_other),*> TypeDependencies for ($t, $($t_other),*)
            where $t: ts_rs::TS + crate::TypeDependencies,
            $($t_other: ts_rs::TS + crate::TypeDependencies),*
        {
            fn get_deps(dependencies: &mut BTreeMap<String, String>) {
                impl_type_dependencies!(generic: $t, dependencies);
                $(impl_type_dependencies!(generic: $t_other, dependencies);)*
            }
        }

        impl_type_dependencies!(tuple: $($t_other),*);
    };

    (generic: $generic:ident, $dependencies:ident) => {
        <$generic as TypeDependencies>::get_deps($dependencies)
    };
}

impl_type_dependencies!(
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    String,
    &'static str,
    bool,
    char,
    ()
);
impl_type_dependencies!(
    Vec<T>,
    Box<T>,
    Option<T>,
    Result<T, E>,
    HashMap<K, V>
);
impl_type_dependencies!(tuple: T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
