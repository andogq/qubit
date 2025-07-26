use crate::handler::{
    RegisterableHandler, marker,
    reflection::{HandlerKind, HandlerMeta},
    response::ResponseValue,
    ts::TsType,
    ts::TsTypeTuple,
};
use std::{any::TypeId, collections::BTreeMap, fmt::Write};

/// TypeScript representation of a [`Router`], containing all required information to generate
/// TypeScript types at runtime.
///
/// `Router`: super::router::Router
#[derive(Clone, Debug, Default)]
pub struct TsRouter {
    /// All user-generated types that must be included.
    user_types: BTreeMap<TypeId, String>,
    /// Router handlers declarations, keyed by their name.
    handlers: BTreeMap<String, String>,
    /// Nested routers, and the prefix they're registered at.
    nested: BTreeMap<String, TsRouter>,
}

impl TsRouter {
    /// Create a new router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new handler to the router, generating the required TypeScript types in the process.
    pub fn add_handler<
        F,
        Ctx,
        MSig,
        MValue: marker::ResponseMarker,
        MReturn: marker::HandlerReturnMarker,
    >(
        &mut self,
        meta: &HandlerMeta,
        _handler: &F,
    ) where
        F: RegisterableHandler<Ctx, MSig, MValue, MReturn>,
    {
        let param_tys = F::Params::get_ts_types();
        let ret_ty = TsType::from_type::<<F::Response as ResponseValue<_>>::Value>();

        assert_eq!(
            param_tys.len(),
            meta.param_names.len(),
            "param types and provided names must be equal length"
        );

        // Register all dependent types of this handler.
        param_tys.iter().chain([&ret_ty]).for_each(|param| {
            let TsType::User(ty) = param else {
                return;
            };
            self.user_types.insert(ty.id, ty.declaration.clone());
        });

        // Generate the signature of this handler.
        let kind = match meta.kind {
            HandlerKind::Query => "Query",
            HandlerKind::Mutation => "Mutation",
            HandlerKind::Subscription => "Subscription",
        };
        let params = meta
            .param_names
            .iter()
            .zip(&param_tys)
            .flat_map(|(name, ty)| [format!("{}: {}", name, ty.name), ", ".to_string()])
            .take(if meta.param_names.is_empty() {
                0
            } else {
                meta.param_names.len() * 2 - 1
            })
            .collect::<String>();
        self.handlers.insert(
            meta.name.to_string(),
            format!("{kind}<[{params}], {}>", ret_ty.name),
        );
    }

    /// Nest another router at a prefix.
    pub fn nest(&mut self, prefix: impl ToString, other: TsRouter) {
        self.nested.insert(prefix.to_string(), other);
    }

    /// Write the router type to the provided writer.
    fn write_router_type(&self, fmt: &mut impl Write) -> std::fmt::Result {
        write!(fmt, "{{ ")?;

        for (prefix, router) in &self.nested {
            write!(fmt, "{prefix}: ")?;
            router.write_router_type(fmt)?;
            write!(fmt, ", ")?;
        }

        for (key, handler) in &self.handlers {
            write!(fmt, "{key}: {handler}, ")?;
        }

        write!(fmt, "}}")?;

        Ok(())
    }

    /// Generate an entire TypeScript file for this router, including all user type definitions.
    pub fn generate_typescript(&self) -> String {
        let user_types = {
            let mut user_types = BTreeMap::new();
            self.copy_user_types(&mut user_types);
            user_types
        };

        let mut typescript = String::new();

        for user_type in user_types.values() {
            writeln!(typescript, "{user_type}").unwrap();
        }

        write!(typescript, "export type QubitServer = ").unwrap();
        self.write_router_type(&mut typescript).unwrap();
        writeln!(typescript, ";").unwrap();

        typescript
    }

    /// Copy user types into the provided [`BTreeMap`].
    fn copy_user_types(&self, user_types: &mut BTreeMap<TypeId, String>) {
        user_types.extend(self.user_types.clone());

        for router in self.nested.values() {
            router.copy_user_types(user_types);
        }
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;
    use serde::{Deserialize, Serialize};
    use ts_rs::TS;

    use super::*;

    mod single_handlers {
        #![allow(unused)]

        use super::*;

        #[derive(Clone, TS, Deserialize, Serialize)]
        struct CustomStruct;

        #[rstest]
        #[case::empty(&[], || {}, "Query<[], null>")]
        #[case::ctx(&[], |ctx: ()| {}, "Query<[], null>")]
        #[case::param(&["a"], |ctx: (), a: usize| {}, "Query<[a: number], null>")]
        #[case::multi_param(&["a", "b"], |ctx: (), a: usize, b: bool| {}, "Query<[a: number, b: boolean], null>")]
        #[case::ret(&[], || -> usize { todo!() }, "Query<[], number>")]
        #[case::ret_ctx(&[], |ctx: ()| -> usize { todo!() }, "Query<[], number>")]
        #[case::custom_param(&["custom"], |ctx: (), custom: CustomStruct| {}, "Query<[custom: CustomStruct], null>")]
        #[case::custom_ret(&[], |ctx: ()| -> CustomStruct { todo!() }, "Query<[], CustomStruct>")]
        #[case::complex_response(&[], |ctx: ()| async { CustomStruct }, "Query<[], CustomStruct>")]
        #[case::everything(&["a", "b", "custom"], |ctx: (), a: usize, b: String, custom: CustomStruct| async { CustomStruct }, "Query<[a: number, b: string, custom: CustomStruct], CustomStruct>")]
        fn test<
            F,
            Ctx,
            MSig,
            MValue: marker::ResponseMarker,
            MReturn: marker::HandlerReturnMarker,
        >(
            #[case] param_names: &'static [&'static str],
            #[case] handler: F,
            #[case] expected: String,
        ) where
            F: RegisterableHandler<Ctx, MSig, MValue, MReturn, Ctx = ()>,
        {
            let mut module = TsRouter::new();
            module.add_handler(
                &HandlerMeta {
                    name: "handler",
                    param_names,
                    kind: HandlerKind::Query,
                },
                &handler,
            );

            let signature = module.handlers.get("handler").unwrap();
            assert_eq!(signature, &expected);
        }
    }

    #[derive(Clone, TS, Deserialize, Serialize)]
    struct UserTypeA {
        a: usize,
        b: bool,
    }

    #[derive(Clone, TS, Deserialize, Serialize)]
    struct UserTypeB(String);

    #[test]
    #[allow(unused)]
    fn user_types() {
        let mut module = TsRouter::new();
        module.add_handler(
            &HandlerMeta {
                name: "handler",
                param_names: &["a"],
                kind: HandlerKind::Query,
            },
            &|ctx: (), a: UserTypeA| -> UserTypeB { todo!() },
        );

        let signature = module.handlers.get("handler").unwrap();
        assert_eq!(signature, "Query<[a: UserTypeA], UserTypeB>");

        assert_eq!(module.user_types.len(), 2);
        assert_eq!(
            module.user_types.get(&TypeId::of::<UserTypeA>()).unwrap(),
            "type UserTypeA = { a: number, b: boolean, };"
        );
        assert_eq!(
            module.user_types.get(&TypeId::of::<UserTypeB>()).unwrap(),
            "type UserTypeB = string;"
        );
    }

    /// Helper to make a router with the named handlers inside.
    fn make_router(handlers: impl IntoIterator<Item = &'static str>) -> TsRouter {
        let mut router = TsRouter::new();
        for handler in handlers {
            router.add_handler::<_, (), _, _, _>(
                &HandlerMeta {
                    name: handler,
                    param_names: &[],
                    kind: HandlerKind::Query,
                },
                &|| {},
            );
        }
        router
    }

    #[rstest]
    #[case::empty(make_router([]), "{ }")]
    #[case::shallow(make_router(["handler"]), "{ handler: Query<[], null>, }")]
    #[case::multi(make_router(["handler_a", "handler_b"]), "{ handler_a: Query<[], null>, handler_b: Query<[], null>, }")]
    #[case::deep(
        {
            let mut module = make_router(["top"]);
            module.nest("layer_2", {
                let mut layer_2 = make_router(["middle"]);
                layer_2.nest("layer_3", make_router(["inner"]));
                layer_2
            });
            module
        },
        "{ layer_2: { layer_3: { inner: Query<[], null>, }, middle: Query<[], null>, }, top: Query<[], null>, }"
    )]
    fn nested(#[case] router: TsRouter, #[case] expected: &str) {
        let mut ty = String::new();
        router.write_router_type(&mut ty).unwrap();
        assert_eq!(ty, expected);
    }

    #[test]
    fn complex() {
        #![allow(unused)]

        let mut router = TsRouter::new();
        router.add_handler(
            &HandlerMeta {
                name: "outer",
                param_names: &["user_type"],
                kind: HandlerKind::Query,
            },
            &|ctx: (), user_type: UserTypeA| {},
        );
        router.nest("nested", {
            let mut router = TsRouter::new();
            router.add_handler(
                &HandlerMeta {
                    name: "inner",
                    param_names: &["user_type"],
                    kind: HandlerKind::Query,
                },
                &|ctx: (), user_type: UserTypeB| {},
            );
            router
        });

        assert_eq!(
            router.generate_typescript(),
            r#"type UserTypeB = string;
type UserTypeA = { a: number, b: boolean, };
export type QubitServer = { nested: { inner: Query<[user_type: UserTypeB], null>, }, outer: Query<[user_type: UserTypeA], null>, };
"#
        );
    }
}
