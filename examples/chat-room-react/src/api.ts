import { build_client, ws } from "@qubit-rs/client";
import type { QubitServer } from "./bindings";

export const api = build_client<QubitServer>(ws("ws://localhost:9944/rpc"));
