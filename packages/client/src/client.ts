import type { StreamHandler, StreamHandlers } from "./handler/subscription";
import { type RpcResponse, create_payload } from "./jsonrpc";
import { create_path_builder } from "./path_builder";

export type Client = {
  request: (id: string | number, payload: any) => Promise<RpcResponse<unknown> | null>;
  subscribe?: (id: string | number, on_data?: (value: any) => void, on_end?: () => void) => () => void;
};

/**
 * Convert promise that resolves into a callback into a callback that can be called synchronously.
 */
function sync_promise(implementation: () => Promise<() => void>): () => void {
  // Run the implementation, save the promise
  const callback_promise = implementation();

  // Synchronously return a callback that will call the asynchronous callback
  return () => {
    callback_promise.then((callback) => {
      callback();
    });
  };
}

/**
 * Destructure user handlers, and ensure that they all exist.
 */
function get_handlers(handler: StreamHandler<unknown>): StreamHandlers<unknown> {
  let on_data = (_: unknown) => {};
  let on_error = (_: Error) => {};
  let on_end = () => {};

  if (typeof handler === "function") {
    on_data = handler;
  } else {
    if (handler?.on_data) {
      on_data = handler.on_data;
    }
    if (handler?.on_error) {
      on_error = handler.on_error;
    }
    if (handler?.on_end) {
      on_end = handler.on_end;
    }
  }

  return { on_data, on_error, on_end };
}

export function build_client<Server>(client: Client): Server {
  let next_id = 0;

  async function send(method: string[], args: unknown): Promise<unknown> {
    const id = next_id++;

    const payload = create_payload(id, method.join("."), args);
    const response = await client.request(id, payload);

    if (response === null || response.type !== "ok") {
      throw response;
    }

    return response.value;
  }

  return create_path_builder({
    query: (method, ...args: unknown[]) => {
      return send(method, args);
    },
    mutate: async (method, ...args: unknown[]) => {
      return send(method, args);
    },
    subscribe: async (method, ...args: unknown[]) => {
      const { on_data, on_error, on_end } = get_handlers(args.pop() as StreamHandler<unknown>);
      const p = send(method, args);

      const unsubscribe = sync_promise(async () => {
        // Get the response of the request
        const subscription_id = await p;

        let count = 0;
        let required_count: number | null = null;

        // Result should be a subscription ID
        if (typeof subscription_id !== "string" && typeof subscription_id !== "number") {
          // TODO: Throw an error
          on_error(new Error("cannot subscribe to subscription"));
          return () => {};
        }

        if (!client.subscribe) {
          on_error(new Error("client does not support subscriptions"));
          return () => {};
        }

        // Subscribe to incomming requests
        return client.subscribe(
          subscription_id,
          (data) => {
            if (typeof data === "object" && "close_stream" in data && data.close_stream === subscription_id) {
              // Prepare to start closing the subscription
              required_count = data.count;
            } else {
              // Keep a count of incoming messages
              count += 1;

              // Forward the response onto the user
              if (on_data) {
                on_data(data);
              }
            }

            if (count === required_count) {
              // The expected amount of messages have been recieved, so it is safe to terminate the connection
              unsubscribe();
            }
          },
          on_end,
        );
      });
    },
  });
}
