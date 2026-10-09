import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import ts from "typescript";
import { createPinia, setActivePinia } from "pinia";
setActivePinia(createPinia());

const temp = await mkdtemp(fileURLToPath(new URL(".app-test-", import.meta.url)));
const transpile = (source) => ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
}).outputText;
try {
  const view = await readFile(new URL("../src/views/UninstallerView.vue", import.meta.url), "utf8");
  const script = view.match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1];
  await writeFile(join(temp, "view.mjs"), transpile(`${script}\nexport { apps, inspectApp, toggleSelected, toggleExpanded, selectedApps, expandedAppPaths, uninstallSelected, requestUninstall, isConfirmingUninstall, dataSource, sortBy, filteredApps };`)
    .replaceAll('"vue"', '"./vue.mjs"')
    .replaceAll('"../lib/demoData"', '"./boundary.mjs"')
    .replaceAll('"../lib/appOccupancy"', '"./occupancy.mjs"')
    .replaceAll('"../stores/disk"', '"./disk.mjs"'));
  await writeFile(join(temp, "occupancy.mjs"), transpile(await readFile(new URL("../src/lib/appOccupancy.ts", import.meta.url), "utf8")));
  await writeFile(join(temp, "vue.mjs"), 'export { computed, ref } from "vue"; export const onMounted = () => {}; export const onUnmounted = () => {};');
  await writeFile(join(temp, "boundary.mjs"), `
    export const formatBytes = String;
    export const demoDiskInfo = {};
    export async function invokeOrDemo(command, fallback, args) {
      globalThis.appCalls.push({ command, args });
      const next = globalThis.appResponses.shift();
      return next?.__error ? { source: "error", error: next.__error } : { source: "native", data: next };
    }
  `);
  await writeFile(join(temp, "disk.mjs"), transpile(await readFile(new URL("../src/stores/disk.ts", import.meta.url), "utf8"))
    .replaceAll('"../lib/demoData"', '"./boundary.mjs"'));
  const { useDiskStore } = await import(pathToFileURL(join(temp, "disk.mjs")));
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
  const disk = {total_bytes:1000, available_bytes:370, used_bytes:630, usage_percent:63};
  globalThis.appResponses = [{ cleaned_count: 1, freed_bytes: 70, skipped_count: 0, errors: [] }, disk];
  const before = globalThis.appCalls.length;
  await state.uninstallSelected();
  assert.equal(globalThis.appCalls.length, before); // no deletion before confirmation
  state.requestUninstall();
  assert.equal(state.isConfirmingUninstall.value, true);
  await state.uninstallSelected(); // mocked boundary: never uninstall real applications
  assert.deepEqual(state.apps.value.map((app) => app.app_path), [a.app_path]);
  assert.equal(globalThis.appCalls.at(-2).args.appPath, b.app_path);
  assert.equal(globalThis.appCalls.at(-1).command, "get_disk_info");
  assert.deepEqual(useDiskStore().diskInfo, disk);
  state.toggleSelected(a.app_path);
  state.requestUninstall();
  const partialDisk = {...disk, available_bytes:400};
  globalThis.appResponses = [{cleaned_count:1, freed_bytes:30, skipped_count:1, errors:[{path:"~/Library/Caches/test.archive", reason:"locked"}]}, partialDisk];
  await state.uninstallSelected();
  assert.deepEqual(useDiskStore().diskInfo, partialDisk);
  state.apps.value = [a];
  state.toggleSelected(a.app_path);
  state.requestUninstall();
  globalThis.appResponses = [{__error:"uninstall interrupted"}, {__error:"disk unavailable"}];
  await state.uninstallSelected();
  assert.deepEqual(useDiskStore().diskInfo, partialDisk);
  assert.match(useDiskStore().diskNotice, /disk unavailable/);
  assert.equal(globalThis.appCalls.at(-1).command, "get_disk_info");
  assert.equal(state.apps.value.length, 1);
  console.log("Passed: confirmation guards deletion; uninstall refreshes shared disk state on success, partial completion and errors.");
  console.log("Passed: same-ID copies are inspected, expanded, selected and removed independently by path.");
} finally {
  await rm(temp, { recursive: true, force: true });
  delete globalThis.appCalls;
  delete globalThis.appResponses;
}
