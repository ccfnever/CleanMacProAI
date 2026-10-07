import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/spaceMapLayout.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } });
const { layoutSpaceMap, spaceMapPreviewEntries } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const entry = (size, index) => ({ name: `folder-${index}`, path: `/root/${index}`, allocated_size: size });
const opposed = [{ ...entry(10, 0), logical_size: 1000 }, { ...entry(90, 1), logical_size: 1 }];
assert.equal(layoutSpaceMap(opposed)[0].entry.path, "/root/1");
assert.ok(Math.abs(layoutSpaceMap(opposed)[0].width * layoutSpaceMap(opposed)[0].height - .9) < 1e-9);
const close = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-9, `${actual} ≠ ${expected}`);
function verify(sizes, aspect) {
  const items = sizes.map(entry);
  const tiles = layoutSpaceMap(items, aspect);
  assert.equal(tiles.length, sizes.length);
  const total = sizes.reduce((sum, size) => sum + size, 0);
  close(tiles.reduce((sum, tile) => sum + tile.width * tile.height, 0), 1);
  for (const tile of tiles) {
    assert.ok(tile.x >= 0 && tile.y >= 0 && tile.x + tile.width <= 1 + 1e-9 && tile.y + tile.height <= 1 + 1e-9);
    close(tile.width * tile.height, tile.entry.allocated_size / total);
  }
  for (let a = 0; a < tiles.length; a++) for (let b = a + 1; b < tiles.length; b++) {
    const left = tiles[a], right = tiles[b];
    const overlapWidth = Math.min(left.x + left.width, right.x + right.width) - Math.max(left.x, right.x);
    const overlapHeight = Math.min(left.y + left.height, right.y + right.height) - Math.max(left.y, right.y);
    assert.ok(overlapWidth <= 1e-9 || overlapHeight <= 1e-9, "tiles overlap");
  }
}
assert.deepEqual(layoutSpaceMap([]), []);
assert.deepEqual(layoutSpaceMap([entry(0, 0), entry(NaN, 1), entry(-1, 2)]), []);
for (const aspect of [.4, 1, 2, 4, 8]) {
  verify([100], aspect);
  verify([999999, 1, 1, 1], aspect);
  // Growing snapshots, changing proportions and 32-entry progress cap.
  for (let count = 1; count <= 33; count++) verify(Array.from({ length: count }, (_, i) => ((i * 7919 + count * 101) % 10000) + 1), aspect);
}
const preview = spaceMapPreviewEntries([entry(70, 0), entry(20, 1)], 100, "/root");
assert.equal(preview.length, 3);
assert.equal(preview[2].allocated_size, 10);
close(layoutSpaceMap(preview)[0].width * layoutSpaceMap(preview)[0].height, .7);
assert.equal(spaceMapPreviewEntries([entry(100, 0)], 100, "/root").length, 1);
console.log("Passed: fixed bounds, 100% coverage, proportional areas, no overlap, growing snapshots and omitted-content aggregation.");
