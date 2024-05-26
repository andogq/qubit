
export type Server = { login: (username: string, password: string) => Promise<boolean>, secret_endpoint: () => Promise<string> };