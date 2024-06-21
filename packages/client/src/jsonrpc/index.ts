export type RpcRequest = {
  jsonrpc: "2.0";
  method: string;
  id: string | number;
  params: unknown;
};

export function create_payload(id: string | number, method: string, params: any): RpcRequest {
  return {
    jsonrpc: "2.0",
    method,
    id,
    params,
  };
}

/**
 * A JSONRPC-2.0 error.
 */
export type RpcError = {
  code: number;
  message: string;
  data: any;
};

/**
 * An incoming message for a specific subscription
 */
export type RpcSubscriptionMessage<T> = {
  type: "message";
  id: string | number;
  value: T;
};

export type RpcResponse<T> =
  | { type: "ok"; id: string | number; value: T }
  | { type: "error"; id: string | number; value: RpcError }
  | RpcSubscriptionMessage<T>;

export function parse_response<T>(response: any): RpcResponse<T> | null {
  try {
    if (typeof response === "string") {
      // biome-ignore lint/style/noParameterAssign: rust-pilled
      response = JSON.parse(response);
    }

    if (response?.jsonrpc !== "2.0") {
      throw new Error("invalid value for `jsonrpc`");
    }

    if ("params" in response && "subscription" in response.params && "result" in response.params) {
      return {
        type: "message",
        id: response.params.subscription,
        value: response.params.result,
      };
    }

    if (typeof response?.id !== "number" && typeof response?.id !== "string" && response?.id !== null) {
      throw new Error("missing `id` field from response");
    }

    if ("result" in response && !("error" in response)) {
      return { type: "ok", id: response.id, value: response.result };
    }

    if ("error" in response && !("result" in response)) {
      if (typeof response.error?.code === "number" && typeof response.error?.message === "string") {
        // TODO: Validate error.data field when it's decided
        return { type: "error", id: response.id, value: response.error };
      }
      throw new Error("malformed error object in response");
    }

    throw new Error("invalid response object");
  } catch (e) {
    console.error("Error encountered whilst parsing response");
    console.error(e);

    return null;
  }
}
