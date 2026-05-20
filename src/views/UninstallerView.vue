<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import {
  demoApps,
  demoCleanReport,
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
const query = ref("");
const activeFacet = ref("status:all");
const selectedBundleIds = ref<Set<string>>(new Set());
const selectedChildKeys = ref<Set<string>>(new Set());
const excludedChildKeys = ref<Set<string>>(new Set());
const failedIconPaths = ref<Set<string>>(new Set());
const expandedBundleIds = ref<Set<string>>(new Set());
const notice = ref<string | null>(null);
const uninstallReport = ref<CleanReport | null>(null);
let inspectRequestId = 0;

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
    .sort((a, b) => appTotalSize(b) - appTotalSize(a));
});

const selectedApps = computed(() =>
  apps.value.filter((app) => selectedBundleIds.value.has(app.bundle_id)),
);

const selectedTotal = computed(() =>
  selectedApps.value.reduce((sum, app) => sum + appTotalSize(app), 0),
);

const totalAppsSize = computed(() =>
  apps.value.reduce((sum, app) => sum + appTotalSize(app), 0),
);

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
  if (!app.icon_path || failedIconPaths.value.has(app.icon_path)) return null;
  return convertFileSrc(app.icon_path);
}

function markIconFailed(app: InstalledApp) {
  if (!app.icon_path) return;
  failedIconPaths.value = new Set([...failedIconPaths.value, app.icon_path]);
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

function groupKey(app: InstalledApp, group: RelatedGroup): string {
  return `${app.bundle_id}:group:${group.id}`;
}

function fileKey(app: InstalledApp, group: RelatedGroup, file: FileInfo): string {
  return `${app.bundle_id}:file:${group.id}:${file.path}`;
}

function isGroupSelected(app: InstalledApp, group: RelatedGroup): boolean {
  const key = groupKey(app, group);
  if (excludedChildKeys.value.has(key)) return false;
  return selectedBundleIds.value.has(app.bundle_id) || selectedChildKeys.value.has(key);
}

function isFileSelected(app: InstalledApp, group: RelatedGroup, file: FileInfo): boolean {
  const key = fileKey(app, group, file);
  if (excludedChildKeys.value.has(groupKey(app, group)) || excludedChildKeys.value.has(key)) return false;
  return (
    selectedBundleIds.value.has(app.bundle_id) ||
    selectedChildKeys.value.has(groupKey(app, group)) ||
    selectedChildKeys.value.has(key)
  );
}

function toggleGroup(app: InstalledApp, group: RelatedGroup) {
  const selectedChildren = new Set(selectedChildKeys.value);
  const excludedChildren = new Set(excludedChildKeys.value);
  const key = groupKey(app, group);
  const keys = [key, ...group.files.map((file) => fileKey(app, group, file))];

  if (isGroupSelected(app, group)) {
    if (selectedBundleIds.value.has(app.bundle_id)) {
      keys.forEach((itemKey) => excludedChildren.add(itemKey));
    } else {
      keys.forEach((itemKey) => selectedChildren.delete(itemKey));
    }
  } else {
    keys.forEach((itemKey) => {
      excludedChildren.delete(itemKey);
      selectedChildren.add(itemKey);
    });
  }

  selectedChildKeys.value = selectedChildren;
  excludedChildKeys.value = excludedChildren;
  uninstallReport.value = null;
}

function toggleFile(app: InstalledApp, group: RelatedGroup, file: FileInfo) {
  const selectedChildren = new Set(selectedChildKeys.value);
  const excludedChildren = new Set(excludedChildKeys.value);
  const key = fileKey(app, group, file);

  if (isFileSelected(app, group, file)) {
    if (selectedBundleIds.value.has(app.bundle_id) || selectedChildKeys.value.has(groupKey(app, group))) {
      excludedChildren.add(key);
    }
    selectedChildren.delete(key);
  } else {
    excludedChildren.delete(key);
    selectedChildren.add(key);
  }

  selectedChildKeys.value = selectedChildren;
  excludedChildKeys.value = excludedChildren;
  uninstallReport.value = null;
}

onMounted(async () => {
  const result = await invokeOrDemo<InstalledApp[]>("list_installed_apps", demoApps, undefined, 5000);
  apps.value = result.data;
  selectedBundleIds.value = new Set(result.data.slice(0, 1).map((app) => app.bundle_id));
  expandedBundleIds.value = new Set(result.data.slice(0, 1).map((app) => app.bundle_id));
  notice.value = result.source === "demo" ? "应用枚举命令尚未返回真实列表，当前展示可交互 Demo 数据。" : null;
  isLoading.value = false;
  if (result.data[0]) void inspectApp(result.data[0].bundle_id);
});

async function inspectApp(bundleId: string) {
  if (!bundleId) return;

  const current = apps.value.find((app) => app.bundle_id === bundleId);
  if (!current || current.related_files.length > 0 || current.app_size > 0) return;

  const requestId = ++inspectRequestId;
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

  if (requestId !== inspectRequestId) return;

  apps.value = apps.value.map((app) => (app.bundle_id === bundleId ? result.data : app));
  if (result.source === "demo") {
    notice.value = "当前环境无法读取该应用详情，已保留快速列表数据。";
  }
  inspectingBundleId.value = null;
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
  excludedChildKeys.value = new Set(
    [...excludedChildKeys.value].filter((key) => !key.startsWith(`${bundleId}:`)),
  );
  selectedChildKeys.value = new Set(
    [...selectedChildKeys.value].filter((key) => !key.startsWith(`${bundleId}:`)),
  );
  uninstallReport.value = null;
}

function selectAllVisible() {
  const next = new Set(selectedBundleIds.value);
  for (const app of filteredApps.value) next.add(app.bundle_id);
  selectedBundleIds.value = next;
  selectedChildKeys.value = new Set();
  excludedChildKeys.value = new Set();
}

async function uninstallSelected() {
  if (selectedApps.value.length === 0) return;
  isUninstalling.value = true;
  const fallback: CleanReport = {
    ...demoCleanReport,
    cleaned_count: selectedApps.value.reduce((sum, app) => sum + app.related_count + 1, 0),
    freed_bytes: selectedTotal.value,
    snapshot_id: `demo-uninstall-${Date.now()}`,
  };
  const result = await invokeOrDemo<CleanReport>("uninstall_app", fallback, {
    bundleIds: selectedApps.value.map((app) => app.bundle_id),
    moveToTrash: true,
  });
  const removedIds = new Set(selectedBundleIds.value);
  uninstallReport.value = result.data;
  apps.value = apps.value.filter((item) => !removedIds.has(item.bundle_id));
  selectedBundleIds.value = new Set();
  selectedChildKeys.value = new Set();
  excludedChildKeys.value = new Set();
  expandedBundleIds.value = new Set();
  notice.value =
    result.source === "demo"
      ? `Demo：已从列表移除 ${removedIds.size} 个应用，请在 macOS App 中运行以执行真实卸载。`
      : `已将 ${removedIds.size} 个应用及关联残留移入废纸篓。`;
  isUninstalling.value = false;
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
          <span>{{ filteredApps.length }} 个应用程序</span>
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
                  <button
                    type="button"
                    :class="['selection-dot', 'small', { checked: isGroupSelected(app, group) }]"
                    :aria-label="`选择 ${group.label}`"
                    @click="toggleGroup(app, group)"
                  >
                    <span>✓</span>
                  </button>
                  <span class="group-icon">{{ groupIcon(group.id) }}</span>
                  <strong>{{ group.label }}</strong>
                  <span class="group-size">{{ formatBytes(group.size) }}</span>
                </div>
                <div v-for="file in group.files" :key="file.path" class="file-row">
                  <button
                    type="button"
                    :class="[
                      'selection-dot',
                      'tiny',
                      { checked: isFileSelected(app, group, file) },
                    ]"
                    :aria-label="`选择 ${file.path}`"
                    @click="toggleFile(app, group, file)"
                  >
                    <span>✓</span>
                  </button>
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
        :disabled="selectedBundleIds.size === 0 || isUninstalling"
        @click="uninstallSelected"
      >
        {{ isUninstalling ? "卸载中" : "卸载" }}
      </button>
      <span>{{ selectedTotal > 0 ? formatBytes(selectedTotal) : formatBytes(totalAppsSize) }}</span>
    </footer>

    <div v-if="uninstallReport" class="report-panel">
      <span>✓</span>
      <p>
        释放 {{ formatBytes(uninstallReport.freed_bytes) }}，处理
        {{ uninstallReport.cleaned_count.toLocaleString() }} 个文件。快照：{{ uninstallReport.snapshot_id }}
      </p>
    </div>
  </section>
</template>

<style scoped>
.uninstaller-page {
  position: relative;
  min-height: 100vh;
  margin: 0;
  padding: 0 0 86px;
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
  height: 58px;
  padding: 10px 18px 0 26px;
  color: rgba(233, 247, 255, 0.76);
  font-size: 15px;
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
  height: 40px;
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
  height: 36px;
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
  grid-template-columns: minmax(230px, 300px) minmax(520px, 1fr);
  gap: clamp(28px, 4vw, 70px);
  padding: 8px 42px 0 28px;
}

.facet-panel {
  color: rgba(233, 247, 255, 0.72);
}

.facet-section {
  display: grid;
  gap: 4px;
  margin-bottom: 22px;
}

.facet-section p {
  margin: 0 0 6px 14px;
  color: rgba(211, 232, 246, 0.46);
  font-size: 13px;
  font-weight: 900;
}

.facet-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 36px;
  padding: 0 12px;
  border: 0;
  border-radius: 18px;
  background: transparent;
  color: inherit;
  text-align: left;
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
  font-size: 13px;
}

.apps-panel {
  min-width: 520px;
}

.apps-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  margin-top: 14px;
}

.apps-head h1 {
  margin: 0;
  color: #fff;
  font-size: 32px;
  line-height: 1.1;
  letter-spacing: 0;
}

.apps-head p {
  margin: 12px 0 0;
  color: rgba(235, 248, 255, 0.74);
  font-size: 15px;
  font-weight: 750;
}

.select-visible,
.list-toolbar button {
  min-height: 34px;
  border: 0;
  border-radius: 999px;
  background: rgba(39, 80, 112, 0.35);
  color: rgba(239, 250, 255, 0.78);
  font-size: 13px;
  font-weight: 850;
}

.select-visible {
  padding: 0 14px;
}

.notice {
  margin: 16px 0 0;
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(255, 215, 92, 0.18);
  color: #fff4bf;
  font-size: 13px;
  font-weight: 800;
}

.list-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 38px 0 16px;
  color: rgba(235, 248, 255, 0.64);
  font-size: 14px;
  font-weight: 850;
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
  max-height: calc(100vh - 238px);
  padding-right: 14px;
  overflow: auto;
}

.app-node {
  border-radius: 14px;
}

.app-main {
  display: grid;
  grid-template-columns: 30px 1fr 26px 110px;
  align-items: center;
  gap: 12px;
  min-height: 58px;
  color: rgba(245, 251, 255, 0.93);
  font-weight: 900;
}

.expand-hit {
  display: flex;
  align-items: center;
  gap: 14px;
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
  width: 36px;
  height: 36px;
}

.app-icon-wrap {
  display: grid;
  place-items: center;
}

.app-icon,
.app-logo {
  display: grid;
  place-items: center;
  border-radius: 10px;
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
  width: 20px;
  height: 20px;
  border: 2px solid rgba(221, 239, 251, 0.58);
  border-radius: 50%;
  background: transparent;
  color: transparent;
  font-size: 11px;
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
  width: 18px;
  height: 18px;
  border-width: 1.5px;
  font-size: 10px;
}

.selection-dot.tiny {
  width: 16px;
  height: 16px;
  border-width: 1.5px;
  font-size: 9px;
}

.chevron,
.row-size {
  justify-self: end;
}

.chevron {
  color: rgba(235, 248, 255, 0.78);
  font-size: 28px;
}

.row-size {
  color: rgba(235, 248, 255, 0.74);
  font-size: 15px;
}

.file-tree {
  margin: -2px 0 12px 74px;
}

.file-group {
  display: grid;
  gap: 6px;
  margin: 8px 0 12px;
}

.group-row,
.file-row {
  display: grid;
  align-items: center;
  gap: 12px;
  min-height: 34px;
}

.group-row {
  grid-template-columns: 24px 28px 1fr 94px;
  color: rgba(246, 252, 255, 0.92);
  font-size: 14px;
  font-weight: 900;
}

.file-row {
  grid-template-columns: 24px 28px 1fr 94px;
  padding-left: 38px;
  color: rgba(238, 249, 255, 0.86);
  font-size: 13px;
  font-weight: 850;
}

.file-row span:nth-child(3) {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-icon,
.file-icon {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border-radius: 7px;
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
  position: absolute;
  right: 0;
  bottom: 0;
  left: 300px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 18px;
  height: 86px;
  background: rgba(49, 53, 122, 0.45);
  color: rgba(230, 242, 255, 0.76);
  font-size: 17px;
  font-weight: 850;
  backdrop-filter: blur(14px);
}

.uninstall-orb {
  width: 86px;
  height: 86px;
  margin-top: -28px;
  border: 4px solid #ff6f9f;
  border-radius: 50%;
  background: rgba(116, 146, 190, 0.86);
  color: #fff;
  font-size: 17px;
  font-weight: 950;
  box-shadow: 0 0 0 4px rgba(255, 117, 166, 0.22), 0 18px 42px rgba(28, 32, 78, 0.32);
}

.uninstall-orb:disabled {
  opacity: 0.62;
  cursor: not-allowed;
}

.report-panel {
  position: absolute;
  right: 28px;
  bottom: 100px;
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
    left: 0;
  }
}
</style>
