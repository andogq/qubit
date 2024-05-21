use std::collections::{BTreeMap, HashMap};

pub trait TypeDependencies {
    #[allow(unused_variables)]
    fn get_deps(dependencies: &mut BTreeMap<String, String>) {}
}

macro_rules! impl_type_dependencies {
    ($($t:ident<$($generic:ident),*>),*) => {
        $(impl<$($generic),*> TypeDependencies for $t<$($generic),*> {})*
    };

    ($($t:ty),*) => {
        $(impl TypeDependencies for $t {})*
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
    Option<T>,
    Result<T, E>,
    HashMap<K, V>
);

macro_rules! impl_tuples {
    ($t:ident) => {
        impl<$t> TypeDependencies for ($t,) {}
    };

    ($t:ident $(, $t_other:ident)*) => {
        impl<$t, $($t_other),*> TypeDependencies for ($t, $($t_other),*) {}

        impl_tuples!($($t_other),*);
    };
}

impl_tuples!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
