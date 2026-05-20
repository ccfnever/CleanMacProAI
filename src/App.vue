<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import Sidebar from "./components/Sidebar.vue";
import DashboardView from "./views/DashboardView.vue";
import ScannerView from "./views/ScannerView.vue";
import UninstallerView from "./views/UninstallerView.vue";
import SettingsView from "./views/SettingsView.vue";
import { demoDiskInfo, invokeOrDemo, type DiskInfo } from "./lib/demoData";

type ViewName = "dashboard" | "scanner" | "uninstaller" | "settings";

const currentView = ref<ViewName>("dashboard");
const diskInfo = ref<DiskInfo>(demoDiskInfo);
const dataSource = ref<"native" | "demo">("demo");

const currentTitle = computed(() => {
  const titles: Record<ViewName, string> = {
    dashboard: "Mac 概览",
    scanner: "智能清理",
    uninstaller: "应用卸载",
    settings: "设置",
  };
  return titles[currentView.value];
});

const stageClass = computed(() => [
  "content-stage",
  `view-${currentView.value}`,
  { "uninstaller-stage": currentView.value === "uninstaller" },
]);

const viewTransitionName = computed(() =>
  currentView.value === "uninstaller" ? "view-fade-flat" : "view-fade",
);

function navigate(view: string) {
  if (["dashboard", "scanner", "uninstaller", "settings"].includes(view)) {
    currentView.value = view as ViewName;
  }
}

onMounted(async () => {
  const result = await invokeOrDemo<DiskInfo>("get_disk_info", demoDiskInfo);
  diskInfo.value = result.data;
  dataSource.value = result.source;
});
</script>

<template>
  <div class="app-shell">
    <Sidebar
      v-model:current-view="currentView"
      :disk-info="diskInfo"
      :data-source="dataSource"
    />

    <main :class="stageClass">
      <header v-if="currentView !== 'uninstaller'" class="topbar">
        <div>
          <p class="eyebrow">CleanMacProAI / {{ dataSource === "native" ? "Native" : "Demo" }}</p>
          <h2>{{ currentTitle }}</h2>
        </div>
        <div class="system-chip">
          <span>●</span>
          本地优先 · 安全模式
        </div>
      </header>

      <Transition :name="viewTransitionName" mode="out-in">
        <DashboardView
          v-if="currentView === 'dashboard'"
          key="dashboard"
          :disk-info="diskInfo"
          :data-source="dataSource"
          @navigate="navigate"
        />
        <ScannerView v-else-if="currentView === 'scanner'" key="scanner" />
        <UninstallerView v-else-if="currentView === 'uninstaller'" key="uninstaller" />
        <SettingsView v-else-if="currentView === 'settings'" key="settings" />
      </Transition>
    </main>
  </div>
</template>

<style>
@property --stage-top {
  syntax: "<color>";
  inherits: false;
  initial-value: rgba(92, 166, 207, 0.94);
}

@property --stage-bottom {
  syntax: "<color>";
  inherits: false;
  initial-value: rgba(65, 70, 132, 0.98);
}

@property --stage-glow {
  syntax: "<color>";
  inherits: false;
  initial-value: rgba(136, 220, 255, 0.2);
}

* {
  box-sizing: border-box;
}

html,
body,
#app {
  height: 100%;
}

body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: #172026;
  background: #4d6f9e;
}

button,
input,
select {
  font: inherit;
}

button {
  cursor: pointer;
}

.app-shell {
  display: flex;
  height: 100vh;
  min-width: 960px;
  overflow: hidden;
  color: #172026;
}

.content-stage {
  --stage-top: rgba(92, 166, 207, 0.94);
  --stage-bottom: rgba(65, 70, 132, 0.98);
  --stage-glow: rgba(136, 220, 255, 0.2);
  --stage-bg:
    radial-gradient(circle at 15% 4%, var(--stage-glow), transparent 34%),
    linear-gradient(180deg, var(--stage-top), var(--stage-bottom));
  flex: 1;
  overflow-y: auto;
  padding: 28px 34px 40px;
  color: #ecf8ff;
  background: var(--stage-bg);
  background-size: 140% 140%;
  animation: stage-drift 14s ease-in-out infinite alternate;
  transition:
    --stage-top 560ms ease,
    --stage-bottom 560ms ease,
    --stage-glow 560ms ease,
    color 320ms ease;
}

.content-stage.view-dashboard {
  --stage-top: rgba(74, 164, 187, 0.96);
  --stage-bottom: rgba(62, 72, 139, 0.98);
  --stage-glow: rgba(138, 228, 207, 0.24);
}

.content-stage.view-scanner {
  --stage-top: rgba(77, 148, 186, 0.96);
  --stage-bottom: rgba(55, 87, 128, 0.98);
  --stage-glow: rgba(255, 219, 124, 0.2);
}

.content-stage.view-settings {
  --stage-top: rgba(91, 136, 190, 0.96);
  --stage-bottom: rgba(75, 65, 134, 0.98);
  --stage-glow: rgba(201, 176, 255, 0.22);
}

.content-stage.view-uninstaller {
  --stage-top: rgba(92, 166, 207, 0.94);
  --stage-bottom: rgba(65, 70, 132, 0.98);
  --stage-glow: rgba(133, 221, 255, 0.18);
}

.content-stage.uninstaller-stage {
  overflow: hidden;
  padding: 0;
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  margin: 0 auto 22px;
  max-width: 1180px;
  color: rgba(239, 250, 255, 0.92);
}

.topbar h2 {
  margin: 3px 0 0;
  font-size: 28px;
  letter-spacing: 0;
  color: #fff;
}

.eyebrow {
  margin: 0;
  color: rgba(235, 248, 255, 0.62);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.system-chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 38px;
  padding: 0 14px;
  border: 1px solid rgba(238, 249, 255, 0.18);
  border-radius: 999px;
  background: rgba(28, 73, 109, 0.36);
  color: rgba(239, 250, 255, 0.9);
  font-size: 13px;
  font-weight: 700;
  box-shadow: 0 16px 40px rgba(22, 41, 88, 0.14);
  backdrop-filter: blur(18px);
}

.view-fade-enter-active,
.view-fade-leave-active {
  transition: opacity 220ms ease, transform 260ms ease, filter 260ms ease;
}

.view-fade-enter-from {
  opacity: 0;
  filter: blur(6px);
  transform: translateY(10px);
}

.view-fade-leave-to {
  opacity: 0;
  filter: blur(4px);
  transform: translateY(-8px);
}

.view-fade-flat-enter-active,
.view-fade-flat-leave-active {
  transition: opacity 180ms ease;
}

.view-fade-flat-enter-from,
.view-fade-flat-leave-to {
  opacity: 0;
}

@keyframes stage-drift {
  from {
    background-position: 0% 0%;
  }
  to {
    background-position: 100% 100%;
  }
}

::-webkit-scrollbar {
  width: 8px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.3);
}
</style>
