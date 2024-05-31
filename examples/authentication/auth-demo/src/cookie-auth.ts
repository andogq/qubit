
export type QubitServer = { echo_cookie: () => Promise<string>, secret_endpoint: () => Promise<string> };