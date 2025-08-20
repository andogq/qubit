---
"qubit": major
---

**(BREAKING)** Refactor router to separate RPC functionality from type generation functionality.
Now, use `router.as_codegen().write_type(path, TypeScript::new())` to generate types, and
`router.as_rpc(ctx).into_service()` to build the service.
