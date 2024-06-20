/**
 * A handler function which must always take the path, represented as an array of strings.
 */
type HandlerFn<TArgs extends any[], TReturn> = (path: string[], ...args: TArgs) => TReturn;

/**
 * Strips the `path` parameter (first parameter) from a `HandlerFn`. This represents the function
 * that is exposed to the end-user.
 */
type StripPath<F> = F extends HandlerFn<infer TArgs, infer TReturn> ? (...args: TArgs) => TReturn : never;

/**
 * A collection of raw handlers, meaning handlers that include the path parameter.
 */
type RawHandlers = Record<string, HandlerFn<any[], any>>;

/**
 * For all available handlers, will produce a handler that has the `path` parameter stripped from
 * it.
 */
type Handlers<THandlers extends RawHandlers> = {
  [K in keyof THandlers]: StripPath<THandlers[K]>;
};

/**
 * Creates a proxied object that will call the provided builder method when accessed. This is
 * useful for building a new 'instance' any time it's accessed.
 */
function proxy_builder<T, TOut extends Record<any, any>>(builder: (property: string | symbol) => T) {
  return new Proxy({} as TOut, {
    get(_target, property) {
      return builder(property);
    },
  });
}

/**
 * Converts an object of raw handlers into handlers suitable for end-users. It requires a
 * reference to the path array which will be captured by all handlers.
 */
function wrap_handlers<THandlers extends RawHandlers>(handlers: THandlers, path: string[]): Handlers<THandlers> {
  const wrapped: Partial<Handlers<THandlers>> = {};

  for (const [key, handler] of Object.entries(handlers)) {
    // @ts-ignore: Indexing into wrapped object with known key
    wrapped[key] = (...args: unknown[]) => {
      return handler(path, ...args);
    };
  }

  // Will contain all keys after loop is done.
  return wrapped as Handlers<THandlers>;
}

/**
 * Create a proxy that will collect property accessing into an array of strings, and upon
 * accessing an item from the `handlers` parameter, return the method with the traversed path
 * provided.
 */
export function create_path_builder<TOut, THandlers extends RawHandlers>(handlers: THandlers): TOut {
  return proxy_builder((property) => {
    const path = [];

    // If the accessed property isn't a string, just skip adding it to the path
    if (typeof property === "string") {
      path.push(property);
    }

    // Build the underlying proxy from the user's handlers
    const proxy = new Proxy(wrap_handlers(handlers, path), {
      get: (target, property, proxy) => {
        // If the accessed property wasn't a string, ignore it.
        if (typeof property !== "string") {
          console.warn("attempted to access non-string property:", property);
          return proxy;
        }

        // The requested item is an underlying handler
        if (property in target) {
          return target[property];
        }

        // Track this item in the path and continue
        path.push(property);
        return proxy;
      },
    });

    return proxy;
  });
}
