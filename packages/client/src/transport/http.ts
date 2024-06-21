import type { Transport } from ".";
import { parse_response } from "../jsonrpc";

export type HttpOptions = {
  fetch: typeof fetch;
};

export function http(host: string, http_options?: HttpOptions) {
  const fetch_impl = http_options?.fetch || fetch;

  return {
    query: async (_id, payload) => {
      // Encode the payload in to the URL
      const url = new URL(host);
      url.searchParams.set("input", encodeURIComponent(JSON.stringify(payload)));

      const res = await fetch_impl(url, {
        method: "GET",
        mode: "cors",
      });

      const body = await res.json();

      return parse_response(body);
    },
    mutate: async (_id, payload) => {
      const res = await fetch_impl(host, {
        method: "POST",
        mode: "cors",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });

      const body = await res.json();

      return parse_response(body);
    },
  } satisfies Transport;
}
