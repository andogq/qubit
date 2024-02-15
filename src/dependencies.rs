use std::collections::BTreeMap;

pub trait TypeDependencies {
    fn get_deps(dependencies: &mut BTreeMap<String, String>) {}
}

impl TypeDependencies for u32 {}
impl TypeDependencies for String {}
impl TypeDependencies for bool {}
impl TypeDependencies for () {}
impl<T> TypeDependencies for Option<T> {}
