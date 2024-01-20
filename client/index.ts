import { RpcResponse, create_payload, parse_response } from "./jsonrpc";
import { WebSocket } from "ws";

// Sample type generations
export interface Metadata { param_a: string, param_b: number, param_c: boolean, }
export interface User { name: string, email: string, age: number, metadata: Metadata, }

// Sample server generation
type Server = {
	get: (p1: string) => Promise<User>,
	create: (p1: string, p2: string, p3: number) => Promise<User>
};

function build_client(do_request: (id: number, payload: any) => Promise<RpcResponse<any>>): Server {
	let next_id = 0;

	return new Proxy({}, {
		get: (_, prop) => {
			let method = [prop];

			return new Proxy(() => {}, {
				get: (_target, prop, client) => {
					method.push(prop);

					return client;
				},
				apply: async (_target, _this, args) => {
					const id = next_id++;

					const payload = create_payload(id, method.join("."), args);
					const response = await do_request(id, payload);

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
		return build_client((_id ,payload) => {
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
		});
	},
	ws: (host: string): Server => {
		// Create a WS client
		const socket = new WebSocket(host);
		const queue: any[] = [];

		let socket_open = false;

		const outstanding: Record<number, (response: RpcResponse<any>) => void> = {};
		const send_request = (id: number, payload: any): Promise<RpcResponse<any>> => {
			return new Promise((resolve) => {
				outstanding[id] = resolve;

				if (socket_open) {
					socket.send(payload);
				} else {
					queue.push(payload);
				}
			});
		};

		socket.addEventListener("open", () => {
			socket_open = true;

			// Empty the queue
			for (let payload of queue) {
				socket.send(payload);
			}
		});

		socket.addEventListener("message", (e) => {
			const response = parse_response(e.data);

			if ("id" in response && outstanding[response.id]) {
				outstanding[response.id](response);
			}
		});

		return build_client(send_request);
	}
}

const client = constructors.ws("ws://localhost:9944/rpc");
client.get("test").then((user) => console.log(user));

export default constructors;
