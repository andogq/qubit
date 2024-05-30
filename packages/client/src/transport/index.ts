export { ws } from "./ws";
export { http } from "./http";

export type ClientBuilder<Server> = (host: string) => Server;
