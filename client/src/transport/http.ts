import { build_client } from "../client";
import { parse_response } from "../jsonrpc";

export function http<Server>(host: string): Server {
	return build_client({
		request: (_id ,payload) => {
			return fetch(host, {
				method: 'POST',
				mode: 'cors',
				headers: { 'Content-Type': 'application/json' },
				body: payload,
				})
				.then((res) => {
					return res.json()
				})
				.then(parse_response);
		}
	});
}
