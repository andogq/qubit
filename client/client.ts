import { RpcResponse, create_payload } from "./jsonrpc";
import { wrap_promise } from "./proxy";

export function build_client<Server>(do_request: (id: number, payload: any) => Promise<RpcResponse<any>>): Server {
	let next_id = 0;

	return new Proxy({}, {
		get: (_, prop) => {
			let method = [prop];

			return new Proxy(() => {}, {
				get: (_target, prop, client) => {
					method.push(prop);

					return client;
				},
				apply: (_target, _this, args) => {
					const id = next_id++;

					const p = new Promise(async (resolve, reject) => {
						const payload = create_payload(id, method.join("."), args);
						const response = await do_request(id, payload);

						if (response.type === "ok") {
							resolve(response.value);
						} else {
							reject(response);
						}
					});

					return wrap_promise(p);
				}
			});
		}
	}) as unknown as Server;
}
