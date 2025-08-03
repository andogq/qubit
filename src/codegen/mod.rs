mod handler;
mod prefix_map;
mod ts;

use std::{any::TypeId, collections::HashSet, fmt::Write};

use handler::{HandlerBuilder, ParamVisitor};
use petgraph::graph::{DiGraph, NodeIndex};
use ts_rs::{TS, TypeVisitor};

use crate::{
    __private::HandlerMeta,
    RegisterableHandler,
    handler::{marker, response::ResponseValue, ts::TsTypeTuple},
};

/// Collects handler and type definitions, and dispatches them to a [`Backend`] to generate the
/// final code. This will handle all book-keeping and tracking to prevent recursion and detect user
/// types, so the backend is safe to trust types that are dispatched to it.
struct Codegen<B: Backend> {
    /// Types that have been visited, tracked to prevent recursing on types.
    visited_types: HashSet<TypeId>,
    /// Backend that will output the generated code.
    backend: B,

    handlers: DiGraph<GraphNode<<B::HandlerBuilder as HandlerBuilder>::Output>, String>,
    handler_root: NodeIndex,

    types: Vec<B::UserType>,
}

#[derive(Clone)]
enum GraphNode<H> {
    Handler(H),
    Parent,
}

impl<B> Codegen<B>
where
    B: Backend,
{
    /// Create a new instance with the provided backend.
    pub fn new(backend: B) -> Self {
        let mut handlers = DiGraph::new();
        let handler_root = handlers.add_node(GraphNode::Parent);

        Self {
            visited_types: HashSet::new(),
            backend,
            handlers,
            handler_root,
            types: Vec::new(),
        }
    }

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
        self.backend.inspect_handler(&handler);

        let handler_node = self.handlers.add_node(GraphNode::Handler(handler));
        self.handlers
            .add_edge(self.handler_root, handler_node, meta.name.to_string());
    }

    pub fn nest(&mut self, prefix: impl ToString, other: Self) {
        // Pull out nodes and edges of the other graph.
        let (other_handlers, other_edges) = other.handlers.into_nodes_edges();

        // Add all the nodes, tracking the new index.
        let new_node_idx = other_handlers
            .into_iter()
            .map(|node| self.handlers.add_node(node.weight))
            .collect::<Vec<_>>();

        // Find the root and connect it to the existing graph.
        let other_root = new_node_idx[other.handler_root.index()];
        self.handlers
            .add_edge(self.handler_root, other_root, prefix.to_string());

        for edge in other_edges {
            let start = new_node_idx[edge.source().index()];
            let end = new_node_idx[edge.target().index()];

            self.handlers.add_edge(start, end, edge.weight);
        }
    }

    /// Generate a [`UserTypeVisitor`] with this codegen instance.
    fn user_type_visitor(&mut self) -> UserTypeVisitor<'_, B> {
        UserTypeVisitor(self)
    }
}

/// [`TypeVisitor`] which will traverse a type, find any user types, and register them against
/// the backend. It tracks the [`TypeId`] of all visited types (without their generics), in order
/// to prevent cycles.
struct UserTypeVisitor<'a, B: Backend>(&'a mut Codegen<B>);
impl<B> TypeVisitor for UserTypeVisitor<'_, B>
where
    B: Backend,
{
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        let type_id = TypeId::of::<T>();
        let type_id_no_generics = TypeId::of::<T::WithoutGenerics>();

        let visit_dependencies = !self.0.visited_types.contains(&type_id_no_generics);
        let visit_generics = !self.0.visited_types.contains(&type_id);

        // Don't bother processing if this type has already been captured.
        if visit_dependencies {
            self.0.visited_types.insert(type_id_no_generics);

            // Pass the type to the backend, if it's a user type.
            if T::output_path().is_some() {
                let ty = B::UserType::from_type::<T::WithoutGenerics>();
                self.0.backend.inspect_user_type(&ty);

                self.0.types.push(ty);
            }

            // Process dependent types
            T::visit_dependencies(self);
        }

        if visit_generics {
            self.0.visited_types.insert(type_id);

            // Process all generic typGes.
            T::visit_generics(self);
        }
    }
}

trait Backend {
    type UserType: FromType;
    type HandlerBuilder: HandlerBuilder;
    type HandlerWriter: HandlerWriter<<Self::HandlerBuilder as HandlerBuilder>::Output>;

    fn write(&self, writer: &mut impl Write) -> Result<Self::HandlerWriter, std::fmt::Error>;

    #[allow(unused)]
    fn inspect_user_type(&mut self, user_type: &Self::UserType) {}
    #[allow(unused)]
    fn inspect_handler(&mut self, handler: &<Self::HandlerBuilder as HandlerBuilder>::Output) {}
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
    use std::collections::VecDeque;

    use petgraph::{Direction, visit::EdgeRef};

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

    pub struct AssertBackend;

    #[derive(Debug, PartialEq)]
    pub struct AssertUserType(TypeId);
    impl FromType for AssertUserType {
        fn from_type<T: TS + 'static + ?Sized>() -> Self {
            Self(TypeId::of::<T>())
        }
    }

    impl Backend for AssertBackend {
        type UserType = AssertUserType;
        type HandlerBuilder = AssertHandlerBuilder;
        type HandlerWriter = AssertHandlerWriter;

        fn write(&self, _writer: &mut impl Write) -> Result<Self::HandlerWriter, std::fmt::Error> {
            unimplemented!()
        }
    }

    pub struct AssertHandlerWriter;
    impl HandlerWriter<AssertHandler> for AssertHandlerWriter {
        fn write_key(&mut self, _key: &str) -> Result<(), std::fmt::Error> {
            todo!()
        }

        fn write_handler(&mut self, _handler: AssertHandler) -> Result<(), std::fmt::Error> {
            todo!()
        }

        fn begin_nested(&mut self) -> Result<(), std::fmt::Error> {
            todo!()
        }

        fn end_nested(&mut self) -> Result<(), std::fmt::Error> {
            todo!()
        }
    }

    fn build_handlers(codegen: &Codegen<AssertBackend>) -> Vec<(String, AssertHandler)> {
        codegen
            .handlers
            .node_indices()
            .filter_map(|idx| match &codegen.handlers[idx] {
                GraphNode::Handler(handler) => Some((idx, handler)),
                GraphNode::Parent => None,
            })
            .map(|(mut idx, handler)| {
                let mut path = VecDeque::new();

                // Walk the node edges backwards to build the path.
                while idx != codegen.handler_root {
                    let mut edges = codegen.handlers.edges_directed(idx, Direction::Incoming);
                    let edge = edges.next().unwrap();
                    assert_eq!(edges.count(), 0);

                    idx = edge.source();
                    path.push_front(edge.weight().as_str());
                    path.push_front(".");
                }

                // Remove trailing separator.
                path.pop_front();

                (path.into_iter().collect(), handler.clone())
            })
            .collect()
    }

    mod register_handler {
        use std::marker::PhantomData;

        use serde::{Deserialize, Serialize};

        use super::*;

        fn assert_root_handlers(
            codegen: Codegen<AssertBackend>,
            expected_handlers: impl AsRef<[AssertHandler]>,
        ) {
            let handlers = codegen
                .handlers
                .node_weights()
                .filter_map(|node| match node {
                    GraphNode::Handler(handler) => Some(handler.clone()),
                    GraphNode::Parent => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(handlers, expected_handlers.as_ref());
        }

        #[test]
        fn empty_handler() {
            let mut codegen = Codegen::new(AssertBackend);

            codegen.register_handler::<_, (), _, _, _>(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &[],
                },
                &|| {},
            );

            assert_eq!(codegen.types, []);
            assert_eq!(
                build_handlers(&codegen),
                [(
                    "some_handler".to_string(),
                    AssertHandler {
                        name: "some_handler",
                        kind: HandlerKind::Query,
                        params: types![].to_vec(),
                        return_ty: TypeId::of::<()>(),
                    }
                )],
            );
        }

        #[test]
        fn multiple_parameters() {
            let mut codegen = Codegen::new(AssertBackend);

            codegen.register_handler(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &["param_a", "param_b", "param_c"],
                },
                #[allow(unused)]
                &|ctx: (), param_a: u32, param_b: bool, param_c: String| {},
            );

            assert_eq!(codegen.types, []);
            assert_eq!(
                build_handlers(&codegen),
                [(
                    "some_handler".to_string(),
                    AssertHandler {
                        name: "some_handler",
                        kind: HandlerKind::Query,
                        params: types![
                            param_a: u32,
                            param_b: bool,
                            param_c: String,
                        ]
                        .to_vec(),
                        return_ty: TypeId::of::<()>(),
                    }
                )]
            );
        }

        #[test]
        fn return_ty() {
            let mut codegen = Codegen::new(AssertBackend);

            codegen.register_handler::<_, (), _, _, _>(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &[],
                },
                #[allow(unused)]
                &|| -> Vec<u32> { todo!() },
            );

            assert_eq!(codegen.types, []);
            assert_eq!(
                build_handlers(&codegen),
                [(
                    "some_handler".to_string(),
                    AssertHandler {
                        name: "some_handler",
                        kind: HandlerKind::Query,
                        params: types![].to_vec(),
                        return_ty: TypeId::of::<Vec<u32>>(),
                    }
                )],
            );
        }

        #[test]
        fn custom_tys() {
            #[derive(TS, Clone, Deserialize)]
            struct TypeA;
            #[derive(TS, Clone, Serialize)]
            struct TypeB;

            let mut codegen = Codegen::new(AssertBackend);

            codegen.register_handler(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &["param"],
                },
                #[allow(unused)]
                &|ctx: (), param: TypeA| -> TypeB { todo!() },
            );

            assert_eq!(codegen.types, types![TypeA, TypeB].map(AssertUserType));
            assert_eq!(
                build_handlers(&codegen),
                [(
                    "some_handler".to_string(),
                    AssertHandler {
                        name: "some_handler",
                        kind: HandlerKind::Query,
                        params: types![
                            param: TypeA,
                        ]
                        .to_vec(),
                        return_ty: TypeId::of::<TypeB>(),
                    }
                )],
            );
        }

        #[test]
        fn custom_tys_in_generic() {
            #[derive(TS, Clone, Deserialize)]
            struct TypeA;
            #[derive(TS, Clone, Serialize)]
            struct TypeB;

            let mut codegen = Codegen::new(AssertBackend);

            codegen.register_handler(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &["param"],
                },
                #[allow(unused)]
                &|ctx: (), param: Option<TypeA>| -> Option<TypeB> { todo!() },
            );

            assert_eq!(codegen.types, types![TypeA, TypeB].map(AssertUserType));
            assert_eq!(
                build_handlers(&codegen),
                [(
                    "some_handler".to_string(),
                    AssertHandler {
                        name: "some_handler",
                        kind: HandlerKind::Query,
                        params: types![
                            param: Option<TypeA>,
                        ]
                        .to_vec(),
                        return_ty: TypeId::of::<Option<TypeB>>(),
                    }
                )],
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

            let mut codegen = Codegen::new(AssertBackend);

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
                codegen.types,
                types![TypeA<ts_rs::Dummy>, InnerA, TypeB<ts_rs::Dummy>, InnerB]
                    .map(AssertUserType)
            );
            assert_eq!(
                build_handlers(&codegen),
                [(
                    "some_handler".to_string(),
                    AssertHandler {
                        name: "some_handler",
                        kind: HandlerKind::Query,
                        params: types![
                            param: TypeA<InnerA>,
                        ]
                        .to_vec(),
                        return_ty: TypeId::of::<TypeB<InnerB>>(),
                    }
                )],
            );
        }
    }

    mod nest {
        use super::*;

        #[test]
        fn nest_something() {
            let mut codegen = Codegen::new(AssertBackend);
            codegen.register_handler(
                &HandlerMeta {
                    kind: HandlerKind::Query,
                    name: "some_handler",
                    param_names: &[],
                },
                #[allow(unused)]
                &|ctx: ()| {},
            );
            codegen.nest("nested", {
                let mut codegen = Codegen::new(AssertBackend);
                codegen.register_handler(
                    &HandlerMeta {
                        kind: HandlerKind::Query,
                        name: "other_handler",
                        param_names: &[],
                    },
                    #[allow(unused)]
                    &|ctx: ()| {},
                );
                codegen
            });

            assert_eq!(
                build_handlers(&codegen),
                [
                    (
                        "some_handler".to_string(),
                        AssertHandler {
                            name: "some_handler",
                            kind: HandlerKind::Query,
                            params: Vec::new(),
                            return_ty: TypeId::of::<()>()
                        }
                    ),
                    (
                        "nested.other_handler".to_string(),
                        AssertHandler {
                            name: "other_handler",
                            kind: HandlerKind::Query,
                            params: Vec::new(),
                            return_ty: TypeId::of::<()>()
                        }
                    )
                ]
            )
        }
    }

    mod user_type_visitor {
        #![allow(unused)]

        use super::*;

        fn visit<T: TS + 'static + ?Sized>() -> Vec<TypeId> {
            let mut codegen = Codegen::new(AssertBackend);

            let mut visitor = codegen.user_type_visitor();
            visitor.visit::<T>();

            codegen.types.iter().map(|ty| ty.0).collect()
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

            let mut codegen = Codegen::new(AssertBackend);

            let mut visitor = codegen.user_type_visitor();

            visitor.visit::<Vec<TypeA>>();
            visitor.visit::<Vec<TypeB>>();

            assert_eq!(codegen.types, types![TypeA, TypeB].map(AssertUserType));
        }

        #[test]
        fn optional_ty() {
            #[derive(TS)]
            struct TypeA;

            assert_eq!(visit::<Option<TypeA>>(), types![TypeA]);
        }
    }
}
