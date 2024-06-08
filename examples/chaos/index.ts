import { ws } from "@qubit-rs/client";
import type { QubitServer } from "./bindings";

const client = ws<QubitServer>("ws://localhost:9944/rpc");

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

client.countdown.subscribe(1, 4, {
  on_data: (data) => {
    console.log("countdown: ", data);
  },
  on_end: () => {
    console.log("countdown done");
  },
});

client.countdown.subscribe(1, 4, (n) => console.log("number is", n));
