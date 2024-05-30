export function wrap_promise<T>(p: Promise<T>, extensions: Record<string, (args: any) => void>): Promise<T> {
  return new Proxy(p, {
    get(target, property, receiver) {
      if (typeof property === "string" && extensions[property]) {
        return extensions[property];
      }
      // Get the property from the original handler
      const value = Reflect.get(target, property, receiver);

      if (typeof value === "function") {
        // Make sure value is bounded to the original target
        return value.bind(target);
      }
      // Return the value as-is
      return value;
    },
  });
}
