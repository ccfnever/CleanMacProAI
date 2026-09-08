<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  formatBytes,
  invokeOrDemo,
  type CleanReport,
  type FileInfo,
  type InstalledApp,
} from "../lib/demoData";

type FacetKind = "status" | "source" | "platform" | "vendor";
type FileGroupId =
  | "binary"
  | "support"
  | "cache"
  | "preferences"
  | "logs"
  | "login"
  | "container"
  | "userData";

interface FacetItem {
  id: string;
  label: string;
  kind: FacetKind;
  count: number;
  muted?: boolean;
}

interface RelatedGroup {
  id: FileGroupId;
  label: string;
  files: FileInfo[];
  size: number;
}

const apps = ref<InstalledApp[]>([]);
const isLoading = ref(true);
const inspectingBundleId = ref<string | null>(null);
const isUninstalling = ref(false);
const isConfirmingUninstall = ref(false);
const query = ref("");
const activeFacet = ref("status:all");
const selectedBundleIds = ref<Set<string>>(new Set());
const failedIconPaths = ref<Set<string>>(new Set());
const expandedBundleIds = ref<Set<string>>(new Set());
const isAutoScanning = ref(false);
const autoScanCompletedCount = ref(0);
const notice = ref<string | null>(null);
const uninstallReport = ref<CleanReport | null>(null);
const dataSource = ref<"native" | "unavailable">("unavailable");
let isUnmounted = false;

const filteredApps = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  return apps.value
    .filter((app) => matchesFacet(app, activeFacet.value))
    .filter((app) => {
      if (!keyword) return true;
      return (
        app.name.toLowerCase().includes(keyword) ||
        app.bundle_id.toLowerCase().includes(keyword) ||
        app.app_path.toLowerCase().includes(keyword)
      );
    })
    .sort((a, b) => a.name.localeCompare(b.name, "zh-Hans-CN"));
});

const selectedApps = computed(() =>
  apps.value.filter((app) => selectedBundleIds.value.has(app.bundle_id)),
);

const selectedTotal = computed(() =>
  selectedApps.value.reduce((sum, app) => sum + appTotalSize(app), 0),
);

const filteredTotalSize = computed(() =>
  filteredApps.value.reduce((sum, app) => sum + appTotalSize(app), 0),
);

const listSizeLabel = computed(() => {
  if (filteredTotalSize.value <= 0) return "";
  return ` · ${formatBytes(filteredTotalSize.value)}`;
});

const scanProgressLabel = computed(() => {
  if (!isAutoScanning.value) return "";
  return ` · 正在统计 ${autoScanCompletedCount.value}/${apps.value.length}`;
});

const facetSections = computed(() => {
  const appStoreCount = apps.value.filter((app) => appSource(app) === "app_store").length;
  const macCount = apps.value.filter((app) => appPlatform(app) === "mac").length;
  const ipadCount = apps.value.filter((app) => appPlatform(app) === "ipad").length;
  const vendors = ["Apple", "Microsoft", "Google", "Adobe", "MacPaw", "其他"].map((vendor) => ({
    id: `vendor:${vendor}`,
    label: vendor,
    kind: "vendor" as const,
    count: apps.value.filter((app) => appVendor(app) === vendor).length,
  }));

  return [
    {
      title: "",
      items: [
        { id: "status:all", label: "所有应用程序", kind: "status" as const, count: apps.value.length },
        { id: "status:unused", label: "未使用", kind: "status" as const, count: unusedApps.value.length },
        { id: "status:leftovers", label: "残留项", kind: "status" as const, count: leftoverApps.value.length },
        { id: "status:suspicious", label: "可疑项", kind: "status" as const, count: suspiciousApps.value.length },
        { id: "status:selected", label: "已选中", kind: "status" as const, count: selectedBundleIds.value.size },
      ],
    },
    {
      title: "商店",
      items: [
        { id: "source:app_store", label: "App Store", kind: "source" as const, count: appStoreCount },
        { id: "source:other", label: "其他", kind: "source" as const, count: apps.value.length - appStoreCount },
      ],
    },
    {
      title: "App Store",
      items: [
        { id: "platform:mac", label: "Mac 应用", kind: "platform" as const, count: macCount },
        { id: "platform:ipad", label: "iPhone 和 iPad 应用", kind: "platform" as const, count: ipadCount },
      ],
    },
    { title: "供应商", items: vendors },
  ];
});

const unusedApps = computed(() =>
  apps.value.filter((app) => app.related_count < 1000 && appTotalSize(app) > 250 * 1024 ** 2),
);
const leftoverApps = computed(() => apps.value.filter((app) => app.related_size > 0));
const suspiciousApps = computed(() =>
  apps.value.filter((app) => app.bundle_id.includes("helper") || app.bundle_id.includes("virtual")),
);

const isInspecting = computed(() => (bundleId: string) => inspectingBundleId.value === bundleId);

function appTotalSize(app: InstalledApp): number {
  return app.app_size + app.related_size;
}

function appSizeLabel(app: InstalledApp): string {
  if (isInspecting.value(app.bundle_id)) return "统计中...";
  const size = appTotalSize(app);
  return size > 0 ? formatBytes(size) : "待统计";
}

function appInitial(app: InstalledApp): string {
  return app.name.trim().charAt(0).toUpperCase();
}

function appIconSrc(app: InstalledApp): string | null {
  const source = app.icon_data_url || app.icon_path;
  if (!source || failedIconPaths.value.has(source)) return null;
  return source;
}

function markIconFailed(app: InstalledApp) {
  const source = app.icon_data_url || app.icon_path;
  if (!source) return;
  failedIconPaths.value = new Set([...failedIconPaths.value, source]);
}

function iconTone(app: InstalledApp): string {
  const tones = ["#6dd5fa", "#5a8dee", "#7f8cff", "#18c3a3", "#ffb457", "#ff6f91"];
  let hash = 0;
  for (const char of app.bundle_id) hash += char.charCodeAt(0);
  return tones[hash % tones.length];
}

function appVendor(app: InstalledApp): string {
  const id = app.bundle_id.toLowerCase();
  if (id.includes("apple")) return "Apple";
  if (id.includes("microsoft")) return "Microsoft";
  if (id.includes("google")) return "Google";
  if (id.includes("adobe")) return "Adobe";
  if (id.includes("macpaw")) return "MacPaw";
  return "其他";
}

function appSource(app: InstalledApp): "app_store" | "other" {
  return app.bundle_id.startsWith("com.apple.") ? "app_store" : "other";
}

function appPlatform(app: InstalledApp): "mac" | "ipad" {
  return app.name.toLowerCase().includes("ipad") ? "ipad" : "mac";
}

function matchesFacet(app: InstalledApp, facet: string): boolean {
  const [kind, value] = facet.split(":");
  if (kind === "status") {
    if (value === "all") return true;
    if (value === "unused") return unusedApps.value.some((item) => item.bundle_id === app.bundle_id);
    if (value === "leftovers") return app.related_size > 0;
    if (value === "suspicious") {
      return suspiciousApps.value.some((item) => item.bundle_id === app.bundle_id);
    }
    if (value === "selected") return selectedBundleIds.value.has(app.bundle_id);
  }
  if (kind === "source") return appSource(app) === value;
  if (kind === "platform") return appPlatform(app) === value;
  if (kind === "vendor") return appVendor(app) === value;
  return true;
}

function classifyFile(file: FileInfo): FileGroupId {
  const path = file.path.toLowerCase();
  if (path.includes("caches")) return "cache";
  if (path.includes("preferences") || path.endsWith(".plist")) return "preferences";
  if (path.includes("logs")) return "logs";
  if (path.includes("launchagents") || path.includes("loginitems")) return "login";
  if (path.includes("containers")) return "container";
  if (path.includes("application support")) return "support";
  return "userData";
}

function fileGroups(app: InstalledApp): RelatedGroup[] {
  const groupMap = new Map<FileGroupId, RelatedGroup>([
    [
      "binary",
      {
        id: "binary",
        label: "二进制文件",
        files: [{ path: app.app_path, size: app.app_size }],
        size: app.app_size,
      },
    ],
    ["support", { id: "support", label: "支持文件", files: [], size: 0 }],
    ["cache", { id: "cache", label: "缓存", files: [], size: 0 }],
    ["preferences", { id: "preferences", label: "偏好设置", files: [], size: 0 }],
    ["logs", { id: "logs", label: "日志", files: [], size: 0 }],
    ["login", { id: "login", label: "登录项", files: [], size: 0 }],
    ["container", { id: "container", label: "容器", files: [], size: 0 }],
    ["userData", { id: "userData", label: "用户数据", files: [], size: 0 }],
  ]);

  for (const file of app.related_files) {
    const group = groupMap.get(classifyFile(file));
    if (!group) continue;
    group.files.push(file);
    group.size += file.size;
  }

  return Array.from(groupMap.values()).filter((group) => group.files.length > 0 || group.id === "binary");
}

function groupIcon(groupId: FileGroupId): string {
  const icons: Record<FileGroupId, string> = {
    binary: "▣",
    support: "▰",
    cache: "▤",
    preferences: "◷",
    logs: "≡",
    login: "⏻",
    container: "□",
    userData: "▥",
  };
  return icons[groupId];
}

onMounted(async () => {
  document.addEventListener("keydown", handleDialogKeydown);
  const result = await invokeOrDemo<InstalledApp[]>("list_installed_apps", [], undefined, 5000);
  if (result.source === "error") {
    notice.value = `无法读取应用列表：${result.error}`;
    isLoading.value = false;
    return;
  }
  if (result.source === "demo") {
    apps.value = [];
    notice.value = "请从 CleanMacProAI 的 macOS 桌面应用打开卸载器；浏览器预览无法卸载应用。";
    isLoading.value = false;
    return;
  }
  apps.value = result.data;
  dataSource.value = "native";
  selectedBundleIds.value = new Set();
  expandedBundleIds.value = new Set();
  notice.value = result.source === "empty" ? "没有找到可卸载的应用。" : null;
  isLoading.value = false;
  void scanAppsInDisplayOrder(result.data);
});

onUnmounted(() => {
  isUnmounted = true;
  document.removeEventListener("keydown", handleDialogKeydown);
});

async function scanAppsInDisplayOrder(appList: InstalledApp[]) {
  isAutoScanning.value = true;
  autoScanCompletedCount.value = 0;
  const queue = [...appList].sort((a, b) => a.name.localeCompare(b.name, "zh-Hans-CN"));

  try {
    for (const app of queue) {
      if (isUnmounted) return;
      await inspectApp(app.bundle_id);
      if (isUnmounted) return;
      autoScanCompletedCount.value += 1;
    }
  } finally {
    isAutoScanning.value = false;
  }
}

async function inspectApp(bundleId: string) {
  if (!bundleId) return null;

  const current = apps.value.find((app) => app.bundle_id === bundleId);
  if (!current || current.related_files.length > 0 || current.app_size > 0) return current ?? null;

  inspectingBundleId.value = bundleId;
  const result = await invokeOrDemo<InstalledApp>(
    "inspect_installed_app",
    current,
    {
      bundleId,
      appPath: current.app_path,
    },
    60000,
  );

  if (result.source === "error") {
    notice.value = `无法读取 ${current.name} 的详情：${result.error}`;
    if (inspectingBundleId.value === bundleId) inspectingBundleId.value = null;
    return null;
  }
  apps.value = apps.value.map((app) => (app.bundle_id === bundleId ? result.data : app));
  if (result.source === "demo") {
    notice.value = "当前环境无法读取该应用详情，已保留快速列表数据。";
  }
  if (inspectingBundleId.value === bundleId) {
    inspectingBundleId.value = null;
  }
  return result.data;
}

function setFacet(item: FacetItem) {
  activeFacet.value = item.id;
}

function toggleExpanded(app: InstalledApp) {
  const next = new Set(expandedBundleIds.value);
  if (next.has(app.bundle_id)) {
    next.delete(app.bundle_id);
  } else {
    next.add(app.bundle_id);
    void inspectApp(app.bundle_id);
  }
  expandedBundleIds.value = next;
}

function toggleSelected(bundleId: string) {
  const next = new Set(selectedBundleIds.value);
  if (next.has(bundleId)) {
    next.delete(bundleId);
  } else {
    next.add(bundleId);
  }
  selectedBundleIds.value = next;
  uninstallReport.value = null;
}

function selectAllVisible() {
  const next = new Set(selectedBundleIds.value);
  for (const app of filteredApps.value) next.add(app.bundle_id);
  selectedBundleIds.value = next;
}

async function uninstallSelected() {
  if (selectedApps.value.length === 0) return;
  isConfirmingUninstall.value = false;
  isUninstalling.value = true;
  uninstallReport.value = null;
  const targets = [...selectedApps.value];
  const removedIds = new Set<string>();
  const failedIds = new Set<string>();
  const failedDetails: string[] = [];
  const aggregate: CleanReport = { cleaned_count: 0, freed_bytes: 0, skipped_count: 0, errors: [] };

  try {
    for (const app of targets) {
      const result = await invokeOrDemo<CleanReport>("uninstall_app", {
        cleaned_count: 0,
        freed_bytes: 0,
        skipped_count: 0,
        errors: [],
      }, {
        bundleId: app.bundle_id,
        appPath: app.app_path,
      });
      if (result.source === "error") {
        failedIds.add(app.bundle_id);
        failedDetails.push(`${app.name}（${result.error}）`);
        continue;
      }
      if (result.source !== "native") {
        failedIds.add(app.bundle_id);
        failedDetails.push(`${app.name}（演示模式未执行）`);
        continue;
      }
      const appRemovalError = result.data.errors.find(
        (error) => normalizePath(error.path) === normalizePath(app.app_path),
      );
      if (appRemovalError) {
        failedIds.add(app.bundle_id);
        failedDetails.push(`${app.name}（${appRemovalError.reason}）`);
        continue;
      }
      removedIds.add(app.bundle_id);
      aggregate.cleaned_count += result.data.cleaned_count;
      aggregate.freed_bytes += result.data.freed_bytes;
      aggregate.skipped_count += result.data.skipped_count;
      aggregate.errors.push(...result.data.errors);
    }

    if (removedIds.size > 0) {
      uninstallReport.value = aggregate;
      apps.value = apps.value.filter((item) => !removedIds.has(item.bundle_id));
    }
    selectedBundleIds.value = failedIds;
    expandedBundleIds.value = new Set(
      [...expandedBundleIds.value].filter((bundleId) => !removedIds.has(bundleId)),
    );

    if (removedIds.size === targets.length && aggregate.errors.length === 0) {
      notice.value = `已将 ${removedIds.size} 个应用及关联残留移入废纸篓。`;
    } else if (removedIds.size === targets.length) {
      notice.value = `已将 ${removedIds.size} 个应用移入废纸篓；${aggregate.errors.length} 个关联残留未能处理。`;
    } else if (removedIds.size > 0) {
      notice.value = `已卸载 ${removedIds.size} 个应用；${failedDetails.length} 个失败并已保留：${failedDetails.join("、")}。`;
    } else {
      notice.value = `卸载失败，所选应用均已保留：${failedDetails.join("、")}。`;
    }
  } finally {
    isUninstalling.value = false;
  }
}

function requestUninstall() {
  if (selectedApps.value.length === 0 || isUninstalling.value) return;
  isConfirmingUninstall.value = true;
}

function handleDialogKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && !isUninstalling.value) {
    isConfirmingUninstall.value = false;
  }
}

function normalizePath(path: string): string {
  return path.replace(/\/$/, "");
}
</script>

<template>
  <section class="uninstaller-page">
    <header class="uninstaller-top">
      <button type="button" class="back-button" aria-label="返回">‹</button>
      <span>简介</span>
      <strong>卸载器</strong>
      <label class="search-box">
        <span>⌕</span>
        <input v-model="query" type="search" placeholder="搜索" />
      </label>
      <button type="button" class="assistant-pill">
        <span></span>
        助手
      </button>
    </header>

    <div class="uninstaller-body">
      <aside class="facet-panel" aria-label="应用归类">
        <div
          v-for="section in facetSections"
          :key="section.title || 'status'"
          class="facet-section"
        >
          <p v-if="section.title">{{ section.title }}</p>
          <button
            v-for="item in section.items"
            :key="item.id"
            type="button"
            :class="['facet-item', { active: activeFacet === item.id, muted: item.count === 0 }]"
            @click="setFacet(item)"
          >
            <span>{{ item.label }}</span>
            <strong>{{ item.count }}</strong>
          </button>
        </div>
      </aside>

      <main class="apps-panel">
        <div class="apps-head">
          <div>
            <h1>所有应用程序</h1>
            <p>您 Mac 上安装的所有应用程序均显示在下方。</p>
          </div>
          <button type="button" class="select-visible" @click="selectAllVisible">
            选择当前列表
          </button>
        </div>

        <p v-if="notice" class="notice">{{ notice }}</p>

        <div class="list-toolbar">
          <span class="list-count">
            {{ filteredApps.length }} 个应用程序{{ listSizeLabel }}{{ scanProgressLabel }}
            <span v-if="isAutoScanning" class="scan-spinner" aria-label="正在统计"></span>
          </span>
          <button type="button">排序方式按 名称⌄</button>
        </div>

        <div v-if="isLoading" class="loading-state">正在读取应用列表...</div>

        <div v-else class="app-tree">
          <article
            v-for="app in filteredApps"
            :key="app.bundle_id"
            :class="['app-node', { expanded: expandedBundleIds.has(app.bundle_id) }]"
          >
            <div class="app-main">
              <button
                type="button"
                :class="['selection-dot', { checked: selectedBundleIds.has(app.bundle_id) }]"
                :aria-label="`选择 ${app.name}`"
                @click="toggleSelected(app.bundle_id)"
              >
                <span>✓</span>
              </button>
              <button type="button" class="expand-hit" @click="toggleExpanded(app)">
                <span class="app-icon-wrap">
                  <img
                    v-if="appIconSrc(app)"
                    class="app-logo"
                    :src="appIconSrc(app) || undefined"
                    :alt="`${app.name} 图标`"
                    @error="markIconFailed(app)"
                  />
                  <span
                    v-else
                    class="app-icon"
                    :style="{ background: `linear-gradient(135deg, ${iconTone(app)}, #294f93)` }"
                  >
                    {{ appInitial(app) }}
                  </span>
                </span>
                <strong>{{ app.name }}</strong>
              </button>
              <span class="chevron">{{ expandedBundleIds.has(app.bundle_id) ? "⌄" : "›" }}</span>
              <strong class="row-size">{{ appSizeLabel(app) }}</strong>
            </div>

            <div v-if="expandedBundleIds.has(app.bundle_id)" class="file-tree">
              <section v-for="group in fileGroups(app)" :key="group.id" class="file-group">
                <div class="group-row">
                  <span class="group-icon">{{ groupIcon(group.id) }}</span>
                  <strong>{{ group.label }}</strong>
                  <span class="group-size">{{ formatBytes(group.size) }}</span>
                </div>
                <div v-for="file in group.files" :key="file.path" class="file-row">
                  <span class="file-icon">▰</span>
                  <span>{{ file.path.split('/').pop() || file.path }}</span>
                  <strong>{{ formatBytes(file.size) }}</strong>
                </div>
              </section>
            </div>
          </article>

          <div v-if="filteredApps.length === 0" class="empty-row">
            没有匹配的应用
          </div>
        </div>
      </main>
    </div>

    <footer class="bottom-bar">
      <span>{{ selectedBundleIds.size }} 个应用程序</span>
      <button
        type="button"
        class="uninstall-orb"
        :title="dataSource !== 'native' ? '请在 macOS 桌面应用中执行卸载' : undefined"
        :disabled="selectedBundleIds.size === 0 || isUninstalling || dataSource !== 'native'"
        @click="requestUninstall"
      >
        {{ isUninstalling ? "卸载中" : "卸载" }}
      </button>
      <span>{{ selectedTotal > 0 ? formatBytes(selectedTotal) : "0 GB" }}</span>
    </footer>

    <div v-if="uninstallReport" class="report-panel">
      <span>✓</span>
      <p>
        释放 {{ formatBytes(uninstallReport.freed_bytes) }}，处理
        {{ uninstallReport.cleaned_count.toLocaleString() }} 个文件。项目已移入废纸篓，可在 macOS 废纸篓中自行恢复。
      </p>
    </div>

    <Teleport to="body">
      <div
        v-if="isConfirmingUninstall"
        class="confirm-backdrop"
        role="presentation"
        @click.self="isConfirmingUninstall = false"
      >
        <section class="confirm-dialog" role="dialog" aria-modal="true" aria-labelledby="uninstall-title">
          <div class="confirm-symbol" aria-hidden="true">!</div>
          <div class="confirm-copy">
            <p>应用卸载</p>
            <h2 id="uninstall-title">将 {{ selectedApps.length }} 个应用移入废纸篓？</h2>
            <span>
              应用程序及扫描到的关联文件将一并处理，预计释放 {{ formatBytes(selectedTotal) }}。
            </span>
          </div>
          <div class="confirm-actions">
            <button type="button" class="confirm-cancel" @click="isConfirmingUninstall = false">
              取消
            </button>
            <button type="button" class="confirm-submit" @click="uninstallSelected">
              移入废纸篓
            </button>
          </div>
        </section>
      </div>
    </Teleport>
  </section>
</template>

<style scoped>
.uninstaller-page {
  position: relative;
  min-height: 100vh;
  margin: 0;
  padding: 0 0 64px;
  overflow: hidden;
  border-left: 1px solid rgba(207, 228, 247, 0.18);
  border-radius: 0;
  background:
    linear-gradient(180deg, rgba(92, 166, 207, 0.94), rgba(65, 70, 132, 0.98)),
    #4d7ca9;
  color: #eaf6ff;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.22), 0 30px 80px rgba(36, 61, 92, 0.22);
}

.uninstaller-top {
  display: grid;
  grid-template-columns: 32px 1fr auto 264px 104px;
  align-items: center;
  gap: 14px;
  height: 48px;
  padding: 7px 18px 0 24px;
  color: rgba(233, 247, 255, 0.76);
  font-size: 13px;
  font-weight: 800;
}

.uninstaller-top strong {
  justify-self: center;
  margin-right: 130px;
  color: rgba(238, 249, 255, 0.82);
  font-size: 14px;
}

.back-button {
  width: 28px;
  height: 28px;
  border: 0;
  background: transparent;
  color: #d8edf8;
  font-size: 32px;
  line-height: 1;
}

.search-box {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 34px;
  padding: 0 16px;
  border-radius: 10px;
  background: rgba(39, 80, 112, 0.54);
  color: rgba(239, 250, 255, 0.86);
}

.search-box input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: #fff;
  font-weight: 800;
}

.search-box input::placeholder {
  color: rgba(235, 247, 255, 0.62);
}

.assistant-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  height: 32px;
  border: 0;
  border-radius: 999px;
  background: rgba(35, 79, 111, 0.68);
  color: #ddf3ff;
  font-weight: 900;
}

.assistant-pill span {
  width: 18px;
  height: 8px;
  border-radius: 999px;
  background: radial-gradient(circle at 4px 4px, #91e5ff 0 3px, transparent 4px),
    radial-gradient(circle at 14px 4px, #91e5ff 0 3px, transparent 4px);
}

.uninstaller-body {
  display: grid;
  grid-template-columns: minmax(210px, 282px) minmax(500px, 1fr);
  gap: clamp(20px, 3.2vw, 54px);
  padding: 6px 34px 0 26px;
}

.facet-panel {
  color: rgba(233, 247, 255, 0.72);
}

.facet-section {
  display: grid;
  gap: 2px;
  margin-bottom: 13px;
}

.facet-section p {
  margin: 0 0 4px 12px;
  color: rgba(211, 232, 246, 0.46);
  font-size: 12px;
  font-weight: 900;
}

.facet-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 29px;
  padding: 0 10px;
  border: 0;
  border-radius: 18px;
  background: transparent;
  color: inherit;
  text-align: left;
  font-size: 13px;
  font-weight: 850;
}

.facet-item.active {
  background: rgba(54, 101, 139, 0.62);
  color: #fff;
}

.facet-item.muted {
  opacity: 0.55;
}

.facet-item strong {
  font-size: 12px;
}

.apps-panel {
  min-width: 520px;
}

.apps-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  margin-top: 10px;
}

.apps-head h1 {
  margin: 0;
  color: #fff;
  font-size: 28px;
  line-height: 1.1;
  letter-spacing: 0;
}

.apps-head p {
  margin: 8px 0 0;
  color: rgba(235, 248, 255, 0.74);
  font-size: 13px;
  font-weight: 750;
}

.select-visible,
.list-toolbar button {
  min-height: 30px;
  border: 0;
  border-radius: 999px;
  background: rgba(39, 80, 112, 0.35);
  color: rgba(239, 250, 255, 0.78);
  font-size: 12px;
  font-weight: 850;
}

.select-visible {
  padding: 0 14px;
}

.notice {
  margin: 10px 0 0;
  padding: 8px 10px;
  border-radius: 12px;
  background: rgba(255, 215, 92, 0.18);
  color: #fff4bf;
  font-size: 12px;
  font-weight: 800;
}

.list-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 24px 0 10px;
  color: rgba(235, 248, 255, 0.64);
  font-size: 12px;
  font-weight: 850;
}

.list-count {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.scan-spinner {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background:
    conic-gradient(from 0deg, #53d8d1, #9ae6ff, rgba(154, 230, 255, 0.12), #53d8d1);
  mask: radial-gradient(circle, transparent 48%, #000 52%);
  animation: scan-spin 780ms linear infinite;
}

.list-toolbar button {
  padding: 0 2px;
  background: transparent;
}

.loading-state,
.empty-row {
  padding: 50px;
  color: rgba(235, 248, 255, 0.68);
  text-align: center;
  font-weight: 850;
}

.app-tree {
  max-height: calc(100vh - 190px);
  padding-right: 14px;
  overflow: auto;
}

.app-node {
  border-radius: 14px;
}

.app-main {
  display: grid;
  grid-template-columns: 26px 1fr 22px 92px;
  align-items: center;
  gap: 10px;
  min-height: 46px;
  color: rgba(245, 251, 255, 0.93);
  font-weight: 900;
}

.expand-hit {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  font-weight: 900;
}

.expand-hit strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-icon-wrap,
.app-icon,
.app-logo {
  flex: 0 0 auto;
  width: 30px;
  height: 30px;
}

.app-icon-wrap {
  display: grid;
  place-items: center;
}

.app-icon,
.app-logo {
  display: grid;
  place-items: center;
  border-radius: 8px;
}

.app-icon {
  color: #fff;
  font-weight: 950;
  box-shadow: 0 10px 22px rgba(22, 44, 74, 0.18);
}

.app-logo {
  object-fit: contain;
  filter: drop-shadow(0 10px 18px rgba(22, 44, 74, 0.22));
}

.selection-dot {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border: 2px solid rgba(221, 239, 251, 0.58);
  border-radius: 50%;
  background: transparent;
  color: transparent;
  font-size: 10px;
  line-height: 1;
  font-weight: 950;
  transition: background 140ms ease, border-color 140ms ease, box-shadow 140ms ease;
}

.selection-dot span {
  display: block;
  transform: translateY(-0.5px);
  line-height: 1;
}

.selection-dot.checked {
  border-color: #53d8d1;
  background: #35c8c0;
  color: #fff;
  box-shadow: 0 0 0 3px rgba(53, 200, 192, 0.16);
}

.selection-dot.small {
  width: 16px;
  height: 16px;
  border-width: 1.5px;
  font-size: 10px;
}

.selection-dot.tiny {
  width: 14px;
  height: 14px;
  border-width: 1.5px;
  font-size: 9px;
}

.chevron,
.row-size {
  justify-self: end;
}

.chevron {
  color: rgba(235, 248, 255, 0.78);
  font-size: 22px;
}

.row-size {
  color: rgba(235, 248, 255, 0.74);
  font-size: 13px;
}

.file-tree {
  margin: -2px 0 8px 62px;
}

.file-group {
  display: grid;
  gap: 4px;
  margin: 6px 0 8px;
}

.group-row,
.file-row {
  display: grid;
  align-items: center;
  gap: 10px;
  min-height: 28px;
}

.group-row {
  grid-template-columns: 24px 1fr 82px;
  color: rgba(246, 252, 255, 0.92);
  font-size: 12px;
  font-weight: 900;
}

.file-row {
  grid-template-columns: 24px 1fr 82px;
  padding-left: 30px;
  color: rgba(238, 249, 255, 0.86);
  font-size: 12px;
  font-weight: 850;
}

.file-row span:nth-child(2) {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-icon,
.file-icon {
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  background: linear-gradient(180deg, #47d2ff, #0792d9);
  color: #dff9ff;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.38);
}

.file-icon {
  background: linear-gradient(180deg, #3ad0fa, #0a83c6);
  font-size: 10px;
}

.group-size,
.file-row strong {
  justify-self: end;
  color: rgba(239, 250, 255, 0.7);
}

.bottom-bar {
  position: fixed;
  right: 0;
  bottom: 0;
  left: 260px;
  z-index: 20;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 18px;
  height: 64px;
  background: rgba(49, 53, 122, 0.45);
  color: rgba(230, 242, 255, 0.76);
  font-size: 14px;
  font-weight: 850;
  backdrop-filter: blur(14px);
}

.uninstall-orb {
  width: 64px;
  height: 64px;
  margin-top: -18px;
  border: 3px solid #ff6f9f;
  border-radius: 50%;
  background: rgba(116, 146, 190, 0.86);
  color: #fff;
  font-size: 14px;
  font-weight: 950;
  box-shadow: 0 0 0 4px rgba(255, 117, 166, 0.22), 0 18px 42px rgba(28, 32, 78, 0.32);
}

.uninstall-orb:disabled {
  opacity: 0.62;
  cursor: not-allowed;
}

.confirm-backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(20, 29, 57, 0.58);
  backdrop-filter: blur(12px);
}

.confirm-dialog {
  display: grid;
  grid-template-columns: 48px minmax(0, 1fr);
  gap: 18px;
  width: min(480px, 100%);
  padding: 24px;
  border: 1px solid rgba(255, 255, 255, 0.28);
  border-radius: 8px;
  background: rgba(235, 246, 255, 0.97);
  color: #22314d;
  box-shadow: 0 28px 80px rgba(16, 25, 55, 0.42);
}

.confirm-symbol {
  display: grid;
  place-items: center;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: #ffe2e9;
  color: #bd3158;
  font-size: 25px;
  font-weight: 900;
}

.confirm-copy p {
  margin: 1px 0 6px;
  color: #bd3158;
  font-size: 12px;
  font-weight: 850;
}

.confirm-copy h2 {
  margin: 0;
  color: #1e2c46;
  font-size: 20px;
  line-height: 1.35;
}

.confirm-copy span {
  display: block;
  margin-top: 10px;
  color: #5c6b82;
  font-size: 13px;
  line-height: 1.6;
}

.confirm-actions {
  grid-column: 1 / -1;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 4px;
}

.confirm-actions button {
  min-height: 40px;
  padding: 0 17px;
  border-radius: 7px;
  font-weight: 800;
}

.confirm-cancel {
  border: 1px solid #b7c5d7;
  background: #fff;
  color: #42526b;
}

.confirm-submit {
  border: 1px solid #c7355e;
  background: #c7355e;
  color: #fff;
}

.confirm-submit:hover {
  background: #a9274c;
}

.report-panel {
  position: absolute;
  right: 28px;
  bottom: 76px;
  display: flex;
  align-items: center;
  gap: 10px;
  max-width: 520px;
  padding: 12px 14px;
  border-radius: 14px;
  background: rgba(18, 52, 79, 0.56);
  color: #ecf9ff;
  font-size: 13px;
  font-weight: 800;
}

.report-panel p {
  margin: 0;
}

@media (max-width: 1180px) {
  .uninstaller-body {
    grid-template-columns: 250px 1fr;
    gap: 36px;
    padding-right: 24px;
    padding-left: 24px;
  }

  .bottom-bar {
    left: 260px;
  }
}

@keyframes scan-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
