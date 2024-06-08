import { defineConfig } from "vite";

export default defineConfig({
  server: {
    proxy: {
      "/rpc": {
        target: "http://localhost:9944",
        ws: true,
      },
      "/login": {
        target: "http://localhost:9944",
        ws: true,
      },
    },
  },
});
