<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, ref } from "vue";
import { storeToRefs } from "pinia";
import Sidebar from "./components/Sidebar.vue";
import AppIcon from "./components/AppIcon.vue";
import DashboardView from "./views/DashboardView.vue";
import ScannerView from "./views/ScannerView.vue";
import UninstallerView from "./views/UninstallerView.vue";
import SettingsView from "./views/SettingsView.vue";
import { demoDiskInfo, invokeOrDemo, type DiskInfo } from "./lib/demoData";
import { useThemeStore, type ThemeName } from "./stores/theme";

type ViewName = "dashboard" | "scanner" | "space-map" | "uninstaller" | "settings";

const currentView = ref<ViewName>("dashboard");
const SpaceMapView = defineAsyncComponent(() => import("./views/SpaceMapView.vue"));
const diskInfo = ref<DiskInfo>(demoDiskInfo);
const dataSource = ref<"native" | "demo">("demo");
const diskNotice = ref<string | null>(null);
const themeStore = useThemeStore();
const { currentTheme } = storeToRefs(themeStore);

const currentTitle = computed(() => ({
  dashboard: "概览",
  scanner: "安心清理",
  "space-map": "空间地图",
  uninstaller: "应用管理",
  settings: "外观与设置",
})[currentView.value]);

const engineLabel = computed(() => dataSource.value === "native" ? "本机引擎在线" : "预览模式");

function navigate(view: string) {
  if (["dashboard", "scanner", "space-map", "uninstaller", "settings"].includes(view)) {
    currentView.value = view as ViewName;
  }
}

function changeTheme(event: Event) {
  themeStore.applyTheme((event.target as HTMLSelectElement).value as ThemeName);
}

onMounted(async () => {
  themeStore.initializeTheme();
  const result = await invokeOrDemo<DiskInfo>("get_disk_info", demoDiskInfo);
  if (result.source === "error") {
    dataSource.value = "demo";
    diskNotice.value = `无法读取本机磁盘信息：${result.error}`;
    return;
  }
  diskInfo.value = result.data;
  dataSource.value = result.source === "native" ? "native" : "demo";
});
</script>

<template>
  <div class="app-shell">
    <Sidebar v-model:current-view="currentView" :disk-info="diskInfo" :data-source="dataSource" />

    <main :class="['content-stage', `view-${currentView}`]">
      <header class="topbar" data-tauri-drag-region>
        <div class="page-heading">
          <span class="page-symbol">
            <AppIcon :name="currentView === 'scanner' ? 'shield' : currentView === 'space-map' ? 'map' : currentView === 'uninstaller' ? 'apps' : currentView === 'settings' ? 'settings' : 'home'" :size="18" />
          </span>
          <div>
            <h2>{{ currentTitle }}</h2>
            <p>把空间整理得清楚，也把选择权留给你</p>
          </div>
        </div>

        <div class="topbar-actions">
          <span :class="['engine-chip', { native: dataSource === 'native' }]">
            <i></i>{{ engineLabel }}
          </span>
          <label class="theme-select" aria-label="切换主题皮肤">
            <AppIcon name="leaf" :size="15" />
            <select :value="currentTheme" @change="changeTheme">
              <option value="pet">萌宠暖阳</option>
              <option value="nature">自然森林</option>
              <option value="classic">经典雾蓝</option>
            </select>
          </label>
        </div>
      </header>

      <p v-if="diskNotice" class="app-notice" role="alert">{{ diskNotice }}</p>

      <Transition name="view-fade" mode="out-in">
        <DashboardView v-if="currentView === 'dashboard'" key="dashboard" :disk-info="diskInfo" :data-source="dataSource" @navigate="navigate" />
        <ScannerView v-else-if="currentView === 'scanner'" key="scanner" />
        <SpaceMapView v-else-if="currentView === 'space-map'" key="space-map" />
        <UninstallerView v-else-if="currentView === 'uninstaller'" key="uninstaller" />
        <SettingsView v-else key="settings" />
      </Transition>
    </main>
  </div>
</template>

<style>
:root,
:root[data-theme="pet"] {
  color-scheme: light;
  --font-ui: -apple-system, BlinkMacSystemFont, "PingFang SC", "Helvetica Neue", sans-serif;
  --app-bg: #eee6dc;
  --sidebar-bg: #e7ddd1;
  --surface: #fffdf9;
  --surface-soft: #f7f0e7;
  --surface-strong: #f0e5d9;
  --text: #382f2a;
  --text-soft: #71665f;
  --text-faint: #9a8e85;
  --accent: #9c5e38;
  --accent-strong: #754128;
  --accent-soft: #ead5c4;
  --success: #648366;
  --warning: #a97632;
  --danger: #ad5c51;
  --border: rgba(75, 57, 47, .12);
  --border-strong: rgba(75, 57, 47, .2);
  --shadow: 0 18px 45px rgba(91, 67, 51, .1);
  --shadow-soft: 0 8px 24px rgba(91, 67, 51, .07);
  --hero-overlay: linear-gradient(90deg, rgba(41,31,26,.82) 0%, rgba(41,31,26,.36) 55%, rgba(41,31,26,.04) 100%);
}

:root[data-theme="nature"] {
  --app-bg: #e6ebe2;
  --sidebar-bg: #dce5d9;
  --surface: #fbfdf9;
  --surface-soft: #eef3eb;
  --surface-strong: #e0e9dd;
  --text: #29352c;
  --text-soft: #617064;
  --text-faint: #89968a;
  --accent: #4e7658;
  --accent-strong: #34563d;
  --accent-soft: #cfe0cf;
  --success: #4e7658;
  --warning: #8a733e;
  --danger: #9e5d56;
  --border: rgba(43,68,49,.12);
  --border-strong: rgba(43,68,49,.2);
  --shadow: 0 18px 45px rgba(48,72,53,.1);
  --shadow-soft: 0 8px 24px rgba(48,72,53,.07);
  --hero-overlay: linear-gradient(90deg, rgba(26,44,31,.84) 0%, rgba(26,44,31,.34) 58%, transparent 100%);
}

:root[data-theme="classic"] {
  --app-bg: #e8ebef;
  --sidebar-bg: #dfe4e9;
  --surface: #fcfdff;
  --surface-soft: #f0f3f6;
  --surface-strong: #e3e8ed;
  --text: #28333d;
  --text-soft: #64717c;
  --text-faint: #8a969f;
  --accent: #4e6f88;
  --accent-strong: #36556d;
  --accent-soft: #d2e0e9;
  --success: #557a6a;
  --warning: #8e703b;
  --danger: #a95858;
  --border: rgba(39,55,67,.12);
  --border-strong: rgba(39,55,67,.2);
  --shadow: 0 18px 45px rgba(48,64,76,.1);
  --shadow-soft: 0 8px 24px rgba(48,64,76,.07);
  --hero-overlay: linear-gradient(90deg, rgba(28,42,52,.82) 0%, rgba(28,42,52,.3) 58%, transparent 100%);
}

* { box-sizing: border-box; }
html, body, #app { height: 100%; }
body {
  margin: 0;
  overflow: hidden;
  color: var(--text);
  background: var(--app-bg);
  font: 14px/1.5 var(--font-ui);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
button, input, select { font: inherit; }
button { cursor: pointer; }
button:focus-visible, input:focus-visible, select:focus-visible {
  outline: 3px solid color-mix(in srgb, var(--accent) 28%, transparent);
  outline-offset: 2px;
}

.app-shell { display: flex; width: 100%; height: 100vh; min-width: 900px; overflow: hidden; background: var(--app-bg); }
.content-stage { flex: 1; min-width: 0; overflow-y: auto; padding: 0 32px 44px; background: var(--app-bg); transition: background 240ms ease; }
.topbar {
  position: sticky;
  z-index: 20;
  top: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  min-height: 76px;
  max-width: 1240px;
  margin: 0 auto;
  border-bottom: 1px solid var(--border);
  background: color-mix(in srgb, var(--app-bg) 86%, transparent);
  backdrop-filter: blur(18px);
}
.page-heading, .topbar-actions, .theme-select, .engine-chip { display: flex; align-items: center; }
.page-heading { gap: 11px; }
.page-symbol { display: grid; place-items: center; width: 34px; height: 34px; border: 1px solid var(--border); border-radius: 11px; color: var(--accent); background: var(--surface); box-shadow: var(--shadow-soft); }
.page-heading h2 { margin: 0; color: var(--text); font-size: 17px; letter-spacing: -.01em; }
.page-heading p { margin: 2px 0 0; color: var(--text-faint); font-size: 11px; }
.topbar-actions { gap: 9px; }
.engine-chip { gap: 7px; min-height: 34px; padding: 0 12px; border: 1px solid var(--border); border-radius: 999px; color: var(--text-soft); background: color-mix(in srgb, var(--surface) 76%, transparent); font-size: 11px; font-weight: 700; }
.engine-chip i { width: 7px; height: 7px; border-radius: 50%; background: var(--warning); box-shadow: 0 0 0 3px color-mix(in srgb, var(--warning) 14%, transparent); }
.engine-chip.native i { background: var(--success); box-shadow: 0 0 0 3px color-mix(in srgb, var(--success) 14%, transparent); }
.theme-select { gap: 7px; min-height: 34px; padding: 0 4px 0 10px; border: 1px solid var(--border); border-radius: 10px; color: var(--accent); background: var(--surface); }
.theme-select select { height: 30px; border: 0; outline: 0; color: var(--text); background: transparent; font-size: 11px; font-weight: 700; }
.app-notice { max-width: 1240px; margin: 14px auto 0; padding: 10px 13px; border: 1px solid color-mix(in srgb, var(--warning) 25%, transparent); border-radius: 10px; color: var(--text); background: color-mix(in srgb, var(--warning) 9%, var(--surface)); font-size: 12px; }
.view-fade-enter-active, .view-fade-leave-active { transition: opacity 160ms ease, transform 180ms ease; }
.view-fade-enter-from { opacity: 0; transform: translateY(5px); }
.view-fade-leave-to { opacity: 0; transform: translateY(-3px); }
::-webkit-scrollbar { width: 8px; height: 8px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { border-radius: 999px; background: color-mix(in srgb, var(--text) 18%, transparent); }

@media (max-width: 1060px) {
  .content-stage { padding-inline: 22px; }
  .page-heading p { display: none; }
}
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { scroll-behavior: auto !important; transition-duration: .01ms !important; animation-duration: .01ms !important; animation-iteration-count: 1 !important; }
}
</style>
