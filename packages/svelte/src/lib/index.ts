import { http, type HttpOptions, type MultiOptions, type Transport, build_client, multi } from "@qubit-rs/client";
import { getContext, hasContext, setContext } from "svelte";

const CONTEXT_NAME = "__qubit-rs-svelte-context";

type SvelteQubitOptions = MultiOptions & {
  /**
   * Whether running in the browser (provided by `$app/environment`). Assumed `true` if not provided.
   */
  browser?: boolean;
};

/**
 * Create a new Qubit instance.
 *
 * @param host - Host URL where the Qubit server is running.
 * @param options - Configuration for the underlying transport.
 */
export function create_qubit_api<Server>(host: string, options?: SvelteQubitOptions) {
  let client: Server;

  function get_client(overrides?: { fetch: HttpOptions["fetch"] }) {
    if (!client) {
      let transport: Transport;

      if (options?.browser === true) {
        // biome-ignore lint/style/noParameterAssign:
        options ??= {};
        options.http = options.http ?? ({} as HttpOptions);
        options.http.fetch = overrides?.fetch;

        transport = multi(host, options);
      } else {
        const http_options = options?.http ?? ({} as HttpOptions);

        if (overrides?.fetch) {
          http_options.fetch = overrides.fetch;
        }

        transport = http(host, http_options);
      }

      client = build_client<Server>(transport);
    }

    return client;
  }

  return {
    /**
     * Initialise the context so the API instance can be accessible within the application. This
     * should be run at the root layout, and only done once.
     */
    init_context: () => {
      setContext(CONTEXT_NAME, get_client());
    },

    /**
     * Fetch the API instance.
     */
    get_api: () => {
      if (!hasContext(CONTEXT_NAME)) {
        throw new Error("@qubit-rs/svelte: ensure that `init_context` has been called at the root layout.");
      }

      return getContext<Server>(CONTEXT_NAME);
    },

    // biome-ignore lint/correctness/noUnusedVariables:
    load_api: ({ fetch, depends }: LoadApiOptions): Server => {
      return get_client({ fetch });
    },
  };
}

type LoadApiOptions = {
  fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;
  depends: (...deps: `${string}:${string}`[]) => void;
};
