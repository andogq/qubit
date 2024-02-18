import { build_client } from "../client";
import { RpcResponse, parse_response } from "../jsonrpc";
import { WebSocket } from "ws";

export function ws<Server>(host: string): Server {
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
