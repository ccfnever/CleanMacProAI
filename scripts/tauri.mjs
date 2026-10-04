import { createRequire } from "node:module";

// Keep local development traffic out of HTTP proxies while preserving remote access.
const bypassHosts = new Set(
  [process.env.NO_PROXY, process.env.no_proxy]
    .filter(Boolean)
    .flatMap((value) => value.split(","))
    .map((value) => value.trim())
    .filter(Boolean),
);
for (const host of ["localhost", "127.0.0.1", "::1", "[::1]"]) {
  bypassHosts.add(host);
}
process.env.NO_PROXY = process.env.no_proxy = [...bypassHosts].join(",");

// Use the installed CLI in this process so it inherits the proxy bypass and signals.
createRequire(import.meta.url)("@tauri-apps/cli/tauri.js");
