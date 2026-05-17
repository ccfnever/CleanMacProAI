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

    <main class="content-stage">
      <header class="topbar">
        <div>
          <p class="eyebrow">CleanMacProAI / {{ dataSource === "native" ? "Native" : "Demo" }}</p>
          <h2>{{ currentTitle }}</h2>
        </div>
        <div class="system-chip">
          <span>●</span>
          本地优先 · 安全模式
        </div>
      </header>

      <DashboardView
        v-if="currentView === 'dashboard'"
        :disk-info="diskInfo"
        :data-source="dataSource"
        @navigate="navigate"
      />
      <ScannerView v-else-if="currentView === 'scanner'" />
      <UninstallerView v-else-if="currentView === 'uninstaller'" />
      <SettingsView v-else-if="currentView === 'settings'" />
    </main>
  </div>
</template>

<style>
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
  background:
    radial-gradient(circle at 10% 0%, rgba(36, 137, 114, 0.16), transparent 30%),
    linear-gradient(145deg, #f6f8f7 0%, #edf2f1 45%, #f8f5ef 100%);
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
  flex: 1;
  overflow-y: auto;
  padding: 28px 34px 40px;
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  margin: 0 auto 22px;
  max-width: 1180px;
}

.topbar h2 {
  margin: 3px 0 0;
  font-size: 28px;
  letter-spacing: 0;
}

.eyebrow {
  margin: 0;
  color: #6f7d77;
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
  border: 1px solid rgba(23, 32, 38, 0.09);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.72);
  color: #395047;
  font-size: 13px;
  font-weight: 700;
  box-shadow: 0 16px 40px rgba(28, 49, 42, 0.08);
  backdrop-filter: blur(18px);
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
