import { readdir } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const directory = new URL("./", import.meta.url);
const tests = (await readdir(directory)).filter((name) => /^test-.*\.mjs$/.test(name) && name !== "test-all.mjs").sort();
for (const name of tests) {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL(name, directory))], { stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
console.log(`Passed all ${tests.length} frontend regression suites.`);
