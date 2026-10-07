<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useThemeStore } from "../stores/theme";
import { storeToRefs } from "pinia";
import * as echarts from "echarts/core";
import { TreemapChart } from "echarts/charts";
import { TooltipComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { EChartsOption } from "echarts";
import AppIcon from "../components/AppIcon.vue";
import { layoutSpaceMap, spaceMapPreviewEntries } from "../lib/spaceMapLayout";
import { removeSpaceMapEntry } from "../lib/spaceMapResults";
import {
  demoSpaceMapResult,
  formatBytes,
  invokeOrDemo,
  type InvokeResult,
  type SpaceMapEntry,
  type SpaceMapProgress,
  type SpaceMapResult,
} from "../lib/demoData";

echarts.use([TreemapChart, TooltipComponent, CanvasRenderer]);

type SortKey = "allocated" | "name";

const themeStore = useThemeStore();
const { currentTheme } = storeToRefs(themeStore);
const chartElement = ref<HTMLDivElement | null>(null);
const liveChartElement = ref<HTMLDivElement | null>(null);
const liveAspectRatio = ref(3);
let liveResizeObserver: ResizeObserver | null = null;
const scanResult = ref<SpaceMapResult | null>(null);
const currentPath = ref("");
const selectedEntry = ref<SpaceMapEntry | null>(null);
const selectedFileElement = ref<HTMLElement | null>(null);
const stopping = ref(false);
const trashDialog = ref<HTMLDialogElement | null>(null);
const pendingTrash = ref<SpaceMapEntry | null>(null);
const deleting = ref(false);
const trashError = ref("");
const noticeMessage = ref("");
const listMode = ref<"directory" | "files">("directory");
const entryIndex = computed(() => {
  const index = new Map<string, SpaceMapEntry>();
  const visit = (items: SpaceMapEntry[]) => {
    for (const entry of items) { index.set(entry.path, entry); visit(entry.children ?? []); }
  };
  visit(scanResult.value?.entries ?? []);
  return index;
});
const result = computed<SpaceMapResult | null>(() => {
  const snapshot = scanResult.value;
  if (!snapshot) return null;
  const directory = entryIndex.value.get(currentPath.value);
  if (!directory) return snapshot;
  return { ...snapshot, root_path: directory.path, display_path: directory.path,
    logical_size: directory.logical_size, allocated_size: directory.allocated_size,
    file_count: directory.file_count, directory_count: directory.directory_count,
    entries: directory.children ?? [] };
});
const loading = ref(false);
const cancelled = ref(false);
const errorMessage = ref("");
const source = ref<"native" | "demo">("demo");
const query = ref("");
const sortKey = ref<SortKey>("allocated");
const scopeRoot = ref("");
const minimumFileSize = ref(100 * 1024 * 1024);
const progress = ref<SpaceMapProgress>(emptyProgress());
let chart: ReturnType<typeof echarts.init> | null = null;
let resizeObserver: ResizeObserver | null = null;
let progressTimer: number | undefined;
let progressRequestInFlight = false;
let requestSequence = 0;

const isNativeRuntime = Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

const visualizationEntries = computed(() =>
  (loading.value ? progress.value.entries : result.value?.entries ?? [])
    .filter((entry) => entry.allocated_size > 0),
);

const liveTiles = computed(() => layoutSpaceMap(spaceMapPreviewEntries(
  progress.value.entries, progress.value.matched_allocated_size, progress.value.root_path,
), liveAspectRatio.value));

const thresholdLabel = computed(() => formatThreshold(minimumFileSize.value));

const entries = computed(() => {
  const keyword = query.value.trim().toLocaleLowerCase("zh-CN");
  const candidates = listMode.value === "files"
    ? [...entryIndex.value.values()].filter((entry) => !entry.is_dir)
    : result.value?.entries ?? [];
  const items = candidates.filter((entry) =>
    !keyword || `${entry.name} ${entry.path}`.toLocaleLowerCase("zh-CN").includes(keyword),
  );
  return [...items].sort((left, right) => {
    if (sortKey.value === "name") return left.name.localeCompare(right.name, "zh-CN");
    const field = "allocated_size";
    return right[field] - left[field] || left.name.localeCompare(right.name, "zh-CN");
  });
});

const breadcrumbItems = computed(() => {
  if (!result.value) {
    if (!loading.value || !progress.value.root_path) return [];
    return [{
      label: progress.value.display_path || progress.value.root_path,
      path: progress.value.root_path,
    }];
  }
  const root = scopeRoot.value || result.value.root_path;
  const current = result.value.root_path;
  const rootParts = root.split("/").filter(Boolean);
  const rootName = root === "/"
    ? "Macintosh HD"
    : rootParts[rootParts.length - 1] || root;
  const items = [{ label: rootName, path: root }];
  if (current === root) return items;
  const relative = current.startsWith(root === "/" ? "/" : `${root}/`) ? current.slice(root === "/" ? 1 : root.length + 1) : "";
  let path = root;
  for (const part of relative.split("/").filter(Boolean)) {
    path = `${path === "/" ? "" : path}/${part}`;
    items.push({ label: part, path });
  }
  return items;
});


function percentage(entry: SpaceMapEntry) {
  const total = (listMode.value === "files" ? scanResult.value : result.value)?.allocated_size ?? 0;
  if (total <= 0) return "0%";
  const value = entry.allocated_size / total * 100;
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)}%`;
}

function formatCount(value: number) {
  return new Intl.NumberFormat("zh-CN").format(value);
}

function formatThreshold(value: number) {
  return formatBytes(value).replace(".0 ", " ");
}


function emptyProgress(): SpaceMapProgress {
  return {
    is_scanning: false,
    root_path: "",
    display_path: "",
    minimum_file_size: minimumFileSize.value,
    scanned_file_count: 0,
    scanned_directory_count: 0,
    matched_file_count: 0,
    matched_logical_size: 0,
    matched_allocated_size: 0,
    ignored_file_count: 0,
    ignored_logical_size: 0,
    skipped_items: 0,
    hard_link_duplicates: 0,
    symlink_count: 0,
    entries: [],
    elapsed_ms: 0,
  };
}

function disposeChart() {
  resizeObserver?.disconnect();
  resizeObserver = null;
  chart?.dispose();
  chart = null;
}

async function analyze(path?: string, resetScope = false) {
  if (loading.value || deleting.value) return;
  const sequence = ++requestSequence;
  stopProgressPolling();
  disposeChart();
  loading.value = true;
  source.value = isNativeRuntime ? "native" : "demo";
  cancelled.value = false;
  stopping.value = false;
  errorMessage.value = "";
  noticeMessage.value = "";
  progress.value = {
    ...emptyProgress(),
    is_scanning: true,
    root_path: path ?? result.value?.root_path ?? "~",
    display_path: path ?? result.value?.display_path ?? "~",
    minimum_file_size: minimumFileSize.value,
  };
  await nextTick();
  renderChart();

  const fallback = demoResultFor(path);
  let response: InvokeResult<SpaceMapResult>;
  if (isNativeRuntime) {
    startProgressPolling(sequence);
    response = await invokeOrDemo<SpaceMapResult>("analyze_space_map", fallback, {
      ...(path ? { path } : {}),
      minimumFileSize: minimumFileSize.value,
    });
  } else {
    await simulateProgress(fallback, sequence);
    response = { source: "demo" as const, data: stopping.value ? partialDemoResult() : fallback };
  }
  if (sequence !== requestSequence) return;
  stopProgressPolling();

  if (response.source === "error") {
    disposeChart();
    loading.value = false;
    errorMessage.value = response.error;
    return;
  }

  scanResult.value = response.data;
  currentPath.value = response.data.root_path;
  selectedEntry.value = null;
  cancelled.value = Boolean(response.data.incomplete);
  stopping.value = false;
  source.value = response.source === "native" ? "native" : "demo";
  if (resetScope || !scopeRoot.value) scopeRoot.value = response.data.root_path;
  disposeChart();
  loading.value = false;
  await nextTick();
  renderChart();
}

function startProgressPolling(sequence: number) {
  const poll = async () => {
    if (progressRequestInFlight || sequence !== requestSequence) return;
    progressRequestInFlight = true;
    try {
      const snapshot = await invoke<SpaceMapProgress>("get_space_map_progress");
      if (sequence !== requestSequence) return;
      progress.value = snapshot;
    } catch {
      // 最终 analyze_space_map 调用负责呈现错误，轮询失败不打断扫描。
    } finally {
      progressRequestInFlight = false;
    }
  };
  progressTimer = window.setInterval(() => void poll(), 250);
}

function stopProgressPolling() {
  if (progressTimer !== undefined) window.clearInterval(progressTimer);
  progressTimer = undefined;
  progressRequestInFlight = false;
}

async function stopAnalysis() {
  stopping.value = true;
  if (isNativeRuntime) {
    try { await invoke("cancel_space_map"); }
    catch (error) { stopping.value = false; errorMessage.value = String(error); }
  }
}

function partialDemoResult(): SpaceMapResult {
  const snapshot = progress.value;
  return { root_path: snapshot.root_path, display_path: snapshot.display_path,
    minimum_file_size: snapshot.minimum_file_size,
    logical_size: snapshot.matched_logical_size, allocated_size: snapshot.matched_allocated_size,
    file_count: snapshot.matched_file_count, directory_count: snapshot.scanned_directory_count,
    scanned_file_count: snapshot.scanned_file_count, ignored_file_count: snapshot.ignored_file_count,
    ignored_logical_size: snapshot.ignored_logical_size, skipped_items: snapshot.skipped_items,
    hard_link_duplicates: snapshot.hard_link_duplicates, symlink_count: snapshot.symlink_count,
    entries: snapshot.entries, scan_duration_ms: snapshot.elapsed_ms, incomplete: true };
}

async function simulateProgress(finalResult: SpaceMapResult, sequence: number) {
  const steps = 10;
  for (let step = 1; step <= steps; step += 1) {
    if (sequence !== requestSequence || stopping.value) return;
    const entries = finalResult.entries
      .slice(0, Math.max(1, Math.ceil(step * finalResult.entries.length / steps)))
      .map((entry) => scaleDemoEntry(entry, step / steps));
    const factor = step / steps;
    progress.value = {
      ...emptyProgress(),
      is_scanning: true,
      root_path: finalResult.root_path,
      display_path: finalResult.display_path,
      current_path: entries[entries.length - 1]?.path.replace("/Users/demo", "~"),
      minimum_file_size: minimumFileSize.value,
      scanned_file_count: Math.round(finalResult.scanned_file_count * factor),
      scanned_directory_count: Math.round(finalResult.directory_count * factor),
      matched_file_count: entries.reduce((sum, entry) => sum + entry.file_count, 0),
      matched_logical_size: entries.reduce((sum, entry) => sum + entry.logical_size, 0),
      matched_allocated_size: entries.reduce((sum, entry) => sum + entry.allocated_size, 0),
      ignored_file_count: Math.round(finalResult.ignored_file_count * factor),
      ignored_logical_size: Math.round(finalResult.ignored_logical_size * factor),
      skipped_items: 0,
      hard_link_duplicates: Math.round(finalResult.hard_link_duplicates * factor),
      symlink_count: Math.round(finalResult.symlink_count * factor),
      entries,
      elapsed_ms: step * 130,
    };
    await new Promise<void>((resolve) => window.setTimeout(resolve, 130));
  }
}

function scaleDemoEntry(entry: SpaceMapEntry, factor: number): SpaceMapEntry {
  const children = entry.children?.map((child) => scaleDemoEntry(child, factor));
  return { ...entry, children,
    logical_size: children?.length ? children.reduce((sum, child) => sum + child.logical_size, 0) : Math.round(entry.logical_size * factor),
    allocated_size: children?.length ? children.reduce((sum, child) => sum + child.allocated_size, 0) : Math.round(entry.allocated_size * factor),
  };
}

function demoResultFor(path?: string): SpaceMapResult {
  const base = demoSpaceMapResult;
  const entries = base.entries.map((entry) => {
    if (!entry.is_dir) return entry;
    const names = ["Archives", "Media", "Builds"];
    const children = names.map((name, index) => {
      const ratio = [.5, .3, .2][index];
      const directory = { ...entry, name, path: `${entry.path}/${name}`,
        logical_size: Math.round(entry.logical_size * ratio), allocated_size: Math.round(entry.allocated_size * ratio) };
      return { ...directory, children: [{ ...directory, name: `${name.toLowerCase()}.zip`,
        path: `${directory.path}/${name.toLowerCase()}.zip`, is_dir: false, is_package: false, file_count: 1, directory_count: 0 }] };
    });
    return { ...entry, children, file_count: children.length, directory_count: children.length };
  });
  const filter = (items: SpaceMapEntry[]): SpaceMapEntry[] => items.flatMap((entry) => {
    if (!entry.is_dir) return entry.allocated_size >= minimumFileSize.value ? [entry] : [];
    const children = filter(entry.children ?? []);
    if (!children.length) return [];
    return [{ ...entry, children,
      logical_size: children.reduce((sum, child) => sum + child.logical_size, 0),
      allocated_size: children.reduce((sum, child) => sum + child.allocated_size, 0),
      file_count: children.reduce((sum, child) => sum + child.file_count, 0) }];
  });
  const filtered = filter(entries);
  const count = filtered.reduce((sum, entry) => sum + entry.file_count, 0);
  return { ...base, root_path: path ?? base.root_path, display_path: path ?? base.display_path,
    minimum_file_size: minimumFileSize.value, entries: filtered,
    logical_size: filtered.reduce((sum, entry) => sum + entry.logical_size, 0),
    allocated_size: filtered.reduce((sum, entry) => sum + entry.allocated_size, 0),
    file_count: count, ignored_file_count: base.scanned_file_count - count };
}

function browseDirectory(path: string) {
  if (path !== scanResult.value?.root_path && !entryIndex.value.get(path)?.is_dir) return;
  currentPath.value = path;
  query.value = "";
  selectedEntry.value = null;
  listMode.value = "directory";
}

async function chooseDirectory() {
  errorMessage.value = "";
  if (!isNativeRuntime) {
    await analyze("/Users/demo", true);
    return;
  }
  try {
    const selected = await invoke<string | null>("choose_space_map_directory");
    if (selected) await analyze(selected, true);
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

function openEntry(entry: SpaceMapEntry) {
  if (entry.is_dir) { browseDirectory(entry.path); return; }
  selectedEntry.value = entry;
  void nextTick(() => selectedFileElement.value?.scrollIntoView({ behavior: "smooth", block: "nearest" }));
}

async function revealInFinder(path: string) {
  if (!isNativeRuntime) {
    noticeMessage.value = `预览模式：本机运行时将在 Finder 中显示 ${path}`;
    return;
  }
  try {
    await invoke("open_in_finder", { path });
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

async function requestTrash(entry: SpaceMapEntry) {
  if (loading.value || deleting.value || !entryIndex.value.has(entry.path)) return;
  pendingTrash.value = entry;
  trashError.value = "";
  await nextTick();
  trashDialog.value?.showModal();
}

function cancelTrash() {
  if (deleting.value) return;
  trashDialog.value?.close();
  pendingTrash.value = null;
  trashError.value = "";
}

async function confirmTrash() {
  const entry = pendingTrash.value;
  const snapshot = scanResult.value;
  if (!entry || !snapshot || deleting.value) return;
  deleting.value = true;
  trashError.value = "";
  try {
    if (isNativeRuntime) {
      await invoke("trash_space_map_entry", { path: entry.path, rootPath: snapshot.root_path });
    }
    scanResult.value = removeSpaceMapEntry(snapshot, entry.path);
    if (selectedEntry.value?.path === entry.path || selectedEntry.value?.path.startsWith(`${entry.path}/`)) selectedEntry.value = null;
    if (currentPath.value !== snapshot.root_path && !entryIndex.value.has(currentPath.value)) currentPath.value = snapshot.root_path;
    noticeMessage.value = isNativeRuntime
      ? `“${entry.name}”已移入废纸篓，可在 Finder 的废纸篓中恢复。`
      : `预览已移除“${entry.name}”，未操作本机文件。`;
    trashDialog.value?.close();
    pendingTrash.value = null;
  } catch (error) {
    trashError.value = error instanceof Error ? error.message : String(error);
  } finally {
    deleting.value = false;
  }
}

async function requestFullDiskAccess() {
  if (!isNativeRuntime) return;
  try {
    await invoke("request_permissions");
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

function renderChart() {
  if (loading.value || !chartElement.value) return;
  const currentElement = chartElement.value;
  chart ??= echarts.init(currentElement, undefined, { renderer: "canvas" });
  const currentChart = chart;
  resizeObserver?.disconnect();
  resizeObserver = new ResizeObserver(() => currentChart.resize());
  resizeObserver.observe(currentElement);
  const styles = getComputedStyle(document.documentElement);
  const surface = styles.getPropertyValue("--surface").trim();
  const text = styles.getPropertyValue("--text").trim();
  const chartEntries = visualizationEntries.value;
  const palettes: Record<string, string[]> = {
    pet: ["#8a4f31", "#ad704d", "#cf9b75", "#e2bea1", "#ecd4be", "#f4e3d4"],
    nature: ["#365d43", "#527a5d", "#78a07d", "#a6c0a2", "#c5d6bf", "#e0e9dc"],
    classic: ["#3c6078", "#587c94", "#7e9db0", "#abc0cd", "#c8d6de", "#e1e8ed"],
  };
  const palette = palettes[currentTheme.value] ?? palettes.pet;
  const option: EChartsOption = {
    animation: !loading.value,
    animationDuration: 0,
    animationDurationUpdate: 0,
    tooltip: {
      confine: true,
      formatter(params: unknown) {
        const data = (params as { data?: SpaceMapEntry & { value: number } }).data;
        if (!data) return "";
        return `<strong>${escapeHtml(data.name)}</strong><br>磁盘占用 ${formatBytes(data.allocated_size)}<br>${formatCount(data.file_count)} 个文件`;
      },
    },
    series: [{
      type: "treemap",
      left: 0,
      top: 0,
      right: 0,
      bottom: 0,
      width: "100%",
      height: "100%",
      roam: false,
      nodeClick: false,
      breadcrumb: { show: false },
      visibleMin: 120,
      label: {
        show: true,
        color: text,
        fontFamily: styles.getPropertyValue("--font-ui").trim(),
        fontSize: 12,
        fontWeight: 600,
        lineHeight: 21,
      },
      upperLabel: { show: true, height: 28, color: "#fff", formatter: "{b}" },
      leafDepth: 2,
      levels: [{ itemStyle: { borderWidth: 0, gapWidth: 4 } },
        { itemStyle: { borderWidth: 3, gapWidth: 3 }, upperLabel: { show: true } },
        { itemStyle: { borderWidth: 1, gapWidth: 2 } }],
      itemStyle: { borderColor: surface, borderWidth: 4, gapWidth: 4, borderRadius: 9 },
      emphasis: { itemStyle: { shadowBlur: 14, shadowColor: "rgba(0,0,0,.14)" } },
      data: chartData(chartEntries, palette, text),
    }],
  };
  currentChart.setOption(option, { notMerge: true, lazyUpdate: false });
  currentChart.off("click");
  if (!loading.value) {
    currentChart.on("click", (params) => {
      const entry = params.data as SpaceMapEntry | undefined;
      if (entry) void openEntry(entry);
    });
  }
}

// Bound canvas data independently of the retained directory tree.
function chartData(items: SpaceMapEntry[], palette: string[], text: string, depth = 0): object[] {
  const sorted = [...items].sort((a, b) => b.allocated_size - a.allocated_size);
  const visible = sorted.slice(0, 24);
  const nodes: object[] = visible.map((entry, index) => ({
    ...entry, id: entry.path, value: entry.allocated_size,
    children: depth < 1 && entry.children?.length ? chartData(entry.children, palette, text, depth + 1) : undefined,
    itemStyle: { color: palette[index % palette.length], borderColor: palette[index % palette.length] },
    upperLabel: { color: index < 2 ? "#fff" : text, formatter: `${entry.name} · ${formatBytes(entry.allocated_size)}` },
    label: { color: index < 2 ? "#fff" : text, formatter: `${entry.name}\n${formatBytes(entry.allocated_size)}` },
  }));
  if (sorted.length > visible.length) {
    const rest = sorted.slice(24);
    const path = rest[0].path.slice(0, rest[0].path.lastIndexOf("/")) || "/";
    nodes.push({ name: `其余 ${rest.length} 项 · 点击查看目录`, path, is_dir: true,
      logical_size: rest.reduce((sum, entry) => sum + entry.logical_size, 0),
      allocated_size: rest.reduce((sum, entry) => sum + entry.allocated_size, 0),
      file_count: rest.reduce((sum, entry) => sum + entry.file_count, 0),
      value: rest.reduce((sum, entry) => sum + entry.allocated_size, 0), itemStyle: { color: palette[5] } });
  }
  return nodes;
}

function escapeHtml(value: string) {
  return value.replace(/[&<>'"]/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;",
  })[character] ?? character);
}

watch(liveChartElement, (element) => {
  liveResizeObserver?.disconnect();
  liveResizeObserver = null;
  if (!element) return;
  liveResizeObserver = new ResizeObserver(([entry]) => {
    if (entry.contentRect.width > 0 && entry.contentRect.height > 0) {
      liveAspectRatio.value = entry.contentRect.width / entry.contentRect.height;
    }
  });
  liveResizeObserver.observe(element);
}, { flush: "post" });

watch(currentTheme, () => nextTick(renderChart));
watch(visualizationEntries, () => nextTick(renderChart));

onMounted(() => {
  if (chartElement.value) {
    resizeObserver = new ResizeObserver(() => chart?.resize());
    resizeObserver.observe(chartElement.value);
  }
  void analyze(undefined, true);
});

onBeforeUnmount(() => {
  liveResizeObserver?.disconnect();
  requestSequence += 1;
  stopProgressPolling();
  if (isNativeRuntime) void invoke("cancel_space_map");
  disposeChart();
});
</script>

<template>
  <section class="space-page" aria-labelledby="space-map-title">
    <header class="space-heading">
      <div>
        <p class="eyebrow">按磁盘占用定位 · 删除前确认</p>
        <h1 id="space-map-title">快速找到真正占空间的文件</h1>
        <p>默认聚焦磁盘占用 ≥ {{ thresholdLabel }} 的文件。点击拼图逐层查看，或用大文件排行直接定位；停止扫描也能查看已发现内容。</p>
      </div>
      <div class="heading-actions">
        <label class="threshold-select">
          <span>占用标准</span>
          <select v-model.number="minimumFileSize" :disabled="loading || deleting" @change="analyze(scanResult?.root_path, true)">
            <option :value="50 * 1024 * 1024">≥ 50 MB</option>
            <option :value="100 * 1024 * 1024">≥ 100 MB</option>
            <option :value="500 * 1024 * 1024">≥ 500 MB</option>
            <option :value="1024 * 1024 * 1024">≥ 1 GB</option>
          </select>
        </label>
        <button type="button" class="secondary-button" :disabled="loading || deleting" @click="analyze(scanResult?.root_path, true)">
          <AppIcon name="refresh" :size="16" /> 重新分析
        </button>
        <button type="button" class="primary-button" :disabled="loading || deleting" @click="chooseDirectory">
          <AppIcon name="folder" :size="17" /> 扫描范围
        </button>
      </div>
    </header>

    <p v-if="errorMessage" class="error-banner" role="alert">{{ errorMessage }}</p>
    <p v-if="noticeMessage" class="partial-banner" role="status">{{ noticeMessage }}</p>

    <div class="pathbar" aria-label="当前分析路径">
      <AppIcon name="disk" :size="16" />
      <template v-for="(item, index) in breadcrumbItems" :key="item.path">
        <span v-if="index" aria-hidden="true">/</span>
        <button type="button" :disabled="loading || index === breadcrumbItems.length - 1" @click="browseDirectory(item.path)">{{ item.label }}</button>
      </template>
      <span :class="['source-chip', { native: source === 'native' }]">{{ source === "native" ? "本机扫描结果" : "界面预览数据" }}</span>
    </div>

    <section v-if="loading" class="live-scan" aria-live="polite">
      <header class="live-scan-head">
        <div class="live-title"><span class="live-dot"></span><div><strong>正在寻找磁盘占用 ≥ {{ thresholdLabel }} 的文件</strong><small>{{ progress.current_path || "正在打开扫描范围…" }}</small></div></div>
        <button type="button" :disabled="stopping" @click="stopAnalysis">{{ stopping ? "正在保存结果…" : "停止并查看" }}</button>
      </header>
      <div class="live-metrics">
        <span><small>已检查</small><strong>{{ formatCount(progress.scanned_file_count) }}</strong><em>个文件</em></span>
        <span><small>已找到</small><strong>{{ formatCount(progress.matched_file_count) }}</strong><em>个大文件</em></span>
        <span><small>累计磁盘占用</small><strong>{{ formatBytes(progress.matched_allocated_size) }}</strong><em>持续增加中</em></span>
        <span><small>用时</small><strong>{{ (progress.elapsed_ms / 1000).toFixed(1) }} 秒</strong><em>扫描中</em></span>
      </div>
      <div class="live-puzzle">
        <div ref="liveChartElement" class="live-treemap" role="img" aria-label="固定区域内按当前已发现总大小分割的空间拼图">
          <div v-if="!liveTiles.length" class="puzzle-placeholder" aria-hidden="true">
            <span v-for="index in 6" :key="index" :style="{ animationDelay: `${index * 45}ms` }"></span>
          </div>
          <div v-for="(tile, index) in liveTiles" :key="tile.entry.path"
            :class="['live-space-tile', `live-color-${index % 6}`, { 'compact-tile': tile.width < .1 || tile.height < .14 }]"
            :style="{ left: `${tile.x * 100}%`, top: `${tile.y * 100}%`, width: `${tile.width * 100}%`, height: `${tile.height * 100}%` }"
            :title="`${tile.entry.name} · ${formatBytes(tile.entry.allocated_size)}`">
            <span>{{ tile.entry.name }}</span><strong>{{ formatBytes(tile.entry.allocated_size) }}</strong>
          </div>
        </div>
        <div class="puzzle-caption"><span>整个区域代表当前已发现内容的 100%；最大 32 项单独显示，其余合并</span><strong>{{ visualizationEntries.length }} 个项目单独显示</strong></div>
      </div>
    </section>

    <section v-else-if="cancelled && !result" class="stopped-state">
      <AppIcon name="map" :size="30" />
      <strong>扫描已停止</strong>
      <p>还没有生成结果，可以调整大文件标准后重新开始。</p>
      <button type="button" class="primary-button" @click="analyze(undefined, true)">重新扫描</button>
    </section>

    <template v-else-if="result">
      <p v-if="cancelled" class="partial-banner" role="status">扫描已停止，以下是已发现的部分结果。可以点击文件夹继续查看，尚未扫描的内容不计入占用。</p>

      <section class="summary-strip" aria-label="空间分析摘要">
        <div><small>磁盘占用</small><strong>{{ formatBytes(result.allocated_size) }}</strong><span>用于拼图面积，按文件系统已分配空间统计</span></div>
        <div><small>找到的大文件</small><strong>{{ formatCount(result.file_count) }}</strong><span>共检查 {{ formatCount(result.scanned_file_count) }} 个文件</span></div>
        <div><small>{{ cancelled ? "停止时用时" : "扫描用时" }}</small><strong>{{ (result.scan_duration_ms / 1000).toFixed(1) }} 秒</strong><span>{{ result.entries.length }} 个直接子项</span></div>
      </section>

      <section class="map-panel">
        <div class="panel-title">
          <div><h2>大文件空间拼图</h2><p>按磁盘占用统计 ≥ {{ formatThreshold(result.minimum_file_size) }} 的文件，面积越大占用越大；父文件夹内展示子项，点击查看已扫描内容</p></div>
          <span>{{ result.display_path }}</span>
        </div>
        <div v-if="result.allocated_size > 0" ref="chartElement" class="treemap-chart" role="img" :aria-label="`${result.display_path} 目录占用矩形图`"></div>
        <div v-else class="empty-map"><AppIcon name="folder" :size="30" /><strong>没有找到达到当前标准的大文件</strong></div>
      </section>

      <div v-if="result.skipped_items || result.hard_link_duplicates || result.symlink_count" class="accuracy-note">
        <AppIcon name="shield" :size="18" />
        <p>
          <strong>统计边界清楚可见</strong>
          <span>已避免重复计算 {{ formatCount(result.hard_link_duplicates) }} 个硬链接，未跟随 {{ formatCount(result.symlink_count) }} 个符号链接。</span>
          <span v-if="result.skipped_items">有 {{ formatCount(result.skipped_items) }} 个条目无法读取，当前结果不包含它们。</span>
        </p>
        <button v-if="result.skipped_items" type="button" @click="requestFullDiskAccess">检查磁盘权限</button>
      </div>

      <section class="directory-section">
        <header class="directory-toolbar">
          <div><h2>{{ listMode === "files" ? "整个扫描范围的大文件" : "当前文件夹 · 从大到小" }}</h2><p>已忽略 {{ formatCount(result.ignored_file_count) }} 个小文件，让结果更聚焦</p></div>
          <div class="list-controls">
            <button type="button" :class="['mode-button', { active: listMode === 'directory' }]" @click="listMode = 'directory'; query = ''">当前目录</button>
            <button type="button" :class="['mode-button', { active: listMode === 'files' }]" @click="listMode = 'files'; query = ''">大文件排行</button>
            <label class="search-field"><AppIcon name="search" :size="15" /><input v-model="query" type="search" :placeholder="listMode === 'files' ? '搜索整个范围的文件' : '搜索当前目录'"></label>
            <select v-model="sortKey" aria-label="目录排序方式">
              <option value="allocated">按磁盘占用</option>
              <option value="name">按名称</option>
            </select>
          </div>
        </header>

      <aside v-if="selectedEntry" ref="selectedFileElement" class="selected-file" aria-label="所选文件详情">
        <div><strong>{{ selectedEntry.name }} · {{ formatBytes(selectedEntry.allocated_size) }}</strong><p>{{ selectedEntry.path }}</p></div>
        <button type="button" class="secondary-button" @click="browseDirectory(selectedEntry.path.slice(0, selectedEntry.path.lastIndexOf('/')) || '/')">查看所在目录</button>
        <button type="button" class="secondary-button" @click="revealInFinder(selectedEntry.path)">在 Finder 中显示</button>
        <button type="button" class="secondary-button" @click="selectedEntry = null">关闭</button>
      </aside>

        <div class="directory-table" role="table" aria-label="目录占用明细">
          <div class="directory-row table-head" role="row">
            <span role="columnheader">文件夹或文件</span><span role="columnheader">磁盘占用</span><span role="columnheader">内容</span><span role="columnheader">操作</span>
          </div>
          <div v-for="entry in entries.slice(0, 200)" :key="entry.path" class="directory-row" role="row">
            <span role="cell" class="entry-name-cell">
              <button type="button" class="entry-name" :aria-label="`${entry.is_dir ? '查看文件夹' : '查看文件'} ${entry.name}`" @click="openEntry(entry)">
                <span class="entry-icon"><AppIcon :name="entry.is_dir ? 'folder' : 'file'" :size="17" /></span>
                <span><strong>{{ entry.name }}<b v-if="entry.is_package">包目录</b><b v-if="entry.is_cloud_placeholder">云占位</b></strong><small v-if="listMode === 'files'">{{ entry.path }}</small></span>
                <AppIcon v-if="entry.is_dir" name="chevron" :size="13" />
              </button>
            </span>
            <span class="entry-size" role="cell"><strong>{{ formatBytes(entry.allocated_size) }}</strong><small>{{ percentage(entry) }}</small></span>
            <span class="entry-count" role="cell">{{ formatCount(entry.file_count) }} 个文件</span>
            <span class="entry-actions" role="cell">
              <button type="button" class="row-action" :aria-label="`在 Finder 中显示 ${entry.name}`" title="在 Finder 中显示" @click="revealInFinder(entry.path)"><AppIcon name="eye" :size="17" /></button>
              <button type="button" class="row-action trash-action" :disabled="deleting" :aria-label="`删除 ${entry.name}`" title="删除 · 移入废纸篓" @click="requestTrash(entry)"><AppIcon name="trash" :size="17" /></button>
            </span>
          </div>
          <div v-if="entries.length > 200" class="no-results">显示最大的 200 项，共 {{ formatCount(entries.length) }} 项；搜索可定位其余内容。</div>
          <div v-if="!entries.length" class="no-results">没有符合搜索条件的项目</div>
        </div>
      </section>

      <p class="method-note">本次按磁盘占用筛选不小于 {{ formatThreshold(result.minimum_file_size) }} 的文件。占用来自文件系统的已分配空间；APFS 克隆和快照可能共享或保留数据块，删除后可释放空间不一定等于此数值。</p>
    </template>
    <dialog ref="trashDialog" class="trash-dialog" aria-labelledby="trash-title" aria-describedby="trash-description" @cancel.prevent="cancelTrash">
      <template v-if="pendingTrash">
        <h2 id="trash-title">将“{{ pendingTrash.name }}”移入废纸篓？</h2>
        <p id="trash-description">{{ pendingTrash.is_dir ? '这个文件夹及其全部内容（包括未显示的小文件）都会移入废纸篓。' : '这个文件将移入废纸篓。' }}可从废纸篓恢复。</p>
        <p class="trash-path">{{ pendingTrash.path }}</p>
        <p class="trash-size">当前结果中的磁盘占用：{{ formatBytes(pendingTrash.allocated_size) }}</p>
        <p v-if="!isNativeRuntime" class="demo-trash-note">预览模式：仅模拟移除，不会操作本机文件。</p>
        <p v-if="trashError" class="error-banner" role="alert">{{ trashError }}</p>
        <div class="trash-dialog-actions">
          <button type="button" class="secondary-button" :disabled="deleting" autofocus @click="cancelTrash">取消</button>
          <button type="button" class="primary-button danger-button" :disabled="deleting" @click="confirmTrash">{{ deleting ? '正在移入废纸篓…' : '确认移入废纸篓' }}</button>
        </div>
      </template>
    </dialog>

  </section>
</template>

<style scoped>
.partial-banner { padding: 12px 16px; border-radius: 10px; color: var(--accent-strong); background: var(--accent-soft); font-size: 12px; }
.selected-file { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; margin: 14px 0; padding: 16px; border: 1px solid var(--border); border-radius: 12px; background: var(--surface); }
.selected-file > div { flex: 1; min-width: 0; }
.selected-file p { overflow-wrap: anywhere; color: var(--text-soft); font-size: 11px; }
.mode-button { padding: 0 10px; border: 1px solid var(--border); border-radius: 8px; background: var(--surface); color: var(--text-soft); font-size: 11px; }
.mode-button.active { color: var(--accent-strong); background: var(--accent-soft); }
button:focus-visible, select:focus-visible, input:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
.summary-strip strong, .entry-size, .live-metrics strong { font-variant-numeric: tabular-nums; }
.list-controls { flex-wrap: wrap; }

.space-page { max-width: 1240px; margin: 0 auto; padding-top: 34px; }
.space-heading { display: flex; align-items: flex-end; justify-content: space-between; gap: 30px; margin-bottom: 24px; }
.eyebrow { margin: 0 0 8px; color: var(--accent); font-size: 10px; font-weight: 800; letter-spacing: .09em; }
.space-heading h1 { margin: 0; color: var(--text); font-size: clamp(27px, 3vw, 38px); font-weight: 650; letter-spacing: -.045em; }
.space-heading p:not(.eyebrow) { max-width: 640px; margin: 9px 0 0; color: var(--text-soft); font-size: 12px; }
.heading-actions { display: flex; align-items: center; gap: 9px; }
.threshold-select { display: flex; align-items: center; gap: 8px; min-height: 40px; padding: 0 5px 0 11px; border: 1px solid var(--border); border-radius: 10px; color: var(--text-faint); background: color-mix(in srgb, var(--surface) 80%, transparent); }
.threshold-select span { font-size: 9px; font-weight: 750; white-space: nowrap; }
.threshold-select select { height: 30px; border: 0; outline: 0; color: var(--text); background: transparent; font-size: 11px; font-weight: 700; }
.primary-button, .secondary-button { display: inline-flex; align-items: center; justify-content: center; gap: 8px; min-height: 40px; padding: 0 15px; border: 1px solid var(--border); border-radius: 10px; font-size: 12px; font-weight: 700; transition: 150ms ease; }
.primary-button { border-color: var(--accent); color: #fff; background: var(--accent); }
.secondary-button { color: var(--text); background: var(--surface); }
.primary-button:hover, .secondary-button:hover { transform: translateY(-1px); box-shadow: var(--shadow-soft); }
.primary-button:disabled, .secondary-button:disabled { cursor: wait; opacity: .55; transform: none; }
.error-banner { margin: 0 0 16px; padding: 11px 13px; border: 1px solid color-mix(in srgb, var(--danger) 25%, transparent); border-radius: 10px; color: var(--danger); background: color-mix(in srgb, var(--danger) 7%, var(--surface)); font-size: 12px; }
.pathbar { display: flex; align-items: center; gap: 7px; min-height: 34px; margin-bottom: 14px; color: var(--text-faint); font-size: 11px; }
.pathbar button { padding: 3px 2px; border: 0; color: var(--accent-strong); background: transparent; }
.pathbar button:disabled { cursor: default; color: var(--text-soft); }
.source-chip { margin-left: auto; padding: 4px 8px; border-radius: 99px; color: var(--warning); background: color-mix(in srgb, var(--warning) 10%, transparent); font-size: 9px; font-weight: 800; }
.source-chip.native { color: var(--success); background: color-mix(in srgb, var(--success) 10%, transparent); }
.live-scan { overflow: hidden; border: 1px solid var(--border); border-radius: 18px; background: var(--surface); box-shadow: var(--shadow-soft); }
.live-scan-head { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 17px 19px; border-bottom: 1px solid var(--border); }
.live-title { display: flex; align-items: center; gap: 11px; min-width: 0; }
.live-dot { flex: 0 0 9px; width: 9px; height: 9px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 35%, transparent); animation: scan-pulse 1.6s ease-out infinite; }
.live-title strong, .live-title small { display: block; }
.live-title strong { font-size: 12px; }
.live-title small { max-width: 720px; margin-top: 3px; overflow: hidden; color: var(--text-faint); font-size: 9px; text-overflow: ellipsis; white-space: nowrap; }
.live-scan-head button { padding: 6px 10px; border: 1px solid var(--border); border-radius: 8px; color: var(--text-soft); background: var(--surface-soft); font-size: 10px; font-weight: 700; }
.live-metrics { display: grid; grid-template-columns: repeat(4, 1fr); border-bottom: 1px solid var(--border); background: var(--surface-soft); }
.live-metrics > span { padding: 13px 18px; border-right: 1px solid var(--border); }
.live-metrics > span:last-child { border-right: 0; }
.live-metrics small, .live-metrics strong, .live-metrics em { display: block; }
.live-metrics small { color: var(--text-faint); font-size: 9px; font-weight: 750; }
.live-metrics strong { margin: 3px 0 1px; font-size: 18px; font-style: normal; letter-spacing: -.025em; }
.live-metrics em { color: var(--text-faint); font-size: 8px; font-style: normal; }
.live-puzzle { position: relative; }
.live-treemap { position: relative; width: 100%; height: 330px; overflow: hidden; background: var(--surface-strong); }
.live-space-tile { position: absolute; display: flex; flex-direction: column; justify-content: center; align-items: center; min-width: 0; overflow: hidden; padding: 8px; border: 2px solid var(--surface); color: #fff; background: var(--accent-strong); }
.live-space-tile span, .live-space-tile strong { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.live-space-tile span { font-size: 12px; font-weight: 650; }
.live-space-tile strong { margin-top: 4px; font-size: 13px; font-variant-numeric: tabular-nums; }
.live-color-1 { background: var(--accent); }
.live-color-2 { color: var(--text); background: color-mix(in srgb, var(--accent) 55%, var(--surface)); }
.live-color-3 { color: var(--text); background: color-mix(in srgb, var(--accent) 38%, var(--surface)); }
.live-color-4 { color: var(--text); background: color-mix(in srgb, var(--accent) 25%, var(--surface)); }
.live-color-5 { color: var(--text); background: var(--accent-soft); }
.compact-tile { padding: 0; }
.compact-tile span, .compact-tile strong { display: none; }
.puzzle-placeholder { position: absolute; z-index: 1; inset: 0; display: grid; grid-template-columns: 1.7fr 1.05fr .75fr; grid-template-rows: 1fr .72fr; gap: 6px; }
.puzzle-placeholder > span { border-radius: 9px; background: var(--surface-strong); animation: tile-breathe 1.25s ease-in-out infinite alternate; }
.puzzle-placeholder > span:nth-child(n+7) { display: none; }
.puzzle-placeholder > span:first-child { grid-row: 1 / 3; }
.puzzle-caption { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 17px; color: var(--text-faint); font-size: 9px; }
.puzzle-caption strong { color: var(--accent-strong); font-size: 9px; }
.stopped-state { display: grid; place-items: center; min-height: 360px; padding: 40px; border: 1px solid var(--border); border-radius: 18px; color: var(--text-faint); background: var(--surface); text-align: center; }
.stopped-state > strong { margin-top: 12px; color: var(--text); font-size: 15px; }
.stopped-state p { margin: 5px 0 18px; font-size: 10px; }
@keyframes scan-pulse { 65%, 100% { box-shadow: 0 0 0 9px transparent; } }
@keyframes tile-breathe { from { opacity: .45; transform: scale(.985); } to { opacity: .92; transform: scale(1); } }
.summary-strip { display: grid; grid-template-columns: repeat(3, 1fr); margin-bottom: 14px; overflow: hidden; border: 1px solid var(--border); border-radius: 15px; background: var(--surface); box-shadow: var(--shadow-soft); }
.summary-strip > div { min-width: 0; padding: 16px 18px; border-right: 1px solid var(--border); }
.summary-strip > div:last-child { border-right: 0; }
.summary-strip small, .summary-strip strong, .summary-strip span { display: block; }
.summary-strip small { color: var(--text-faint); font-size: 9px; font-weight: 800; letter-spacing: .05em; }
.summary-strip strong { margin: 5px 0 2px; color: var(--text); font-size: 21px; letter-spacing: -.025em; }
.summary-strip span { color: var(--text-faint); font-size: 9px; }
.map-panel { padding: 19px; border: 1px solid var(--border); border-radius: 18px; background: var(--surface); box-shadow: var(--shadow-soft); }
.panel-title, .directory-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
.panel-title { padding: 0 3px 13px; }
.panel-title h2, .directory-toolbar h2 { margin: 0; font-size: 14px; }
.panel-title p, .directory-toolbar p { margin: 3px 0 0; color: var(--text-faint); font-size: 10px; }
.panel-title > span { max-width: 50%; overflow: hidden; color: var(--text-faint); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.treemap-chart { width: 100%; height: 330px; overflow: hidden; border-radius: 12px; background: var(--surface-soft); }
.empty-map { display: grid; place-items: center; min-height: 250px; color: var(--text-faint); background: var(--surface-soft); }
.empty-map strong { margin-top: 8px; font-size: 12px; }
.accuracy-note { display: flex; align-items: flex-start; gap: 12px; margin: 13px 2px 0; padding: 13px 15px; border: 1px solid color-mix(in srgb, var(--accent) 14%, var(--border)); border-radius: 12px; color: var(--accent-strong); background: color-mix(in srgb, var(--accent-soft) 35%, transparent); }
.accuracy-note > svg { flex: 0 0 auto; margin-top: 2px; }
.accuracy-note p { flex: 1; margin: 0; }
.accuracy-note strong, .accuracy-note span { display: block; }
.accuracy-note strong { font-size: 11px; }
.accuracy-note span { margin-top: 2px; color: var(--text-soft); font-size: 10px; }
.accuracy-note button { align-self: center; flex: 0 0 auto; padding: 6px 9px; border: 1px solid var(--border); border-radius: 8px; color: var(--accent-strong); background: var(--surface); font-size: 10px; font-weight: 700; }
.directory-section { margin-top: 28px; }
.directory-toolbar { margin-bottom: 12px; }
.list-controls { display: flex; gap: 8px; }
.search-field { display: flex; align-items: center; gap: 7px; width: 200px; min-height: 35px; padding: 0 10px; border: 1px solid var(--border); border-radius: 9px; color: var(--text-faint); background: var(--surface); }
.search-field input { width: 100%; border: 0; outline: 0; color: var(--text); background: transparent; font-size: 11px; }
.list-controls select { min-height: 35px; padding: 0 9px; border: 1px solid var(--border); border-radius: 9px; color: var(--text-soft); background: var(--surface); font-size: 10px; }
.directory-table { overflow: hidden; border: 1px solid var(--border); border-radius: 14px; background: var(--surface); box-shadow: var(--shadow-soft); }
.directory-row { display: grid; grid-template-columns: minmax(180px, 1fr) minmax(130px, .45fr) 90px 76px; align-items: center; gap: 12px; width: 100%; min-height: 46px; padding: 5px 13px; border: 0; border-bottom: 1px solid var(--border); color: var(--text); background: transparent; text-align: left; }
.directory-row:not(.table-head):hover { background: var(--surface-soft); }
.directory-row:last-of-type { border-bottom: 0; }
.table-head { min-height: 36px; color: var(--text-faint); background: var(--surface-soft); font-size: 9px; font-weight: 800; letter-spacing: .035em; }
.entry-name { display: flex; align-items: center; gap: 8px; width: 100%; min-width: 0; padding: 0; border: 0; color: var(--text); background: transparent; text-align: left; }
.entry-name-cell { min-width: 0; }
.entry-icon { display: grid; place-items: center; flex: 0 0 28px; width: 28px; height: 28px; border-radius: 9px; color: var(--accent); background: var(--accent-soft); }
.entry-name > span { min-width: 0; }
.entry-name strong, .entry-name small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.entry-name strong { font-size: 11px; }
.entry-name small { margin-top: 3px; color: var(--text-faint); font-size: 9px; }
.entry-name b { margin-left: 6px; padding: 2px 5px; border-radius: 4px; color: var(--accent-strong); background: var(--accent-soft); font-size: 8px; }
.entry-size { display: flex; align-items: baseline; gap: 8px; }
.entry-size strong { font-size: 11px; }
.entry-size small { color: var(--text-faint); font-size: 9px; }
.entry-allocation, .entry-count { color: var(--text-soft); font-size: 10px; }
 .entry-actions { display: flex; justify-content: flex-end; gap: 4px; }
.row-action { display: grid; place-items: center; width: 32px; height: 32px; padding: 0; border: 0; border-radius: 7px; color: var(--text-soft); background: transparent; }
.row-action:hover { color: var(--accent-strong); background: var(--accent-soft); }
.trash-action:hover { color: var(--danger); background: color-mix(in srgb, var(--danger) 10%, transparent); }
.trash-dialog { width: min(480px, calc(100vw - 40px)); padding: 24px; border: 1px solid var(--border); border-radius: 16px; color: var(--text); background: var(--surface); box-shadow: var(--shadow); }
.trash-dialog::backdrop { background: rgba(40, 32, 27, .38); }
.trash-dialog h2 { margin: 0 0 12px; font-size: 18px; overflow-wrap: anywhere; }
.trash-dialog p { font-size: 12px; line-height: 1.65; }
.trash-path { padding: 10px; border-radius: 8px; overflow-wrap: anywhere; color: var(--text-soft); background: var(--surface-soft); }
.trash-size, .demo-trash-note { color: var(--text-soft); }
.trash-dialog-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 22px; }
.danger-button { background: var(--danger); border-color: var(--danger); }
.no-results { padding: 42px; color: var(--text-faint); text-align: center; font-size: 11px; }
.method-note { max-width: 850px; margin: 16px auto 0; color: var(--text-faint); font-size: 9px; line-height: 1.65; text-align: center; }
@media (max-width: 1080px) {
  .space-heading { align-items: flex-start; flex-wrap: wrap; }
  .heading-actions { width: 100%; }
  .threshold-select { margin-right: auto; }
  .summary-strip { grid-template-columns: repeat(2, 1fr); }
  .summary-strip > div:nth-child(2) { border-right: 0; }
  .summary-strip > div:nth-child(-n+2) { border-bottom: 1px solid var(--border); }
  .directory-row { grid-template-columns: minmax(160px, 1fr) 130px 76px; }
  .directory-row > :nth-child(3) { display: none; }
}
</style>
