import { build_client } from "../client";
import { RpcResponse, parse_response } from "../jsonrpc";
import { WebSocket } from "ws";

export function ws<Server>(host: string): Server {
	// Create a WS client
	const socket = new WebSocket(host);
	const queue: any[] = [];

	const subscription_queue: Record<string, any[]> = {};
	const subscriptions: Record<string, (value: any) => void> = {};

	let socket_open = false;

	const outstanding: Record<number, (response: RpcResponse<any>) => void> = {};
	const send_request = (id: string | number, payload: any): Promise<RpcResponse<any>> => {
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

		if (response.type === "message") {
			if (subscriptions[response.id]) {
				subscriptions[response.id](response.value);
			} else {
				// Initialise the queue if it doesn't exist
				if (!subscription_queue[response.id]) {
					subscription_queue[response.id] = [];
				}

				// Add the message to the queue
				subscription_queue[response.id].push(response.value);
			}
		} else if ("id" in response && outstanding[response.id]) {
			outstanding[response.id](response);
		}
	});

	return build_client({
		request: send_request,
		subscribe: (id, on_data, on_end) => {
			if (on_data) {
				// Subscribe to the events
				subscriptions[id] = on_data;

				// Pull out existing values from the queue
				for (const value of subscription_queue[id] || []) {
					on_data(value);
				}

				// Remove the queue
				delete subscription_queue[id];
			}

			// Return an unsubscribe handler
			return () => {
				// Remove the subscription
				delete subscriptions[id];

				// Send an unsubscribe request
				send_request(`${id}_unsubscribe`, [id]);

				// Notify the subscriber
				if (on_end) {
					on_end();
				}
			}
		}
	});
}
