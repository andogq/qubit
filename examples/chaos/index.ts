import { ws } from "@qubit-rs/client";
import type { QubitServer } from "./bindings.ts";

const client = ws<QubitServer>("ws://localhost:9944/rpc");

client
  .version()
  .then((version) => console.log({ version }))
  .catch(console.error);
client.user
  .someHandler("test")
  .then((user) => console.log(user))
  .catch(console.error);
client
  .count()
  .then((value) => console.log({ value }))
  .catch(console.error);

client.countdown(1, 4).subscribe({
  on_data: (data) => {
    console.log("countdown: ", data);
  },
  on_end: () => {
    console.log("countdown done");
  },
});

client.countdown(1, 4).subscribe((n) => console.log("number is", n));
