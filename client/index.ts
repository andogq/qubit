// Sample type generations
export interface Metadata { param_a: string, param_b: number, param_c: boolean, }
export interface User { name: string, email: string, age: number, metadata: Metadata, }

// Sample server generation
type Server = {
	get: (p1: string) => Promise<User>,
	create: (p1: string, p2: string, p3: number) => Promise<User>
};

function request(method: string, params: any[]) {
	return fetch("http://localhost:9944/rpc", {
		method: 'POST',
		mode: 'cors',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			jsonrpc: '2.0',
			method,
			id: 1,
			params,
		})
	})
		.then((res) => {
			return res.json()
		})
		.then((body) => {
			return body.result
		});
}

const client = new Proxy({}, {
	get: (_, prop) => {
		let path = [prop];

		return new Proxy(() => {}, {
			get: (_target, prop, client) => {
				path.push(prop);

				return client;
			},
			apply: (_target, _this, args) => {
				console.log("running", path.join("."), "with", args);

				return request(path.join("."), args);
			}
		});
	}
}) as unknown as Server;

client.get("test").then((user) => console.log(user));
