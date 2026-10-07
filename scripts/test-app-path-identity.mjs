import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import ts from "typescript";

const temp = await mkdtemp(fileURLToPath(new URL(".app-test-", import.meta.url)));
const transpile = (source) => ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
}).outputText;
try {
  const view = await readFile(new URL("../src/views/UninstallerView.vue", import.meta.url), "utf8");
  const script = view.match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1];
  await writeFile(join(temp, "view.mjs"), transpile(`${script}\nexport { apps, inspectApp, toggleSelected, toggleExpanded, selectedApps, expandedAppPaths, uninstallSelected, dataSource, sortBy, filteredApps };`)
    .replaceAll('"vue"', '"./vue.mjs"')
    .replaceAll('"../lib/demoData"', '"./boundary.mjs"')
    .replaceAll('"../lib/appOccupancy"', '"./occupancy.mjs"'));
  await writeFile(join(temp, "occupancy.mjs"), transpile(await readFile(new URL("../src/lib/appOccupancy.ts", import.meta.url), "utf8")));
  await writeFile(join(temp, "vue.mjs"), 'export { computed, ref } from "vue"; export const onMounted = () => {}; export const onUnmounted = () => {};');
  await writeFile(join(temp, "boundary.mjs"), `
    export const formatBytes = String;
    export async function invokeOrDemo(command, fallback, args) {
      globalThis.appCalls.push({ command, args });
      return { source: "native", data: globalThis.appResponses.shift() };
    }
  `);
  const state = await import(pathToFileURL(join(temp, "view.mjs")));
  const base = { name: "Archive", bundle_id: "test.archive", app_size: 0, related_size: 0, related_files: [], is_system_app: false };
  const a = { ...base, app_path: "/Applications/Archive.app" };
  const b = { ...base, app_path: "/Applications/Archive copy.app" };
  state.apps.value = [a, b];
  globalThis.appCalls = [];
  globalThis.appResponses = [{ ...a, app_size: 50 }, { ...b, app_size: 70 }];
  await state.inspectApp(a.app_path);
  assert.equal(state.apps.value[1].app_size, 0); // never overwrite another copy sharing the ID
  await state.inspectApp(b.app_path);
  assert.deepEqual(state.apps.value.map((app) => app.app_size), [50, 70]);
  assert.equal(state.sortBy.value, "size");
  assert.equal(state.filteredApps.value[0].app_path, b.app_path);
  state.apps.value[0].last_opened_at = 200;
  state.apps.value[1].last_opened_at = 100;
  state.sortBy.value = "opened";
  assert.equal(state.filteredApps.value[0].app_path, a.app_path);
  state.sortBy.value = "name";
  state.apps.value[0].name = "Z";
  state.apps.value[1].name = "A";
  assert.equal(state.filteredApps.value[0].app_path, b.app_path);
  assert.deepEqual(globalThis.appCalls.map((call) => call.args.appPath), [a.app_path, b.app_path]);
  state.toggleSelected(b.app_path);
  assert.equal(state.selectedApps.value.length, 1);
  assert.equal(state.selectedApps.value[0].app_path, b.app_path);
  state.toggleExpanded(b);
  assert.deepEqual([...state.expandedAppPaths.value], [b.app_path]);
  state.dataSource.value = "native";
  globalThis.appResponses = [{ cleaned_count: 1, freed_bytes: 70, skipped_count: 0, errors: [] }];
  await state.uninstallSelected(); // mocked boundary: never uninstall real applications
  assert.deepEqual(state.apps.value.map((app) => app.app_path), [a.app_path]);
  assert.equal(globalThis.appCalls.at(-1).args.appPath, b.app_path);
  console.log("Passed: same-ID copies are inspected, expanded, selected and removed independently by path.");
} finally {
  await rm(temp, { recursive: true, force: true });
  delete globalThis.appCalls;
  delete globalThis.appResponses;
}
