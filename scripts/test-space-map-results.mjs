import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/spaceMapResults.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } });
const { removeSpaceMapEntry } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const a = { path: "/root/A/a", name: "a", is_dir: false, logical_size: 900, allocated_size: 90, file_count: 1, directory_count: 0 };
const b = { ...a, path: "/root/A/b", name: "b", logical_size: 100, allocated_size: 40 };
const sibling = { ...a, path: "/root/AB", name: "AB", allocated_size: 20 };
const dir = { ...a, path: "/root/A", name: "A", is_dir: true, logical_size: 1000, allocated_size: 130, file_count: 2, children: [a, b] };
const snapshot = { root_path: "/root", entries: [dir, sibling], allocated_size: 150, logical_size: 1900, file_count: 3, scanned_file_count: 10 };
const fileRemoved = removeSpaceMapEntry(snapshot, a.path);
assert.equal(fileRemoved.allocated_size, 60);
assert.equal(fileRemoved.file_count, 2);
assert.equal(fileRemoved.entries[0].allocated_size, 40);
assert.equal(fileRemoved.entries[0].children[0].name, "b");
assert.equal(snapshot.entries[0].children.length, 2); // original stays intact for failed requests
const folderRemoved = removeSpaceMapEntry(snapshot, dir.path);
assert.equal(folderRemoved.allocated_size, 20);
assert.equal(folderRemoved.entries[0].path, "/root/AB"); // don't remove paths with a shared prefix
const empty = removeSpaceMapEntry(fileRemoved, b.path);
assert.equal(empty.entries.length, 1); // prune now-empty ancestor
assert.equal(removeSpaceMapEntry(folderRemoved, sibling.path).allocated_size, 0);
assert.equal(fileRemoved.scanned_file_count, 10); // scan counters remain historical
console.log("Passed: file/folder removal, ancestor sizes, global totals, immutable snapshot, prefix safety and empty-result cleanup.");
