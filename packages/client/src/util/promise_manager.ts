import type { RpcResponse } from "../jsonrpc";

/**
 * Utility to create promises assigned with some ID, and later resolve them by referring to the
 * same ID.
 */
export function create_promise_manager() {
  const promises: Record<string | number, (response: RpcResponse<unknown>) => void> = {};

  return {
    /** Send some payload, for a given ID */
    wait_for: (id: string | number): Promise<RpcResponse<unknown>> => {
      return new Promise((resolve) => {
        promises[id] = resolve;
      });
    },

    /** Resolve a response based on an ID */
    resolve: (response: RpcResponse<unknown>) => {
      const handler = promises[response.id];
      if (handler) {
        handler(response);
      }
    },
  };
}
