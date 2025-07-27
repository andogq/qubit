#![allow(unused_variables)]

use qubit::*;

macro_rules! test_handler {
    ($handler:ident = $kind:ident<[$($params:tt)*], $ret:tt>) => {
        test_handler!($handler <()> ($handler) = $kind<[$($params)*], $ret>);
    };

    ($handler:ident ($handler_name:ident) = $kind:ident<[$($params:tt)*], $ret:tt>) => {
        test_handler!($handler <()> ($handler_name) = $kind<[$($params)*], $ret>);
    };

    ($handler:ident <$ctx:ty> = $kind:ident<[$($params:tt)*], $ret:tt>) => {
        test_handler!($handler <$ctx> ($handler) = $kind<[$($params)*], $ret>);
    };

    ($handler:ident <$ctx:ty> ($handler_name:ident) = $kind:ident<[$($params:tt)*], $ret:tt>) => {
        let ty = Router::<$ctx>::new().handler($handler).generate_type_to_string();

        assert_eq!(
            ty,
            format!(
                "export type QubitServer = {{ {handler_name}: {kind}<[{params}], {ret}>, }};\n",
                handler_name = stringify!($handler_name),
                kind = stringify!($kind),
                params = stringify!($($params)*),
                ret = stringify!($ret),
            )
        );
    };
}

#[test]
fn empty_handler() {
    #[handler(query)]
    fn handler() {}

    test_handler!(handler = Query<[], null>);
}

#[test]
fn rename_empty_handler() {
    #[handler(query, name = "some_name")]
    fn handler() {}

    test_handler!(handler(some_name) = Query<[], null>);
}

#[test]
fn empty_mutation_handler() {
    #[handler(mutation)]
    fn handler() {}

    test_handler!(handler = Mutation<[], null>);
}

#[test]
fn unit_ctx_parameter() {
    #[handler(query)]
    fn handler(ctx: ()) {}

    test_handler!(handler = Query<[], null>);
}

#[test]
fn user_ctx_parameter() {
    #[derive(Clone)]
    struct Ctx;

    #[handler(query)]
    fn handler(ctx: Ctx) {}

    test_handler!(handler<Ctx> = Query<[], null>);
}

#[test]
fn primitive_parameters() {
    #[handler(query)]
    fn handler(ctx: (), a: i32, b: String, c: Option<bool>) {}

    test_handler!(handler = Query<[a: number, b: string, c: boolean | null], null>);
}

#[test]
fn primitive_return() {
    #[handler(query)]
    fn handler() -> String {
        todo!()
    }

    test_handler!(handler = Query<[], string>);
}

#[test]
fn subscription_return() {
    #[handler(subscription)]
    fn handler() -> impl futures::Stream<Item = u32> {
        futures::stream::iter([])
    }

    test_handler!(handler = Subscription<[], number>);
}

#[test]
fn everything_query() {
    #[derive(Clone)]
    struct Ctx;

    #[handler(query, name = "other_name")]
    async fn handler(ctx: Ctx, param_1: u32, param_2: String) -> bool {
        todo!()
    }

    test_handler!(handler<Ctx>(other_name) = Query<[param_1: number, param_2: string], boolean>);
}

#[test]
fn everything_mutation() {
    #[derive(Clone)]
    struct Ctx;

    #[handler(mutation, name = "other_name")]
    async fn handler(ctx: Ctx, param_1: u32, param_2: String) -> bool {
        todo!()
    }

    test_handler!(handler<Ctx>(other_name) = Mutation<[param_1: number, param_2: string], boolean>);
}

#[test]
fn everything_subscription() {
    #[derive(Clone)]
    struct Ctx;

    #[handler(subscription, name = "other_name")]
    fn handler(ctx: Ctx, param_1: u32, param_2: String) -> impl futures::Stream<Item = bool> {
        futures::stream::iter([])
    }

    test_handler!(handler<Ctx>(other_name) = Subscription<[param_1: number, param_2: string], boolean>);
}
