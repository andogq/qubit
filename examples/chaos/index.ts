import { http, build_client } from "@qubit-rs/client";
import type { QubitServer } from "./bindings";

const client = build_client<QubitServer>(http("http://localhost:9944/rpc"));

client.version
  .query()
  .then((version) => console.log({ version }))
  .catch(console.error);
client.user.someHandler
  .query("test")
  .then((user) => console.log(user))
  .catch(console.error);
client.count
  .mutate()
  .then((value) => console.log({ value }))
  .catch(console.error);
