import type { RpcResponse } from "../jsonrpc";

export { ws } from "./ws";
export { http, type HttpOptions } from "./http";
export type { SocketOptions } from "../util";

export type ClientBuilder<Server> = (host: string) => Server;

/**
 * Interface required for a transport.
 */
export type Transport = {
  /**
   * Initiate a request, and receive a single response.
   */
  request: (id: string | number, payload: any) => Promise<RpcResponse<unknown> | null>;
  /**
   * Start a subscription, calling `on_data` for every message from the server, and `on_end` when
   * the subscription terminates. An unsubscribe method must be returned, which must terminate the
   * subscription when called.
   */
  subscribe?: (id: string | number, on_data?: (value: any) => void, on_end?: () => void) => () => void;
};
