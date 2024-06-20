import { build_client, ws } from "@qubit-rs/client";
import type { QubitServer } from "./bindings";

async function main() {
  console.log("----- Beginning Authentication Flow -----");

  function build_api() {
    return build_client<QubitServer>(ws(`ws://${window.location.host}/rpc`));
  }

  document.cookie += "qubit-auth=;expires=Thu, 01 Jan 1970 00:00:01 GMT;SameSite=Lax";

  // Make some un-authenticated requests
  {
    const api = build_api();

    console.log("Cookie echo from server:", await api.echo_cookie.query());

    // Can we get the secret?
    await api.secret_endpoint.query().catch((e) => {
      console.error("Error whilst accessing secret:", e);
    });
  }
  // Authenticate with the API
  await fetch("/login", {
    method: "POST",
    body: new URLSearchParams({ username: "user", password: "password" }),
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
    },
  });

  console.log("Successfully authenticated with the API");

  {
    // Re-create the API now that we're authenticated
    const api = build_api();
    console.log("Cookie is:", await api.echo_cookie.query());
    console.log("Can we get the secret?", await api.secret_endpoint.query());
  }

  console.log("----- Ending Authentication Flow -----");
}

main();
