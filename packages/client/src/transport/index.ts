export { ws } from "./ws";
export { http, type HttpOptions } from "./http";
export type { SocketOptions } from "../util";

export type ClientBuilder<Server> = (host: string) => Server;
