import { type RpcResponse, create_payload } from "./jsonrpc";
import { wrap_promise } from "./proxy";
import type { StreamHandler, StreamHandlers, StreamSubscriber } from "./stream";

export type Client = {
  request: (
    id: string | number,
    payload: any,
  ) => Promise<RpcResponse<unknown> | null>;
  subscribe?: (
    id: string | number,
    on_data?: (value: any) => void,
    on_end?: () => void,
  ) => () => void;
};

/**
 * Set up a proxy that tracks all the methods chained onto it, and calls the provided method when
 * the proxy is used as a function called.
 */
function proxy_chain<T>(
  apply: (chain: string[], args: unknown[]) => unknown,
  chain: string[] = [],
): T {
  const proxy: T = new Proxy(() => {}, {
    get: (_target, property, client) => {
      // Make sure it was accessed with a valid property
      if (typeof property !== "string") {
        return client;
      }

      // If there is no chain, create a new instance
      if (chain.length === 0) {
        return proxy_chain(apply, [property]);
      }

      // Update the existing chain
      chain.push(property);

      return client;
    },
    apply: (_target, _this, args) => {
      return apply(chain, args);
    },
  }) as unknown as T;

  return proxy;
}

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
function get_handlers(
  handler: StreamHandler<unknown>,
): StreamHandlers<unknown> {
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

  return proxy_chain<Server>((method, args) => {
    const id = next_id++;

    // biome-ignore lint/suspicious/noAsyncPromiseExecutor: TODO: check this is correct
    const p = new Promise(async (resolve, reject) => {
      const payload = create_payload(id, method.join("."), args);
      const response = await client.request(id, payload);

      if (response !== null && response.type === "ok") {
        resolve(response.value);
      } else {
        reject(response);
      }
    });

    const subscribe: StreamSubscriber<unknown> = (handler) => {
      // Get user handlers for the subscription
      const { on_data, on_error, on_end } = get_handlers(handler);

      // Make sure the client can handle susbcriptions
      if (!client.subscribe) {
        on_error(new Error("client does not support subscriptions"));
        return () => {};
      }
      const subscribe = client.subscribe;

      const unsubscribe = sync_promise(async () => {
        // Get the response of the request
        const subscription_id = await p;

        let count = 0;
        let required_count: number | null = null;

        // Result should be a subscription ID
        if (
          typeof subscription_id !== "string" &&
          typeof subscription_id !== "number"
        ) {
          // TODO: Throw an error
          on_error(new Error("cannot subscribe to subscription"));
          return () => {};
        }

        // Subscribe to incomming requests
        return subscribe(
          subscription_id,
          (data) => {
            if (
              typeof data === "object" &&
              "close_stream" in data &&
              data.close_stream === subscription_id
            ) {
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

      return unsubscribe;
    };

    return wrap_promise(p, { subscribe });
  });
}
