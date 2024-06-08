import { ws } from "@qubit-rs/client";
import type { QubitServer as CookieServer } from "./bindings-cookie-auth";
import type { QubitServer as MutableCtxServer } from "./bindings-mutable-ctx";

async function cookie_flow() {
  console.log("----- Beginning Cookie Flow -----");

  function build_api(): CookieServer {
    return ws<CookieServer>(`ws://${window.location.host}/cookie/rpc`);
  }

  document.cookie += "qubit-auth=;expires=Thu, 01 Jan 1970 00:00:01 GMT";

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
  await fetch("/cookie/login", {
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
    console.log("Cookie is", await api.echo_cookie.query());
    console.log("Can we get the secret?", await api.secret_endpoint.query());
  }

  console.log("----- Ending Cookie Flow -----");
}

async function mutable_ctx_flow() {
  console.log("----- Beginning Mutable Ctx Flow -----");

  const api = ws<MutableCtxServer>(`ws://${window.location.host}/mutable-ctx/rpc`);

  // Attempt to get the secret without authentication
  await api.secret_endpoint.query().catch((e) => {
    console.error("Error whilst accessing secret:", e);
  });

  // Login to authenticate this connection
  await api.login.mutate("user", "password");
  console.log("Successfully authenticated with the API");

  console.log("The secret is", await api.secret_endpoint.query());

  console.log("----- Ending Mutable Ctx Flow -----");
}

async function main() {
  await cookie_flow();
  await mutable_ctx_flow();
}

main();
