import { ws } from "@qubit-rs/client";
import type { Server } from "./bindings";

export const api = ws<Server>("ws://localhost:9944/rpc");
