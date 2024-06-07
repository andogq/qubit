import { build_client } from "../client";
import { parse_response } from "../jsonrpc";

export function http<Server>(host: string): Server {
  return build_client({
    request: async (_id, payload) => {
      const res = await fetch(host, {
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
