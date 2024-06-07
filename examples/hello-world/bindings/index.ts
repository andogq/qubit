import type { Query } from "@qubit-rs/client";
export type QubitServer = { hello_world: Query<() => Promise<string>> };