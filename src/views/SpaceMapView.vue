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
import {
  demoSpaceMapResult,
  formatBytes,
  invokeOrDemo,
  type SpaceMapEntry,
  type SpaceMapResult,
} from "../lib/demoData";

echarts.use([TreemapChart, TooltipComponent, CanvasRenderer]);

type SortKey = "logical" | "allocated" | "name";

const themeStore = useThemeStore();
const { currentTheme } = storeToRefs(themeStore);
const chartElement = ref<HTMLDivElement | null>(null);
const result = ref<SpaceMapResult | null>(null);
const loading = ref(false);
const errorMessage = ref("");
const source = ref<"native" | "demo">("demo");
const query = ref("");
const sortKey = ref<SortKey>("logical");
const scopeRoot = ref("");
let chart: ReturnType<typeof echarts.init> | null = null;
let resizeObserver: ResizeObserver | null = null;
let requestSequence = 0;

const isNativeRuntime = Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

const entries = computed(() => {
  const keyword = query.value.trim().toLocaleLowerCase("zh-CN");
  const items = (result.value?.entries ?? []).filter((entry) =>
    !keyword || entry.name.toLocaleLowerCase("zh-CN").includes(keyword),
  );
  return [...items].sort((left, right) => {
    if (sortKey.value === "name") return left.name.localeCompare(right.name, "zh-CN");
    const field = sortKey.value === "allocated" ? "allocated_size" : "logical_size";
    return right[field] - left[field] || left.name.localeCompare(right.name, "zh-CN");
  });
});

const breadcrumbItems = computed(() => {
  if (!result.value) return [];
  const root = scopeRoot.value || result.value.root_path;
  const current = result.value.root_path;
  const rootParts = root.split("/").filter(Boolean);
  const rootName = root === "/"
    ? "Macintosh HD"
    : rootParts[rootParts.length - 1] || root;
  const items = [{ label: rootName, path: root }];
  if (current === root) return items;
  const relative = current.startsWith(`${root}/`) ? current.slice(root.length + 1) : "";
  let path = root;
  for (const part of relative.split("/").filter(Boolean)) {
    path = `${path === "/" ? "" : path}/${part}`;
    items.push({ label: part, path });
  }
  return items;
});

const allocationDifference = computed(() => {
  if (!result.value) return 0;
  return Math.max(0, result.value.logical_size - result.value.allocated_size);
});

function percentage(entry: SpaceMapEntry) {
  const total = result.value?.logical_size ?? 0;
  if (total <= 0) return "0%";
  const value = entry.logical_size / total * 100;
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)}%`;
}

function formatCount(value: number) {
  return new Intl.NumberFormat("zh-CN").format(value);
}

function formatDate(value?: string) {
  if (!value) return "未知";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "未知";
  return new Intl.DateTimeFormat("zh-CN", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" }).format(date);
}

async function analyze(path?: string, resetScope = false) {
  const sequence = ++requestSequence;
  resizeObserver?.disconnect();
  resizeObserver = null;
  chart?.dispose();
  chart = null;
  loading.value = true;
  errorMessage.value = "";
  const response = await invokeOrDemo<SpaceMapResult>(
    "analyze_space_map",
    demoResultFor(path),
    path ? { path } : undefined,
  );
  if (sequence !== requestSequence) return;
  loading.value = false;

  if (response.source === "error") {
    errorMessage.value = response.error;
    return;
  }

  result.value = response.data;
  source.value = response.source === "native" ? "native" : "demo";
  if (resetScope || !scopeRoot.value) scopeRoot.value = response.data.root_path;
  await nextTick();
  renderChart();
}

function demoResultFor(path?: string): SpaceMapResult {
  if (!path || path === demoSpaceMapResult.root_path) return demoSpaceMapResult;
  const parent = demoSpaceMapResult.entries.find((entry) => entry.path === path);
  if (!parent) return { ...demoSpaceMapResult, root_path: path, display_path: path, entries: [] };
  const ratios = [0.46, 0.28, 0.17, 0.09];
  const names = parent.name === "Projects"
    ? ["build", "node_modules", "Sources", "Archives"]
    : ["Application Support", "Caches", "Containers", "Logs"];
  const childEntries = names.map((name, index) => ({
    name,
    path: `${path}/${name}`,
    logical_size: Math.round(parent.logical_size * ratios[index]),
    allocated_size: Math.round(parent.allocated_size * ratios[index]),
    file_count: Math.round(parent.file_count * ratios[index]),
    directory_count: Math.round(parent.directory_count * ratios[index]),
    modified_at: parent.modified_at,
    is_dir: true,
    is_package: false,
    is_cloud_placeholder: false,
  }));
  return {
    ...demoSpaceMapResult,
    root_path: path,
    display_path: path.replace("/Users/demo", "~"),
    logical_size: parent.logical_size,
    allocated_size: parent.allocated_size,
    file_count: parent.file_count,
    directory_count: parent.directory_count,
    entries: childEntries,
    scan_duration_ms: 940,
  };
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

async function openEntry(entry: SpaceMapEntry) {
  if (entry.is_dir && !entry.is_package) {
    await analyze(entry.path, false);
    return;
  }
  await revealInFinder(entry.path);
}

async function revealInFinder(path: string) {
  if (!isNativeRuntime) return;
  try {
    await invoke("open_in_finder", { path });
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
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
  if (!chartElement.value || !result.value) return;
  const currentElement = chartElement.value;
  chart ??= echarts.init(currentElement, undefined, { renderer: "canvas" });
  const currentChart = chart;
  resizeObserver?.disconnect();
  resizeObserver = new ResizeObserver(() => currentChart.resize());
  resizeObserver.observe(currentElement);
  const styles = getComputedStyle(document.documentElement);
  const surface = styles.getPropertyValue("--surface").trim();
  const text = styles.getPropertyValue("--text").trim();
  const chartEntries = result.value.entries.filter((entry) => entry.logical_size > 0);
  const palettes: Record<string, string[]> = {
    pet: ["#8a4f31", "#ad704d", "#cf9b75", "#e2bea1", "#ecd4be", "#f4e3d4"],
    nature: ["#365d43", "#527a5d", "#78a07d", "#a6c0a2", "#c5d6bf", "#e0e9dc"],
    classic: ["#3c6078", "#587c94", "#7e9db0", "#abc0cd", "#c8d6de", "#e1e8ed"],
  };
  const palette = palettes[currentTheme.value] ?? palettes.pet;
  const option: EChartsOption = {
    animationDuration: 320,
    tooltip: {
      confine: true,
      formatter(params: unknown) {
        const data = (params as { data?: SpaceMapEntry & { value: number } }).data;
        if (!data) return "";
        return `<strong>${escapeHtml(data.name)}</strong><br>逻辑大小 ${formatBytes(data.logical_size)}<br>实际分配 ${formatBytes(data.allocated_size)}<br>${formatCount(data.file_count)} 个文件`;
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
      upperLabel: { show: false },
      itemStyle: { borderColor: surface, borderWidth: 4, gapWidth: 4, borderRadius: 9 },
      emphasis: { itemStyle: { shadowBlur: 14, shadowColor: "rgba(0,0,0,.14)" } },
      data: chartEntries.map((entry, index) => ({
        ...entry,
        value: entry.logical_size,
        itemStyle: { color: palette[index % palette.length] },
        label: {
          color: index < 2 ? "#fff" : text,
          formatter: `${entry.name}\n${formatBytes(entry.logical_size)}`,
        },
      })),
    }],
  };
  currentChart.setOption(option, true);
  currentChart.off("click");
  currentChart.on("click", (params) => {
    const entry = params.data as SpaceMapEntry | undefined;
    if (entry) void openEntry(entry);
  });
}

function escapeHtml(value: string) {
  return value.replace(/[&<>'"]/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;",
  })[character] ?? character);
}

watch(currentTheme, () => nextTick(renderChart));
watch(entries, () => nextTick(renderChart));

onMounted(() => {
  if (chartElement.value) {
    resizeObserver = new ResizeObserver(() => chart?.resize());
    resizeObserver.observe(chartElement.value);
  }
  void analyze(undefined, true);
});

onBeforeUnmount(() => {
  requestSequence += 1;
  if (isNativeRuntime) void invoke("cancel_space_map");
  resizeObserver?.disconnect();
  chart?.dispose();
  chart = null;
});
</script>

<template>
  <section class="space-page" aria-labelledby="space-map-title">
    <header class="space-heading">
      <div>
        <p class="eyebrow">只读分析 · 不会移动或删除文件</p>
        <h1 id="space-map-title">看清空间都去了哪里</h1>
        <p>矩形面积代表目录的逻辑大小。点击文件夹逐层进入，先找到占用，再决定如何处理。</p>
      </div>
      <div class="heading-actions">
        <button type="button" class="secondary-button" :disabled="loading" @click="analyze(result?.root_path, false)">
          <AppIcon name="refresh" :size="16" /> 重新分析
        </button>
        <button type="button" class="primary-button" :disabled="loading" @click="chooseDirectory">
          <AppIcon name="folder" :size="17" /> 扫描范围
        </button>
      </div>
    </header>

    <p v-if="errorMessage" class="error-banner" role="alert">{{ errorMessage }}</p>

    <div class="pathbar" aria-label="当前分析路径">
      <AppIcon name="disk" :size="16" />
      <template v-for="(item, index) in breadcrumbItems" :key="item.path">
        <span v-if="index" aria-hidden="true">/</span>
        <button type="button" :disabled="loading || index === breadcrumbItems.length - 1" @click="analyze(item.path, false)">{{ item.label }}</button>
      </template>
      <span :class="['source-chip', { native: source === 'native' }]">{{ source === "native" ? "本机实时数据" : "界面预览数据" }}</span>
    </div>

    <div v-if="loading" class="analysis-state" aria-live="polite">
      <span class="analysis-spinner" aria-hidden="true"></span>
      <div><strong>正在读取目录结构</strong><small>大目录需要一些时间，分析只读取文件元数据。</small></div>
    </div>

    <template v-else-if="result">
      <section class="summary-strip" aria-label="空间分析摘要">
        <div><small>逻辑大小</small><strong>{{ formatBytes(result.logical_size) }}</strong><span>用于矩形面积</span></div>
        <div><small>实际分配</small><strong>{{ formatBytes(result.allocated_size) }}</strong><span>文件系统已分配块</span></div>
        <div><small>文件与目录</small><strong>{{ formatCount(result.file_count + result.directory_count) }}</strong><span>{{ formatCount(result.file_count) }} 个文件</span></div>
        <div><small>完成时间</small><strong>{{ (result.scan_duration_ms / 1000).toFixed(1) }} 秒</strong><span>{{ result.entries.length }} 个直接子项</span></div>
      </section>

      <section class="map-panel">
        <div class="panel-title">
          <div><h2>目录占用图</h2><p>逻辑大小越大，矩形面积越大</p></div>
          <span>{{ result.display_path }}</span>
        </div>
        <div v-if="result.logical_size > 0" ref="chartElement" class="treemap-chart" role="img" :aria-label="`${result.display_path} 目录占用矩形图`"></div>
        <div v-else class="empty-map"><AppIcon name="folder" :size="30" /><strong>这个文件夹里没有可统计的普通文件</strong></div>
      </section>

      <div v-if="result.skipped_items || result.hard_link_duplicates || result.symlink_count || allocationDifference" class="accuracy-note">
        <AppIcon name="shield" :size="18" />
        <p>
          <strong>统计边界清楚可见</strong>
          <span v-if="allocationDifference">逻辑大小比已分配空间多 {{ formatBytes(allocationDifference) }}；稀疏文件、压缩或云占位可能造成差异。</span>
          <span>已避免重复计算 {{ formatCount(result.hard_link_duplicates) }} 个硬链接，未跟随 {{ formatCount(result.symlink_count) }} 个符号链接。</span>
          <span v-if="result.skipped_items">有 {{ formatCount(result.skipped_items) }} 个条目无法读取，当前结果不包含它们。</span>
        </p>
        <button v-if="result.skipped_items" type="button" @click="requestFullDiskAccess">检查磁盘权限</button>
      </div>

      <section class="directory-section">
        <header class="directory-toolbar">
          <div><h2>目录明细</h2><p>列表与上方矩形图使用同一份实时结果</p></div>
          <div class="list-controls">
            <label class="search-field"><AppIcon name="search" :size="15" /><input v-model="query" type="search" placeholder="搜索当前目录"></label>
            <select v-model="sortKey" aria-label="目录排序方式">
              <option value="logical">按逻辑大小</option>
              <option value="allocated">按实际分配</option>
              <option value="name">按名称</option>
            </select>
          </div>
        </header>

        <div class="directory-table" role="table" aria-label="目录占用明细">
          <div class="directory-row table-head" role="row">
            <span role="columnheader">文件夹或文件</span><span role="columnheader">逻辑大小</span><span role="columnheader">实际分配</span><span role="columnheader">内容</span><span role="columnheader"></span>
          </div>
          <button v-for="entry in entries" :key="entry.path" type="button" class="directory-row" role="row" @click="openEntry(entry)">
            <span class="entry-name" role="cell">
              <i><AppIcon :name="entry.is_dir ? 'folder' : 'file'" :size="19" /></i>
              <span><strong>{{ entry.name }}</strong><small>{{ formatDate(entry.modified_at) }}<b v-if="entry.is_package">包目录</b><b v-if="entry.is_cloud_placeholder">云占位</b></small></span>
            </span>
            <span class="entry-size" role="cell"><strong>{{ formatBytes(entry.logical_size) }}</strong><small>{{ percentage(entry) }}</small></span>
            <span class="entry-allocation" role="cell">{{ formatBytes(entry.allocated_size) }}</span>
            <span class="entry-count" role="cell">{{ formatCount(entry.file_count) }} 个文件</span>
            <span class="entry-action" role="cell"><AppIcon :name="entry.is_dir && !entry.is_package ? 'chevron' : 'eye'" :size="17" /></span>
          </button>
          <div v-if="!entries.length" class="no-results">没有符合搜索条件的项目</div>
        </div>
      </section>

      <p class="method-note">逻辑大小用于比较目录占用；实际分配来自文件系统块数。APFS 克隆、压缩和共享块无法被普通目录遍历精确归属，因此这些数字不代表删除后必然增加的可用空间。</p>
    </template>
  </section>
</template>

<style scoped>
.space-page { max-width: 1240px; margin: 0 auto; padding-top: 34px; }
.space-heading { display: flex; align-items: flex-end; justify-content: space-between; gap: 30px; margin-bottom: 24px; }
.eyebrow { margin: 0 0 8px; color: var(--accent); font-size: 10px; font-weight: 800; letter-spacing: .09em; }
.space-heading h1 { margin: 0; color: var(--text); font-size: clamp(27px, 3vw, 38px); font-weight: 650; letter-spacing: -.045em; }
.space-heading p:not(.eyebrow) { max-width: 640px; margin: 9px 0 0; color: var(--text-soft); font-size: 12px; }
.heading-actions { display: flex; gap: 9px; }
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
.analysis-state { display: flex; align-items: center; justify-content: center; gap: 18px; min-height: 470px; border: 1px solid var(--border); border-radius: 18px; background: var(--surface); }
.analysis-state strong, .analysis-state small { display: block; }
.analysis-state strong { font-size: 15px; }
.analysis-state small { margin-top: 4px; color: var(--text-faint); font-size: 11px; }
.analysis-spinner { width: 42px; height: 42px; border: 4px solid var(--accent-soft); border-top-color: var(--accent); border-radius: 50%; animation: spin .9s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.summary-strip { display: grid; grid-template-columns: repeat(4, 1fr); margin-bottom: 14px; overflow: hidden; border: 1px solid var(--border); border-radius: 15px; background: var(--surface); box-shadow: var(--shadow-soft); }
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
.directory-row { display: grid; grid-template-columns: minmax(220px, 1.7fr) minmax(110px, .65fr) minmax(100px, .65fr) minmax(105px, .65fr) 32px; align-items: center; gap: 12px; width: 100%; min-height: 62px; padding: 9px 13px; border: 0; border-bottom: 1px solid var(--border); color: var(--text); background: transparent; text-align: left; }
button.directory-row:hover { background: var(--surface-soft); }
.directory-row:last-of-type { border-bottom: 0; }
.table-head { min-height: 36px; color: var(--text-faint); background: var(--surface-soft); font-size: 9px; font-weight: 800; letter-spacing: .035em; }
.entry-name { display: flex; align-items: center; gap: 10px; min-width: 0; }
.entry-name > i { display: grid; place-items: center; flex: 0 0 34px; width: 34px; height: 34px; border-radius: 9px; color: var(--accent); background: var(--accent-soft); }
.entry-name > span { min-width: 0; }
.entry-name strong, .entry-name small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.entry-name strong { font-size: 11px; }
.entry-name small { margin-top: 3px; color: var(--text-faint); font-size: 9px; }
.entry-name b { margin-left: 6px; padding: 2px 5px; border-radius: 4px; color: var(--accent-strong); background: var(--accent-soft); font-size: 8px; }
.entry-size strong, .entry-size small { display: block; }
.entry-size strong { font-size: 11px; }
.entry-size small { margin-top: 2px; color: var(--text-faint); font-size: 9px; }
.entry-allocation, .entry-count { color: var(--text-soft); font-size: 10px; }
.entry-action { display: grid; place-items: center; color: var(--text-faint); }
.no-results { padding: 42px; color: var(--text-faint); text-align: center; font-size: 11px; }
.method-note { max-width: 850px; margin: 16px auto 0; color: var(--text-faint); font-size: 9px; line-height: 1.65; text-align: center; }
@media (max-width: 1080px) {
  .space-heading { align-items: flex-start; }
  .summary-strip { grid-template-columns: repeat(2, 1fr); }
  .summary-strip > div:nth-child(2) { border-right: 0; }
  .summary-strip > div:nth-child(-n+2) { border-bottom: 1px solid var(--border); }
  .directory-row { grid-template-columns: minmax(200px, 1.5fr) 100px 100px 32px; }
  .directory-row > :nth-child(4) { display: none; }
}
</style>
