import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { createPinia, setActivePinia } from "pinia";
import ts from "typescript";

// Exercise the real stores with a command boundary stub; never touch user files.
const temp = await mkdtemp(fileURLToPath(new URL(".cleanup-test-", import.meta.url)));
try {
  for (const name of ["disk", "scanner"]) {
    const source = await readFile(new URL(`../src/stores/${name}.ts`, import.meta.url), "utf8");
    const { outputText } = ts.transpileModule(source, {
      compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
    });
    await writeFile(join(temp, `${name}.mjs`), outputText
      .replaceAll('"../lib/demoData"', '"./boundary.mjs"')
      .replaceAll('"./disk"', '"./disk.mjs"'));
  }
  await writeFile(join(temp, "boundary.mjs"), `
    export const demoDiskInfo = { total_bytes: 1000, available_bytes: 100, used_bytes: 900, usage_percent: 90 };
    export const demoScanResult = {};
    export const demoCleanReport = {};
    export async function invokeOrDemo(command, fallback, args) {
      globalThis.cleanupCalls.push({ command, args });
      const next = globalThis.cleanupResponses.shift();
      if (!next || next.command !== command) throw new Error("Unexpected command: " + command);
      return next.result;
    }
  `);
  const { useScannerStore } = await import(pathToFileURL(join(temp, "scanner.mjs")));
  const { useDiskStore } = await import(pathToFileURL(join(temp, "disk.mjs")));
  const disk = { volume_name: "Test", total_bytes: 1000, available_bytes: 300, used_bytes: 700, usage_percent: 70 };
  const category = { id: "app_logs", name: "Logs", risk: "low", total_size: 200, file_count: 1, files: [] };
  const report = { cleaned_count: 1, freed_bytes: 200, skipped_count: 0, errors: [] };
  function setup(responses) {
    setActivePinia(createPinia());
    globalThis.cleanupCalls = [];
    globalThis.cleanupResponses = responses;
    const scanner = useScannerStore();
    scanner.dataSource = "native";
    scanner.scanResults = [category];
    scanner.selectedCategories = new Set([category.id]);
    return scanner;
  }
  function responses(cleanReport = report, scan = { source: "native", data: { categories: [], scan_duration_ms: 1 } }) {
    return [
      { command: "clean_categories", result: { source: "native", data: cleanReport } },
      { command: "get_disk_info", result: { source: "native", data: disk } },
      { command: "scan_system", result: scan },
    ];
  }

  let scanner = setup(responses());
  await scanner.cleanSelected();
  assert.deepEqual(useDiskStore().diskInfo, disk);
  assert.equal(useDiskStore().dataSource, "native");
  assert.equal(scanner.cleanProgress, 100);
  assert.equal(scanner.isCleaning, false);
  assert.deepEqual(globalThis.cleanupCalls.map(({ command }) => command), ["clean_categories", "get_disk_info", "scan_system"]);
  assert.deepEqual(globalThis.cleanupCalls[0].args, { categoryIds: [category.id], mode: "trash", permanentConfirmed: false });

  scanner = setup(responses({ ...report, skipped_count: 1, errors: [{ path: "locked", reason: "permission" }] }, { source: "error", error: "scan failed" }));
  await scanner.cleanSelected();
  assert.deepEqual(useDiskStore().diskInfo, disk); // refresh even on partial success and rescan failure
  assert.match(scanner.notice, /清理部分完成/);

  const failedDisk = responses();
  failedDisk[1].result = { source: "error", error: "disk unavailable" };
  scanner = setup(failedDisk);
  const previous = { ...disk, available_bytes: 120, used_bytes: 880, usage_percent: 88 };
  useDiskStore().diskInfo = previous;
  await scanner.cleanSelected();
  assert.deepEqual(useDiskStore().diskInfo, previous); // preserve last real reading
  assert.match(useDiskStore().diskNotice, /disk unavailable/);
  assert.equal(scanner.cleanProgress, 100);

  scanner = setup([{ command: "clean_categories", result: { source: "error", error: "cleanup failed" } }]);
  await scanner.cleanSelected();
  assert.equal(globalThis.cleanupCalls.length, 1);
  assert.equal(scanner.isCleaning, false);
  assert.match(scanner.notice, /cleanup failed/);

  scanner = setup(responses({ ...report, processed_bytes: 200, deletion_mode: "permanent" }));
  scanner.deletionMode = "permanent";
  await scanner.cleanSelected();
  assert.equal(globalThis.cleanupCalls.length, 0); // permanent mode alone is not consent
  assert.match(scanner.notice, /明确确认/);
  await scanner.cleanSelected(true);
  assert.deepEqual(globalThis.cleanupCalls[0].args, { categoryIds: [category.id], mode: "permanent", permanentConfirmed: true });
  assert.equal(scanner.cleanReport.deletion_mode, "permanent");

  const actualDemoSource = await readFile(new URL("../src/lib/demoData.ts", import.meta.url), "utf8");
  await writeFile(join(temp, "demo.mjs"), ts.transpileModule(actualDemoSource, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
  }).outputText);
  const { demoScanResult: actualDemo } = await import(pathToFileURL(join(temp, "demo.mjs")));
  assert.equal(actualDemo.total_size, actualDemo.categories.reduce((total, item) => total + item.total_size, 0));
  assert.equal(actualDemo.categories.find((item) => item.id === "xcode_archives").risk, "high");
  assert.equal(actualDemo.categories.find((item) => item.id === "xcode_device_support").risk, "medium");
  assert.ok(actualDemo.categories.filter((item) => item.risk === "low").every((item) => !item.description.includes("Archives") && item.files.every((file) => !/Archives|DeviceSupport/.test(file.path))));
  scanner = setup([{ command: "scan_system", result: { source: "demo", data: actualDemo } }]);
  globalThis.window = { setTimeout };
  try {
    await scanner.startScan();
    assert.equal(scanner.selectedCategories.has("xcode_archives"), false);
    assert.equal(scanner.selectedCategories.has("xcode_device_support"), false);
    scanner.toggleAllCategories();
    assert.equal(scanner.selectedCategories.has("xcode_archives"), false);
  } finally { delete globalThis.window; }

  scanner = setup([]);
  scanner.dataSource = "demo";
  await scanner.cleanSelected();
  assert.equal(globalThis.cleanupCalls.length, 0);
  console.log("Passed: cleanup refreshes disk totals/usage, partial success, failed rescan, retained reading on disk errors, cleanup failure and demo guard.");
} finally {
  await rm(temp, { recursive: true, force: true });
  delete globalThis.cleanupCalls;
  delete globalThis.cleanupResponses;
}
