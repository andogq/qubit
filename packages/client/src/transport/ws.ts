import { build_client } from "../client";
import type { RpcResponse } from "../jsonrpc";
import {
  create_promise_manager,
  create_socket,
  create_subscription_manager,
  type SocketOptions,
} from "../util";

export function ws<Server>(
  host: string,
  socket_options?: SocketOptions,
): Server {
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

  const send_request = (
    id: string | number,
    payload: any,
  ): Promise<RpcResponse<any>> => {
    // Send the data to the socket
    socket.send(payload);

    // Return a promise to wait for the request
    return requests.wait_for(id);
  };

  return build_client({
    request: send_request,
    subscribe: (id, on_data, on_end) => {
      if (on_data) {
        // Subscribe to the events
        subscriptions.register(id, on_data);
      }

      // Return an unsubscribe handler
      return () => {
        // Remove the subscription
        subscriptions.remove(id);

        // Send an unsubscribe request
        send_request(`${id}_unsubscribe`, [id]);

        // Notify the subscriber
        if (on_end) {
          on_end();
        }
      };
    },
  });
}
