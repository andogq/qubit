mod backend;
mod handler;
mod prefix_map;
mod ts;

use std::{
    any::TypeId,
    collections::{BTreeMap, BTreeSet},
    fmt::{Display, Write},
};

use ts_rs::{TS, TypeVisitor};

pub use self::handler::ParamVisitor;

use crate::{
    __private::{HandlerKind, HandlerMeta},
    RegisterableHandler,
    handler::{marker, response::ResponseValue},
};

pub const QUBIT_HEADER: &str = include_str!("header.txt");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenType {
    name: String,
    generics: Vec<String>,
}

impl CodegenType {
    pub fn from_type_with_definition<T: TS + 'static + ?Sized>() -> (Self, String) {
        // Generate the declaration, which includes `type ... =`, and any generic
        // parameters.
        let declaration = T::decl();

        // Split the declaration into the name and definition.
        let (name, definition) = declaration.split_once("=").expect("valid TS declaration");

        // Process the definition.
        let definition = definition.strip_suffix(';').unwrap().trim().to_string();

        let name = name.strip_prefix("type").unwrap().trim().to_string();

        (Self::from_name_and_generics(name), definition)
    }

    pub fn from_type<T: TS + 'static + ?Sized>() -> Self {
        Self::from_name_and_generics(T::name())
    }

    fn from_name_and_generics(s: impl AsRef<str>) -> Self {
        let (name, generics) = match s.as_ref().split_once('<') {
            Some((name, generics)) => (
                name,
                // Extract the generics.
                generics
                    .rsplit_once('>')
                    .unwrap()
                    .0
                    .split(',')
                    .map(|generic| generic.trim().to_string())
                    .collect(),
            ),
            // No generics present in the definition.
            None => (s.as_ref(), Vec::new()),
        };

        Self {
            name: name.to_string(),
            generics,
        }
    }
}

impl Display for CodegenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;

        if !self.generics.is_empty() {
            write!(f, "<{}>", self.generics.join(", "))?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlerCodegen {
    kind: HandlerKind,
    params: Vec<(&'static str, CodegenType)>,
    return_ty: CodegenType,
}

impl HandlerCodegen {
    pub fn from_handler<F, Ctx, MSig, MValue, MReturn>(meta: &HandlerMeta, _handler: &F) -> Self
    where
        F: RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        MValue: marker::ResponseMarker,
        MReturn: marker::HandlerReturnMarker,
    {
        HandlerCodegen {
            kind: meta.kind,
            params: ParamVisitor::visit::<F::Params>(meta.param_names).unwrap(),
            return_ty: CodegenType::from_type::<<F::Response as ResponseValue<MValue>>::Value>(),
        }
    }
}

pub struct Codegen {
    pub dependent_types: DependentTypes,
    pub tree: Node,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            dependent_types: DependentTypes::new(),
            tree: Node::new(),
        }
    }

    pub fn generate<W: Write, B: Backend<W>>(&self, writer: &mut W) -> Result<(), std::fmt::Error> {
        B::begin(writer)?;

        for stage in B::STAGES {
            match stage {
                BackendStage::Handler => {
                    B::HandlerBackend::begin(writer)?;

                    fn write_node<W: Write, B: Backend<W>>(
                        node: &Node,
                        root: bool,
                        writer: &mut W,
                    ) -> Result<(), std::fmt::Error> {
                        B::HandlerBackend::begin_nested(root, writer)?;

                        // Write out all the handlers.
                        for (key, handler) in &node.handlers {
                            B::HandlerBackend::write_key(key, writer)?;
                            B::HandlerBackend::write_handler(handler, writer)?;
                        }

                        // Recurse and write nested nodes.
                        for (key, node) in &node.children {
                            B::HandlerBackend::write_key(key, writer)?;
                            write_node::<W, B>(node, false, writer)?;
                        }

                        B::HandlerBackend::end_nested(root, writer)?;

                        Ok(())
                    }

                    // Walk tree with recursion.
                    write_node::<W, B>(&self.tree, true, writer)?;

                    B::HandlerBackend::end(writer)?;
                }
                BackendStage::Type => {
                    B::TypeBackend::begin(writer)?;

                    for (name, definition) in self.dependent_types.definitions.values() {
                        B::TypeBackend::write_type(name, definition, writer)?;
                    }

                    B::TypeBackend::end(writer)?;
                }
            }
        }

        B::end(writer)?;

        Ok(())
    }
}

pub struct DependentTypes {
    visited_types: BTreeSet<TypeId>,
    definitions: BTreeMap<TypeId, (CodegenType, String)>,
}

impl DependentTypes {
    pub fn new() -> Self {
        Self {
            visited_types: BTreeSet::new(),
            definitions: BTreeMap::new(),
        }
    }
}

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

pub struct Node {
    handlers: BTreeMap<String, HandlerCodegen>,
    children: BTreeMap<String, Node>,
}

impl Node {
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
            children: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, path: &[&str], handler: &HandlerCodegen) {
        assert!(!path.is_empty());

        if path.len() == 1 {
            self.handlers.insert(path[0].to_string(), handler.clone());
            return;
        }

        self.children
            .entry(path[0].to_string())
            .or_insert_with(Node::new)
            .insert(&path[1..], handler);
    }
}

pub trait Backend<W: Write> {
    type HandlerBackend: HandlerBackend<W>;
    type TypeBackend: TypeBackend<W>;

    const STAGES: &[BackendStage];

    #[allow(unused)]
    fn begin(writer: &mut W) -> Result<(), std::fmt::Error> {
        Ok(())
    }

    #[allow(unused)]
    fn end(writer: &mut W) -> Result<(), std::fmt::Error> {
        Ok(())
    }
}

pub trait HandlerBackend<W: Write> {
    #[allow(unused)]
    fn begin(writer: &mut W) -> Result<(), std::fmt::Error> {
        Ok(())
    }

    #[allow(unused)]
    fn end(writer: &mut W) -> Result<(), std::fmt::Error> {
        Ok(())
    }

    fn write_key(key: &str, writer: &mut W) -> Result<(), std::fmt::Error>;
    fn write_handler(handler: &HandlerCodegen, writer: &mut W) -> Result<(), std::fmt::Error>;
    fn begin_nested(root: bool, writer: &mut W) -> Result<(), std::fmt::Error>;
    fn end_nested(root: bool, writer: &mut W) -> Result<(), std::fmt::Error>;
}

pub trait TypeBackend<W: Write> {
    #[allow(unused)]
    fn begin(writer: &mut W) -> Result<(), std::fmt::Error> {
        Ok(())
    }

    #[allow(unused)]
    fn end(writer: &mut W) -> Result<(), std::fmt::Error> {
        Ok(())
    }

    fn write_type(
        name: &CodegenType,
        definition: &str,
        writer: &mut W,
    ) -> Result<(), std::fmt::Error>;
}

pub enum BackendStage {
    Handler,
    Type,
}

#[cfg(test)]
mod test {
    use crate::__private::HandlerKind;

    use super::*;

    macro_rules! types {
        ($($ty:ty),* $(,)?) => {
            [$(TypeId::of::<$ty>()),*]
        };

        ($($ident:ident: $ty:ty),* $(,)?) => {
            [$((stringify!($ident), TypeId::of::<$ty>())),*]
        };
    }

    pub struct AssertBackend;

    impl<W: Write> Backend<W> for AssertBackend {
        type HandlerBackend = AssertHandlerBackend;
        type TypeBackend = AssertTypeBackend;

        const STAGES: &[BackendStage] = &[BackendStage::Handler, BackendStage::Type];
    }

    pub struct AssertHandlerBackend;
    impl<W: Write> HandlerBackend<W> for AssertHandlerBackend {
        fn write_key(key: &str, writer: &mut W) -> Result<(), std::fmt::Error> {
            todo!()
        }

        fn write_handler(handler: &HandlerCodegen, writer: &mut W) -> Result<(), std::fmt::Error> {
            todo!()
        }

        fn begin_nested(root: bool, writer: &mut W) -> Result<(), std::fmt::Error> {
            todo!()
        }

        fn end_nested(root: bool, writer: &mut W) -> Result<(), std::fmt::Error> {
            todo!()
        }
    }

    pub struct AssertTypeBackend;
    impl<W: Write> TypeBackend<W> for AssertTypeBackend {
        fn write_type(
            name: &CodegenType,
            definition: &str,
            writer: &mut W,
        ) -> Result<(), std::fmt::Error> {
            todo!()
        }
    }

    mod dependent_types {
        use std::{fmt::Debug, marker::PhantomData};

        use serde::{Deserialize, Serialize};

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
}
