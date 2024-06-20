import type { Transport } from ".";
import { parse_response } from "../jsonrpc";

export type HttpOptions = {
  fetch: typeof fetch;
};

export function http(host: string, http_options?: HttpOptions) {
  const fetch_impl = http_options?.fetch || fetch;

  return {
    request: async (_id, payload) => {
      const res = await fetch_impl(host, {
        method: "POST",
        mode: "cors",
        headers: { "Content-Type": "application/json" },
        body: payload,
      });

      const body = await res.json();

      return parse_response(body);
    },
  } satisfies Transport;
}
