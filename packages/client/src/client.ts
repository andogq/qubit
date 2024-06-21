import type { StreamHandler, StreamHandlers } from "./handler/subscription";
import { type RpcRequest, type RpcResponse, create_payload } from "./jsonrpc";
import { type Handlers, type Plugins, create_path_builder } from "./path_builder";
import type { Transport } from "./transport";

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

/**
 * Determines if the provided type has a nested object, or is just made up of functions.
 */
type HasNestedObject<T> = {
  [K in keyof T]: T[K] extends (...args: any[]) => any ? T[K] : never;
};
type AtEdge<T, Yes, No> = T extends HasNestedObject<T> ? Yes : No;

/**
 * Inject the provided plugins into the edges of the server.
 */
type InjectPlugins<TServer, TPlugins extends Plugins> = AtEdge<
  TServer,
  // Is at edge, merge with plugins
  TServer & TPlugins,
  // Not at edge, recurse
  { [K in keyof TServer]: InjectPlugins<TServer[K], TPlugins> }
>;

/**
 * Build a new client for a server.
 */
export function build_client<Server>(client: Transport): Server;
/**
 * Build a new client and inject the following plugins.
 */
export function build_client<Server, TPlugins extends Plugins>(
  transport: Transport,
  plugins: TPlugins,
): InjectPlugins<Server, Handlers<TPlugins>>;
export function build_client<Server, TPlugins extends Plugins>(
  client: Transport,
  plugins?: TPlugins,
): InjectPlugins<Server, Handlers<TPlugins>> {
  let next_id = 0;

  async function send(
    method: string[],
    sender: (id: string | number, payload: RpcRequest) => Promise<RpcResponse<unknown> | null>,
    args: unknown,
  ): Promise<unknown> {
    const id = next_id++;

    const payload = create_payload(id, method.join("."), args);
    const response = await sender(id, payload);

    if (response === null || response.type !== "ok") {
      throw response;
    }

    return response.value;
  }

  return create_path_builder({
    ...(plugins ?? {}),
    query: (method, ...args: unknown[]) => {
      return send(method, client.query, args);
    },
    mutate: async (method, ...args: unknown[]) => {
      return send(method, client.mutate, args);
    },
    subscribe: (method, ...args: unknown[]) => {
      const { on_data, on_error, on_end } = get_handlers(args.pop() as StreamHandler<unknown>);
      const p = send(method, client.mutate, args);

      const transport_unsubscribe = sync_promise(async () => {
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

        // Subscribe to incoming requests
        return client.subscribe(subscription_id, (data) => {
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
        });
      });

      const unsubscribe = async () => {
        // Send an unsubscribe message so the server knows we're not interested in the subscription
        const unsubscribe_method = method.slice(0, -1);
        const subscription_id = await p;
        unsubscribe_method.push(`${method.at(-1)}_unsub`);
        send(unsubscribe_method, client.query, [subscription_id]);

        // Allow the transport to clean up
        transport_unsubscribe();

        // Notify the user
        on_end();
      };

      return unsubscribe;
    },
  });
}
