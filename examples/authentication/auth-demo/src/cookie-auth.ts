
export type Server = { echo_cookie: () => Promise<string>, secret_endpoint: () => Promise<string> };