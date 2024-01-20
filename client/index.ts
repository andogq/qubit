import { RpcResponse, create_payload, parse_response } from "./jsonrpc";

// Sample type generations
export interface Metadata { param_a: string, param_b: number, param_c: boolean, }
export interface User { name: string, email: string, age: number, metadata: Metadata, }

// Sample server generation
type Server = {
	get: (p1: string) => Promise<User>,
	create: (p1: string, p2: string, p3: number) => Promise<User>
};

function build_client(do_request: (payload: any) => Promise<RpcResponse<any>>): Server {
	let id = 0;

	return new Proxy({}, {
		get: (_, prop) => {
			let method = [prop];

			return new Proxy(() => {}, {
				get: (_target, prop, client) => {
					method.push(prop);

					return client;
				},
				apply: async (_target, _this, args) => {
					const payload = create_payload(id++, method.join("."), args);
					const response = await do_request(payload);

					if (response.type === "ok") {
						return response.value;
					} else {
						throw response;
					}
				}
			});
		}
	}) as unknown as Server;
}

const constructors = {
	http: (host: string): Server => {
		return build_client((payload) => {
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
		})

	},
	ws: (host: string): Server => {
		// Create a WS client
	}
}

const client = constructors.http("http://localhost:9944/rpc");
client.get("test").then((user) => console.log(user));

export default constructors;
