mod handler;
mod prefix_map;
mod ts;

use std::{any::TypeId, collections::HashSet, fmt::Write};

use handler::{HandlerBuilder, ParamVisitor};
use ts_rs::{TS, TypeVisitor};

use crate::{
    __private::HandlerMeta,
    RegisterableHandler,
    handler::{marker, response::ResponseValue, ts::TsTypeTuple},
};

/// Collects handler and type definitions, and dispatches them to a [`Backend`] to generate the
/// final code. This will handle all book-keeping and tracking to prevent recursion and detect user
/// types, so the backend is safe to trust types that are dispatched to it.
struct Codegen<B> {
    /// Types that have been visited, tracked to prevent recursing on types.
    visited_types: HashSet<TypeId>,
    /// Backend that will output the generated code.
    backend: B,
}

impl<B> Codegen<B> {
    /// Create a new instance with the provided backend.
    pub fn new(backend: B) -> Self {
        Self {
            visited_types: HashSet::new(),
            backend,
        }
    }
}

impl<B> Codegen<B>
where
    B: Backend,
{
    /// Register a handler definition.
    pub fn register_handler<F, Ctx, MSig, MValue, MReturn>(
        &mut self,
        meta: &HandlerMeta,
        _handler: &F,
    ) where
        F: RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        MValue: marker::ResponseMarker,
        MReturn: marker::HandlerReturnMarker,
    {
        let mut visitor = self.user_type_visitor();

        // Register all associated types.
        <F::Params as TsTypeTuple>::visit_tys(&mut visitor);
        visitor.visit::<<F::Response as ResponseValue<MValue>>::Value>();

        // Begin the handler definition.
        let handler = B::HandlerBuilder::new(meta.name, meta.kind);
        // Add parameters.
        let handler = ParamVisitor::visit::<F::Params>(handler, meta.param_names).unwrap();
        // Set the return type.
        let handler = handler.returning::<<F::Response as ResponseValue<MValue>>::Value>();

        // Pass the handler on to the backend.
        self.backend.register_handler(handler);
    }

    /// Generate a [`UserTypeVisitor`] with this codegen instance.
    fn user_type_visitor(&mut self) -> UserTypeVisitor<'_, B> {
        UserTypeVisitor(self)
    }
}

/// [`TypeVisitor`] which will traverse a type, find any user types, and register them against
/// the backend. It tracks the [`TypeId`] of all visited types (without their generics), in order
/// to prevent cycles.
struct UserTypeVisitor<'a, B>(&'a mut Codegen<B>);
impl<B> TypeVisitor for UserTypeVisitor<'_, B>
where
    B: Backend,
{
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        let type_id = TypeId::of::<T::WithoutGenerics>();

        // Don't bother processing if this type has already been captured.
        if !self.0.visited_types.contains(&type_id) {
            self.0.visited_types.insert(type_id);

            // Pass the type to the backend, if it's a user type.
            if T::output_path().is_some() {
                self.0.backend.register_user_type::<T::WithoutGenerics>();
            }
        }

        // Process all dependent and generic typGes.
        T::visit_dependencies(self);
        T::visit_generics(self);
    }
}

trait Backend {
    /// Type of the handler this backend will generate.
    type HandlerBuilder: HandlerBuilder;

    /// File extension the backend produces.
    const FILE_EXTENSION: &'static str;

    /// Register a type to the backend. This will be a user type, so this can be used to track type
    /// declarations.
    fn register_user_type<T: TS + 'static + ?Sized>(&mut self);

    /// Register a handler to the backend. All user types within the handler will already have been
    /// registered with [`Backend::register_user_type`].
    fn register_handler(&mut self, handler: <Self::HandlerBuilder as HandlerBuilder>::Output);

    /// Generate the code into the provided writer. The provided header must be prepended to the
    /// output, normally in a comment block.
    fn codegen(&self, header: &'static str, writer: impl Write) -> Result<(), std::fmt::Error>;
}

type Tree<'a, T> = &'a [(&'a str, TreeItem<'a, T>)];
enum TreeItem<'a, T> {
    Item(T),
    Nested(Tree<'a, T>),
}

trait Backend2 {
    type UserType;
    type HandlerBuilder: HandlerBuilder;

    fn user_type<T: TS + 'static + ?Sized>(&mut self) -> Self::UserType;
    fn register_handler(&mut self, handler: &<Self::HandlerBuilder as HandlerBuilder>::Output);

    fn write_handler_tree(
        &self,
        writer: &mut impl Write,
        tree: Tree<'_, <Self::HandlerBuilder as HandlerBuilder>::Output>,
    ) -> Result<(), std::fmt::Error>;
}

trait Backend3 {
    type UserType: FromType;
    type HandlerBuilder: HandlerBuilder;
    type HandlerWriter: HandlerWriter<<Self::HandlerBuilder as HandlerBuilder>::Output>;

    fn write(&self, writer: &mut impl Write) -> Result<Self::HandlerWriter, std::fmt::Error>;

    #[allow(unused)]
    fn inspect_user_type(&mut self, user_type: &Self::UserType) {}
    #[allow(unused)]
    fn inspect_handler(&mut self, handler: <Self::HandlerBuilder as HandlerBuilder>::Output) {}
}

trait HandlerWriter<H> {
    fn write_key(&mut self, key: &str) -> Result<(), std::fmt::Error>;
    fn write_handler(&mut self, handler: H) -> Result<(), std::fmt::Error>;
    fn begin_nested(&mut self) -> Result<(), std::fmt::Error>;
    fn end_nested(&mut self) -> Result<(), std::fmt::Error>;
}

trait FromType {
    fn from_type<T: TS + 'static + ?Sized>() -> Self;
}

#[cfg(test)]
mod test {
    use crate::__private::HandlerKind;

    use super::{
        handler::test::{AssertHandler, AssertHandlerBuilder},
        *,
    };

    macro_rules! types {
        ($($ty:ty),* $(,)?) => {
            [$(TypeId::of::<$ty>()),*]
        };

        ($($ident:ident: $ty:ty),* $(,)?) => {
            [$((stringify!($ident), TypeId::of::<$ty>())),*]
        };
    }

    mod backend2 {
        use super::*;

        struct JsonWriter;
        impl Backend2 for JsonWriter {
            type UserType = ();

            type HandlerBuilder = TestHandler;

            fn user_type<T: TS + 'static + ?Sized>(&mut self) -> Self::UserType {
                todo!()
            }

            fn register_handler(
                &mut self,
                handler: &<Self::HandlerBuilder as HandlerBuilder>::Output,
            ) {
                todo!()
            }

            fn write_handler_tree(
                &self,
                writer: &mut impl Write,
                tree: Tree<'_, <Self::HandlerBuilder as HandlerBuilder>::Output>,
            ) -> Result<(), std::fmt::Error> {
                write!(writer, "{{ ")?;
                for (key, item) in tree {
                    write!(writer, r#""{key}": "#)?;

                    match item {
                        TreeItem::Item(handler) => write!(writer, r#""{}""#, handler.0)?,
                        TreeItem::Nested(tree) => self.write_handler_tree(writer, tree)?,
                    }

                    write!(writer, ", ")?;
                }
                write!(writer, "}}")?;

                Ok(())
            }
        }

        struct TestHandler(&'static str);
        impl HandlerBuilder for TestHandler {
            type Output = TestHandler;

            fn new(name: &'static str, kind: HandlerKind) -> Self {
                todo!()
            }

            fn push_param<T: TS + 'static + ?Sized>(&mut self, param_name: &'static str) {
                todo!()
            }

            fn returning<T: TS + 'static + ?Sized>(self) -> Self::Output {
                todo!()
            }
        }

        #[test]
        fn it_works() {
            let mut output = String::new();
            JsonWriter
                .write_handler_tree(
                    &mut output,
                    &[("something", TreeItem::Item(TestHandler("hello")))],
                )
                .unwrap();
            assert_eq!(output, r#"{ "something": "hello", }"#);

            let mut output = String::new();
            JsonWriter
                .write_handler_tree(
                    &mut output,
                    &[
                        ("something", TreeItem::Item(TestHandler("hello"))),
                        (
                            "nested",
                            TreeItem::Nested(&[
                                ("first_level", TreeItem::Item(TestHandler("ok"))),
                                (
                                    "deep",
                                    TreeItem::Nested(&[(
                                        "super_deep",
                                        TreeItem::Item(TestHandler("yep")),
                                    )]),
                                ),
                            ]),
                        ),
                    ],
                )
                .unwrap();
            assert_eq!(
                output,
                r#"{ "something": "hello", "nested": { "first_level": "ok", "deep": { "super_deep": "yep", }, }, }"#
            );
        }
    }

    #[derive(Default)]
    pub struct AssertBackend {
        pub types: Vec<TypeId>,
        pub handlers: Vec<AssertHandler>,
    }

    impl Backend for AssertBackend {
        type HandlerBuilder = AssertHandlerBuilder;

        const FILE_EXTENSION: &'static str = "something";

        fn register_user_type<T: TS + 'static + ?Sized>(&mut self) {
            self.types.push(TypeId::of::<T>());
        }

        fn register_handler(&mut self, handler: <Self::HandlerBuilder as HandlerBuilder>::Output) {
            self.handlers.push(handler);
        }

        fn codegen(
            &self,
            _header: &'static str,
            _writer: impl Write,
        ) -> Result<(), std::fmt::Error> {
            unimplemented!()
        }
    }

    mod register_handler {
        use std::marker::PhantomData;

        use serde::{Deserialize, Serialize};

        use super::*;

        #[test]
        fn empty_handler() {
            let mut codegen = Codegen::new(AssertBackend::default());

            codegen.register_handler::<_, (), _, _, _>(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &[],
                },
                &|| {},
            );

            assert_eq!(codegen.backend.types, []);
            assert_eq!(
                codegen.backend.handlers,
                [AssertHandler {
                    name: "some_handler",
                    kind: HandlerKind::Query,
                    params: types![].to_vec(),
                    return_ty: TypeId::of::<()>()
                }]
            );
        }

        #[test]
        fn multiple_parameters() {
            let mut codegen = Codegen::new(AssertBackend::default());

            codegen.register_handler(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &["param_a", "param_b", "param_c"],
                },
                #[allow(unused)]
                &|ctx: (), param_a: u32, param_b: bool, param_c: String| {},
            );

            assert_eq!(codegen.backend.types, []);
            assert_eq!(
                codegen.backend.handlers,
                [AssertHandler {
                    name: "some_handler",
                    kind: HandlerKind::Query,
                    params: types![
                        param_a: u32,
                        param_b: bool,
                        param_c: String,
                    ]
                    .to_vec(),
                    return_ty: TypeId::of::<()>()
                }]
            );
        }

        #[test]
        fn return_ty() {
            let mut codegen = Codegen::new(AssertBackend::default());

            codegen.register_handler::<_, (), _, _, _>(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &[],
                },
                #[allow(unused)]
                &|| -> Vec<u32> { todo!() },
            );

            assert_eq!(codegen.backend.types, []);
            assert_eq!(
                codegen.backend.handlers,
                [AssertHandler {
                    name: "some_handler",
                    kind: HandlerKind::Query,
                    params: types![].to_vec(),
                    return_ty: TypeId::of::<Vec<u32>>()
                }]
            );
        }

        #[test]
        fn custom_tys() {
            #[derive(TS, Clone, Deserialize)]
            struct TypeA;
            #[derive(TS, Clone, Serialize)]
            struct TypeB;

            let mut codegen = Codegen::new(AssertBackend::default());

            codegen.register_handler(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &["param"],
                },
                #[allow(unused)]
                &|ctx: (), param: TypeA| -> TypeB { todo!() },
            );

            assert_eq!(
                codegen.backend.types,
                [TypeId::of::<TypeA>(), TypeId::of::<TypeB>()]
            );
            assert_eq!(
                codegen.backend.handlers,
                [AssertHandler {
                    name: "some_handler",
                    kind: HandlerKind::Query,
                    params: types![
                        param: TypeA,
                    ]
                    .to_vec(),
                    return_ty: TypeId::of::<TypeB>()
                }]
            );
        }

        #[test]
        fn custom_tys_in_generic() {
            #[derive(TS, Clone, Deserialize)]
            struct TypeA;
            #[derive(TS, Clone, Serialize)]
            struct TypeB;

            let mut codegen = Codegen::new(AssertBackend::default());

            codegen.register_handler(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &["param"],
                },
                #[allow(unused)]
                &|ctx: (), param: Option<TypeA>| -> Option<TypeB> { todo!() },
            );

            assert_eq!(
                codegen.backend.types,
                [TypeId::of::<TypeA>(), TypeId::of::<TypeB>()]
            );
            assert_eq!(
                codegen.backend.handlers,
                [AssertHandler {
                    name: "some_handler",
                    kind: HandlerKind::Query,
                    params: types![
                        param: Option<TypeA>,
                    ]
                    .to_vec(),
                    return_ty: TypeId::of::<Option<TypeB>>()
                }]
            );
        }

        #[test]
        fn custom_tys_with_generic() {
            #[derive(TS, Clone, Deserialize)]
            struct TypeA<T>(PhantomData<T>);
            #[derive(TS, Clone, Deserialize)]
            struct InnerA;
            #[derive(TS, Clone, Serialize)]
            struct TypeB<T>(PhantomData<T>);
            #[derive(TS, Clone, Serialize)]
            struct InnerB;

            let mut codegen = Codegen::new(AssertBackend::default());

            codegen.register_handler(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &["param"],
                },
                #[allow(unused)]
                &|ctx: (), param: TypeA<InnerA>| -> TypeB<InnerB> { todo!() },
            );

            assert_eq!(
                codegen.backend.types,
                [
                    TypeId::of::<TypeA<ts_rs::Dummy>>(),
                    TypeId::of::<InnerA>(),
                    TypeId::of::<TypeB<ts_rs::Dummy>>(),
                    TypeId::of::<InnerB>(),
                ]
            );

            assert_eq!(
                codegen.backend.handlers,
                [AssertHandler {
                    name: "some_handler",
                    kind: HandlerKind::Query,
                    params: types![
                        param: TypeA<InnerA>,
                    ]
                    .to_vec(),
                    return_ty: TypeId::of::<TypeB<InnerB>>()
                }]
            );
        }
    }

    mod user_type_visitor {
        #![allow(unused)]

        use super::*;

        fn visit<T: TS + 'static + ?Sized>() -> Vec<TypeId> {
            let mut codegen = Codegen::new(AssertBackend::default());

            let mut visitor = codegen.user_type_visitor();
            visitor.visit::<T>();

            codegen.backend.types
        }

        #[test]
        fn unit() {
            assert_eq!(visit::<()>(), types![]);
        }

        #[test]
        fn primitive() {
            assert_eq!(visit::<u32>(), types![]);
        }

        #[test]
        fn unit_struct() {
            #[derive(TS)]
            struct MyType;
            assert_eq!(visit::<MyType>(), types![MyType]);
        }

        #[test]
        fn primitive_field_struct() {
            #[derive(TS)]
            struct MyType {
                a: u32,
                b: bool,
            }
            assert_eq!(visit::<MyType>(), types![MyType]);
        }

        #[test]
        fn simple_enum() {
            #[derive(TS)]
            enum MyType {
                A,
                B,
            }
            assert_eq!(visit::<MyType>(), types![MyType]);
        }

        #[test]
        fn primitive_tuple_enum() {
            #[derive(TS)]
            enum MyType {
                A(u32),
                B(bool),
            }
            assert_eq!(visit::<MyType>(), types![MyType]);
        }

        #[test]
        fn primitive_struct_enum() {
            #[derive(TS)]
            enum MyType {
                A { field_a: u32 },
                B { field_b: bool },
            }
            assert_eq!(visit::<MyType>(), types![MyType]);
        }

        #[test]
        fn primitive_generic() {
            #[derive(TS)]
            struct MyType;
            assert_eq!(visit::<Option<Box<MyType>>>(), types![MyType]);
        }

        #[test]
        fn recursive_struct() {
            #[derive(TS)]
            struct MyType {
                nested: Box<MyType>,
            }
            assert_eq!(visit::<MyType>(), types![MyType]);
        }

        #[test]
        fn nested_user_types() {
            #[derive(TS)]
            struct MyType {
                nested: MyNested,
            }
            #[derive(TS)]
            struct MyNested;
            assert_eq!(visit::<MyType>(), types![MyType, MyNested]);
        }

        #[test]
        fn deeply_nested_user_types() {
            #[derive(TS)]
            struct MyType {
                nested: MyNested,
            }
            #[derive(TS)]
            struct MyNested {
                super_nested: MySuperNested,
            }
            #[derive(TS)]
            struct MySuperNested;
            assert_eq!(visit::<MyType>(), types![MyType, MyNested, MySuperNested]);
        }

        #[test]
        fn generic_user_type() {
            #[derive(TS)]
            struct MyType<T: TS>(T);
            #[derive(TS)]
            struct MyOtherType;
            assert_eq!(
                visit::<MyType<MyOtherType>>(),
                types![MyType<ts_rs::Dummy>, MyOtherType]
            );
        }

        #[test]
        fn duplicated_generic_user_type() {
            #[derive(TS)]
            struct MyType<T: TS>(T);
            #[derive(TS)]
            struct MyOtherType;
            assert_eq!(
                visit::<MyType<MyType<MyOtherType>>>(),
                types![MyType<ts_rs::Dummy>, MyOtherType,]
            );
        }

        #[test]
        fn different_generics() {
            #[derive(TS)]
            struct TypeA;
            #[derive(TS)]
            struct TypeB;

            let mut codegen = Codegen::new(AssertBackend::default());

            let mut visitor = codegen.user_type_visitor();

            visitor.visit::<Vec<TypeA>>();
            visitor.visit::<Vec<TypeB>>();

            assert_eq!(codegen.backend.types, types![TypeA, TypeB]);
        }
    }
}
