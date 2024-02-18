export function wrap_promise<T>(p: Promise<T>): Promise<T> {
	return new Proxy(p, {
		get(target, property, receiver) {
			if (property === "subscribe") {
				// Return subscription function
				return () => {
					console.log("subscription function");
				}
			} else {
				// Get the property from the original handler
				const value = Reflect.get(target, property, receiver);

				if (typeof value === "function") {
					// Make sure value is bounded to the original target
					return value.bind(target);
				} else {
					// Return the value as-is
					return value;
				}
			}
		},
	});
}
