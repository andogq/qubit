use std::collections::BTreeMap;

pub trait TypeDependencies {
    #[allow(unused_variables)]
    fn get_deps(dependencies: &mut BTreeMap<String, String>) {}
}

impl TypeDependencies for u32 {}
impl TypeDependencies for usize {}
impl TypeDependencies for String {}
impl TypeDependencies for bool {}
impl TypeDependencies for char {}
impl TypeDependencies for () {}
impl<T> TypeDependencies for Option<T> {}
