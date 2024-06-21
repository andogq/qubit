import type { RpcRequest, RpcResponse } from "../jsonrpc";

export { ws } from "./ws";
export { http, type HttpOptions } from "./http";
export { multi, type MultiOptions } from "./multi";
export type { SocketOptions } from "../util";

export type ClientBuilder<Server> = (host: string) => Server;

/**
 * Interface required for a transport.
 */
export type Transport = {
  /**
   * Make a request that is a query, meaning that it is safe to be cached.
   */
  query: (id: string | number, payload: RpcRequest) => Promise<RpcResponse<unknown> | null>;
  /**
   * Make a request that is a mutation, meaning that it should not be cached.
   */
  mutate: (id: string | number, payload: RpcRequest) => Promise<RpcResponse<unknown> | null>;
  /**
   * Start a subscription, calling `on_data` for every message from the server. An unsubscribe
   * method must be returned, which must terminate the subscription when called.
   */
  subscribe?: (id: string | number, on_data?: (value: any) => void) => () => void;
};
