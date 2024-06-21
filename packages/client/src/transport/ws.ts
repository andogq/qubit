import type { Transport } from ".";
import type { RpcResponse } from "../jsonrpc";
import { type SocketOptions, create_promise_manager, create_socket, create_subscription_manager } from "../util";

export function ws(host: string, socket_options?: SocketOptions) {
  const subscriptions = create_subscription_manager();
  const requests = create_promise_manager();

  // Create a WS client
  const socket = create_socket(
    host,
    (message) => {
      if (message.type === "message") {
        subscriptions.handle(message);
      } else if ("id" in message) {
        requests.resolve(message);
      }
    },
    socket_options,
  );

  const send_request = (id: string | number, payload: string): Promise<RpcResponse<any>> => {
    // Send the data to the socket
    socket.send(payload);

    // Return a promise to wait for the request
    return requests.wait_for(id);
  };

  return {
    query: (id, payload) => send_request(id, JSON.stringify(payload)),
    mutate: (id, payload) => send_request(id, JSON.stringify(payload)),
    subscribe: (id, on_data) => {
      if (on_data) {
        // Subscribe to the events
        subscriptions.register(id, on_data);
      }

      // Return an unsubscribe handler
      return () => {
        // Remove the subscription
        subscriptions.remove(id);
      };
    },
  } satisfies Transport;
}
