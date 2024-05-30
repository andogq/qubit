import { ws } from "@qubit-rs/client";
import type { QubitServer } from "./bindings";

export const api = ws<QubitServer>("ws://localhost:9944/rpc");
