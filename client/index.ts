import { RpcResponse, create_payload, parse_response } from "./jsonrpc";
import { WebSocket } from "ws";

import type { Server } from "./bindings";

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
			console.log(payload);
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
client.version().then((version) => console.log({ version })).catch(console.error);
// client.user.get("test").then((user) => console.log(user)).catch(console.error);
client.count().then((value) => console.log({ value })).catch(console.error);

client.countdown(1, 4).subscribe({
	on_data: (data) => {
		console.log("countdown: ", data);
	},
	on_end: () => {
		console.log("countdown done");
	}
});

export default constructors;
