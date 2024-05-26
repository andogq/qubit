import { defineConfig } from "vite";

export default defineConfig({
  server: {
    proxy: {
      "/cookie": {
        target: "http://localhost:9944",
        ws: true,
      },
      "/mutable-ctx": {
        target: "http://localhost:9944",
        ws: true,
      },
    },
  },
});
