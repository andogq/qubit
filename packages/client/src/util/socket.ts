import { type RpcResponse, parse_response } from "../jsonrpc";

type Socket = { send: (payload: string) => void };

export type SocketOptions = {
  WebSocket: unknown; // TODO: Work out how to properly type this
};

export function create_socket(
  host: string,
  on_message: (message: RpcResponse<unknown>) => void,
  options?: SocketOptions,
): Socket {
  /** Track whether the socket has been opened. */
  let socket_open = false;
  let next_timeout = 0.5;

  /** Queue of requests that were made before the socket was opened. */
  let queue: string[] = [];

  // TODO: Type this
  const WS: any = options?.WebSocket || WebSocket;

  let socket: WebSocket;

  function new_socket() {
    socket = new WS(host);

    socket.addEventListener("open", () => {
      socket_open = true;
      next_timeout = 0.5; // Reset timeout

      // Run through the items in the queue and send them off
      for (const payload of queue) {
        socket.send(payload);
      }

      queue = [];
    });

    socket.addEventListener("message", (e) => {
      const message = parse_response(e.data);

      if (message) {
        on_message(message);
      }
    });

    socket.addEventListener("close", () => {
      // Start attempting to re-open the socket
      socket_open = false;

      setTimeout(() => {
        // Increase the timeout
        next_timeout *= 2;

        // Try re-create the socket
        new_socket();

        // TODO: Re-subscribe to subscriptions
      }, next_timeout * 1000);
    });
  }

  new_socket();

  return {
    send: (payload: string) => {
      if (!socket_open) {
        // Queue the request up for when the socket opens
        queue.push(payload);
      } else {
        socket.send(payload);
      }
    },
  };
}
