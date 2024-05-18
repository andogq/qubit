import { build_client } from "../client";
import { type RpcResponse, parse_response } from "../jsonrpc";

export function ws<Server>(host: string, { WebSocket = window.WebSocket } = {}): Server {
	// Create a WS client
	const socket = new WebSocket(host);
	const queue: any[] = [];

	const subscription_queue: Record<string, any[]> = {};
	const subscriptions: Record<string, (value: any) => void> = {};

	let socket_open = false;

	const outstanding: Record<string | number, (response: RpcResponse<any>) => void> = {};
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
			const handler = subscriptions[response.id];
			if (handler) {
				handler(response.value);
			} else {
				let queue = subscription_queue[response.id];

				// Initialise the queue if it doesn't exist
				if (!queue) {
					queue = [];
					subscription_queue[response.id] = queue;
				}

				// Add the message to the queue
				queue.push(response.value);
			}
		} else if ("id" in response) {
			const handler = outstanding[response.id] ;
			if (handler) {
				handler(response);
			}
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
