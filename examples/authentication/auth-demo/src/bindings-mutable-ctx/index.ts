
export type QubitServer = { login: (username: string, password: string) => Promise<boolean>, secret_endpoint: () => Promise<string> };