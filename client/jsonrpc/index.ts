export function create_payload(id: number, method: string, params: any) {
	return JSON.stringify({
		jsonrpc: '2.0',
		method,
		id,
		params,
	});
}

export type RpcError = {
	code: number,
	message: string,
	data: any,
};

export type RpcResponse<T> = { type: "ok", value: T }
	| { type: "error", value: RpcError }
	| { type: "bad_response" };

export function parse_response<T>(response: any): RpcResponse<T> {
	try {
		if (response?.jsonrpc !== "2.0") {
			throw new Error("invalid value for `jsonrpc`");
		}

		if (typeof response?.id !== "number" && response?.id !== null) {
			throw new Error("missing `id` field from response");
		}

		if ("result" in response && !("error" in response)) {
			return { type: "ok", value: response.result };
		}

		if ("error" in response && !("result" in response)) {
			if (typeof response.error?.code === "number" && typeof response.error?.message === "string") {
				// TODO: Validate error.data field when it's decided
				return { type: "error", value: response.error };
			} else {
				throw new Error("malformed error object in response");
			}
		}

		throw new Error("invalid response object");
	} catch (e) {
		console.error("Error encountered whilst parsing response");
		console.error(e);

		return { type: "bad_response" };
	}
}
