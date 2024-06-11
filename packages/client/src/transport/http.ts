import { build_client } from "../client";
import { parse_response } from "../jsonrpc";

export type HttpOptions = {
  fetch: typeof fetch;
};

export function http<Server>(host: string, http_options?: HttpOptions): Server {
  const fetch_impl = http_options?.fetch || fetch;

  return build_client({
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
  });
}
