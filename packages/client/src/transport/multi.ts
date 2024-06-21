import { http, type HttpOptions, type SocketOptions, type Transport, ws } from ".";

export type MultiOptions = {
  ws?: SocketOptions;
  http?: HttpOptions;
};

/**
 * Transport that combines both `http` for query and mutate, and `ws` for subscriptions.
 */
export function multi(host: string, options?: MultiOptions) {
  const http_client = http(host, options?.http);
  const ws_client = ws(host, options?.ws);

  return {
    query: (id, payload) => {
      return http_client.query(id, payload);
    },
    mutate: (id, payload) => {
      return http_client.mutate(id, payload);
    },
    subscribe: (id, on_data) => {
      return ws_client.subscribe(id, on_data);
    },
  } satisfies Transport;
}
