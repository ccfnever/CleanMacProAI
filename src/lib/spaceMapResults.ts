import type { SpaceMapEntry, SpaceMapResult } from "./demoData";

// Remove only after the native trash operation succeeds. Update all cached ancestors
// so the chart, percentages and global file ranking agree without a new scan.
export function removeSpaceMapEntry(snapshot: SpaceMapResult, path: string): SpaceMapResult {
  function remove(items: SpaceMapEntry[]): SpaceMapEntry[] {
    return items.flatMap((entry) => {
      if (entry.path === path) return [];
      if (!entry.children?.length) return [entry];
      const children = remove(entry.children);
      if (!children.length) return [];
      return [{ ...entry, children,
        logical_size: children.reduce((sum, child) => sum + child.logical_size, 0),
        allocated_size: children.reduce((sum, child) => sum + child.allocated_size, 0),
        file_count: children.reduce((sum, child) => sum + child.file_count, 0),
        directory_count: children.reduce((sum, child) => sum + (child.is_dir ? 1 + child.directory_count : 0), 0),
      }];
    });
  }
  const entries = remove(snapshot.entries);
  return { ...snapshot, entries,
    logical_size: entries.reduce((sum, entry) => sum + entry.logical_size, 0),
    allocated_size: entries.reduce((sum, entry) => sum + entry.allocated_size, 0),
    file_count: entries.reduce((sum, entry) => sum + entry.file_count, 0),
  };
}
