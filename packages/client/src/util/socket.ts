import { parse_response, type RpcResponse } from "../jsonrpc";

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

  /** Queue of requests that were made before the socket was opened. */
  let queue: string[] = [];

  let socket: WebSocket;
  if (options?.WebSocket) {
    // TODO: Also work out how to type this
    const WebSocket = options.WebSocket as any;
    socket = new WebSocket(host);
  } else {
    socket = new WebSocket(host);
  }

  socket.addEventListener("open", () => {
    socket_open = true;

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

  return {
    send: (payload: string) => {
      if (!socket_open) {
        // Queue the request up for when the socket opens
        queue.push(payload);
      }
    },
  };
}
