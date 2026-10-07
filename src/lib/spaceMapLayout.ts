import type { SpaceMapEntry } from "./demoData";

export interface SpaceMapTile {
  entry: SpaceMapEntry;
  x: number;
  y: number;
  width: number;
  height: number;
}

// The backend caps progress at 32 entries. Keep omitted bytes in the preview's 100%.
export function spaceMapPreviewEntries(entries: SpaceMapEntry[], total: number, rootPath: string): SpaceMapEntry[] {
  const visible = entries.filter((entry) => Number.isFinite(entry.allocated_size) && entry.allocated_size > 0);
  const visibleSize = visible.reduce((sum, entry) => sum + entry.allocated_size, 0);
  const remaining = Math.max(0, total - visibleSize);
  if (!remaining) return visible;
  return [...visible, {
    name: "其余已发现内容", path: `${rootPath}/__space_map_remainder__`,
    logical_size: 0, allocated_size: remaining, file_count: 0, directory_count: 0,
    is_dir: true, is_package: false, is_cloud_placeholder: false,
  }];
}

// Partition one fixed unit rectangle. Every leaf area is its share of the total;
// each split uses the parent's remaining extent, so no gaps or overlaps accumulate.
export function layoutSpaceMap(entries: SpaceMapEntry[], aspectRatio = 3): SpaceMapTile[] {
  const sorted = entries.filter((entry) => Number.isFinite(entry.allocated_size) && entry.allocated_size > 0)
    .sort((a, b) => b.allocated_size - a.allocated_size || a.path.localeCompare(b.path));
  const tiles: SpaceMapTile[] = [];
  const ratio = Number.isFinite(aspectRatio) && aspectRatio > 0 ? aspectRatio : 3;
  function partition(items: SpaceMapEntry[], total: number, x: number, y: number, width: number, height: number) {
    if (items.length === 1) {
      tiles.push({ entry: items[0], x, y, width, height });
      return;
    }
    let split = 1;
    let firstSize = items[0].allocated_size;
    while (split < items.length - 1 && Math.abs(firstSize + items[split].allocated_size - total / 2) < Math.abs(firstSize - total / 2)) {
      firstSize += items[split].allocated_size;
      split += 1;
    }
    const fraction = firstSize / total;
    if (width * ratio >= height) {
      const firstWidth = width * fraction;
      partition(items.slice(0, split), firstSize, x, y, firstWidth, height);
      partition(items.slice(split), total - firstSize, x + firstWidth, y, width - firstWidth, height);
    } else {
      const firstHeight = height * fraction;
      partition(items.slice(0, split), firstSize, x, y, width, firstHeight);
      partition(items.slice(split), total - firstSize, x, y + firstHeight, width, height - firstHeight);
    }
  }
  if (sorted.length) partition(sorted, sorted.reduce((sum, entry) => sum + entry.allocated_size, 0), 0, 0, 1, 1);
  return tiles;
}
