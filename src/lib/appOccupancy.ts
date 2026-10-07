import type { InstalledApp } from "./demoData";

export type AppSort = "size" | "opened" | "name";

export function compareApps(a: InstalledApp, b: InstalledApp, sort: AppSort): number {
  if (sort === "size") {
    const difference = b.app_size + b.related_size - a.app_size - a.related_size;
    if (difference) return difference;
  }
  if (sort === "opened") {
    const aTime = a.last_opened_at ?? -Infinity;
    const bTime = b.last_opened_at ?? -Infinity;
    if (aTime !== bTime) return bTime > aTime ? 1 : -1;
  }
  return a.name.localeCompare(b.name, "zh-Hans-CN") || a.app_path.localeCompare(b.app_path);
}

export function formatLastOpened(timestamp?: number | null): string {
  if (timestamp == null || !Number.isFinite(timestamp)) return "暂无打开记录";
  return new Date(timestamp * 1000).toLocaleString("zh-CN", {
    year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false,
  });
}

export function uniqueAppsTotalSize(apps: InstalledApp[]): number {
  const entries = new Map<string, number>();
  for (const app of apps) {
    entries.set(app.app_path.replace(/\/$/, ""), app.app_size);
    for (const file of app.related_files) {
      entries.set(file.path.replace(/\/$/, ""), file.size);
    }
  }
  // Shared containers belong to one physical path, even for multiple app copies.
  const roots: string[] = [];
  let total = 0;
  for (const [path, size] of [...entries].sort(([a], [b]) => a.length - b.length)) {
    if (roots.some((root) => path.startsWith(`${root}/`))) continue;
    roots.push(path);
    total += size;
  }
  return total;
}
