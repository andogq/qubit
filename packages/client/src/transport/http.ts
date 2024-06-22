import type { Transport } from ".";
import { parse_response } from "../jsonrpc";

export type HttpOptions = {
  fetch?: typeof fetch;
};

export function http(host: string, http_options?: HttpOptions) {
  const fetch_impl = http_options?.fetch || fetch;

  return {
    query: async (_id, payload) => {
      // Create a temporary URL with a fallback host to appease the URL constructor
      const temp_url = new URL(host, "http://example.com");

      // Set the search parameters, to let it do the processing for us
      temp_url.searchParams.set("input", encodeURIComponent(JSON.stringify(payload)));

      // Use the original host, but replace anything after the `?` with our modified query parameters
      const url = `${host.replace(/\?.*$/, "")}?${temp_url.searchParams.toString()}`;

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
