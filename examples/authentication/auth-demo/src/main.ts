import { ws } from "@qubit-rs/client";
import { Server as CookieServer } from "./cookie-auth";

async function cookie_flow() {
  function build_api(): CookieServer {
    return ws<CookieServer>(`ws://${window.location.host}/cookie/rpc`);
  }

  document.cookie += "qubit-auth=;expires=Thu, 01 Jan 1970 00:00:01 GMT";

  // Make some un-authenticated requests
  {
    const api = build_api();

    console.log("Cookie echo from server:", await api.echo_cookie());

    // Can we get the secret?
    await api.secret_endpoint().catch((e) => {
      console.error("Error whilst accessing secret:", e);
    });
  }

  {
    // Authenticate with the API
    await fetch("/cookie/login", {
      method: "POST",
      body: new URLSearchParams({ username: "user", password: "password" }),
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
    });

    console.log("Successfully authenticated with the API");
  }

  {
    // Re-create the API now that we're authenticated
    const api = build_api();
    console.log("Cookie is", await api.echo_cookie());
    console.log("Can we get the secret?", await api.secret_endpoint());
  }
}

cookie_flow();
