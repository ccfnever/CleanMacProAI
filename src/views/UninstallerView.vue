<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { demoApps, formatBytes, invokeOrDemo, type InstalledApp } from "../lib/demoData";

const apps = ref<InstalledApp[]>([]);
const isLoading = ref(true);
const query = ref("");
const selectedBundleId = ref<string | null>(null);
const notice = ref<string | null>(null);

const filteredApps = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  if (!keyword) return apps.value;
  return apps.value.filter(
    (app) =>
      app.name.toLowerCase().includes(keyword) ||
      app.bundle_id.toLowerCase().includes(keyword) ||
      app.app_path.toLowerCase().includes(keyword),
  );
});

const selectedApp = computed(
  () => apps.value.find((app) => app.bundle_id === selectedBundleId.value) ?? filteredApps.value[0],
);

const totalRecoverable = computed(() =>
  apps.value.reduce((sum, app) => sum + app.related_size, 0),
);

onMounted(async () => {
  const result = await invokeOrDemo<InstalledApp[]>("list_installed_apps", demoApps);
  apps.value = result.data;
  selectedBundleId.value = result.data[0]?.bundle_id ?? null;
  notice.value = result.source === "demo" ? "应用枚举命令尚未返回真实列表，当前展示可交互 Demo 数据。" : null;
  isLoading.value = false;
});

function selectApp(bundleId: string) {
  selectedBundleId.value = bundleId;
}

async function uninstallApp(app: InstalledApp) {
  const result = await invokeOrDemo<string>("uninstall_app", "demo-uninstalled", {
    bundleId: app.bundle_id,
    moveToTrash: true,
  });
  apps.value = apps.value.filter((item) => item.bundle_id !== app.bundle_id);
  selectedBundleId.value = apps.value[0]?.bundle_id ?? null;
  notice.value =
    result.source === "demo"
      ? `Demo：已从列表移除 ${app.name}，真实卸载逻辑待 Rust 接入。`
      : `${app.name} 已移入废纸篓。`;
}
</script>

<template>
  <section class="uninstaller-page">
    <div class="toolbar-panel">
      <div>
        <p class="section-kicker">应用卸载</p>
        <h1>{{ formatBytes(totalRecoverable) }}</h1>
        <p>可跟随应用一起处理的缓存、日志、偏好残留。系统应用默认标记为只读。</p>
      </div>
      <label class="search-box">
        <span>⌕</span>
        <input v-model="query" type="search" placeholder="搜索应用、Bundle ID 或路径" />
      </label>
    </div>

    <p v-if="notice" class="notice">{{ notice }}</p>

    <div v-if="isLoading" class="loading-state">正在读取应用列表...</div>

    <div v-else class="uninstall-layout">
      <div class="app-list">
        <button
          v-for="app in filteredApps"
          :key="app.bundle_id"
          type="button"
          :class="['app-row', { active: selectedApp?.bundle_id === app.bundle_id }]"
          @click="selectApp(app.bundle_id)"
        >
          <span class="app-icon">{{ app.name.charAt(0) }}</span>
          <span class="app-copy">
            <strong>{{ app.name }}</strong>
            <small>{{ app.bundle_id }}</small>
          </span>
          <span class="app-size">{{ formatBytes(app.app_size + app.related_size) }}</span>
        </button>

        <div v-if="filteredApps.length === 0" class="empty-row">没有匹配的应用</div>
      </div>

      <aside v-if="selectedApp" class="detail-panel">
        <div class="detail-head">
          <span class="app-icon large">{{ selectedApp.name.charAt(0) }}</span>
          <div>
            <h3>{{ selectedApp.name }}</h3>
            <p>{{ selectedApp.bundle_id }}</p>
          </div>
        </div>

        <dl class="detail-list">
          <div>
            <dt>应用本体</dt>
            <dd>{{ formatBytes(selectedApp.app_size) }}</dd>
          </div>
          <div>
            <dt>关联残留</dt>
            <dd>{{ formatBytes(selectedApp.related_size) }}</dd>
          </div>
          <div>
            <dt>关联文件</dt>
            <dd>{{ selectedApp.related_count.toLocaleString() }}</dd>
          </div>
          <div>
            <dt>安装路径</dt>
            <dd>{{ selectedApp.app_path }}</dd>
          </div>
        </dl>

        <div class="cleanup-map">
          <p>将检查</p>
          <span>~/Library/Caches</span>
          <span>~/Library/Logs</span>
          <span>~/Library/Preferences</span>
          <span>~/Library/Application Support</span>
        </div>

        <button
          type="button"
          class="danger-action"
          :disabled="selectedApp.is_system_app"
          @click="uninstallApp(selectedApp)"
        >
          <span>⌫</span>
          安全卸载并移入废纸篓
        </button>
      </aside>
    </div>
  </section>
</template>

<style scoped>
.uninstaller-page {
  max-width: 1180px;
  margin: 0 auto;
}

.toolbar-panel,
.app-list,
.detail-panel,
.loading-state {
  border: 1px solid rgba(35, 52, 45, 0.09);
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.78);
  box-shadow: 0 18px 52px rgba(28, 49, 42, 0.08);
}

.toolbar-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 28px;
  padding: 26px;
}

.section-kicker {
  margin: 0 0 8px;
  color: #1f8b72;
  font-size: 12px;
  font-weight: 850;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

h1 {
  margin: 0;
  font-size: 42px;
  letter-spacing: 0;
}

.toolbar-panel p:not(.section-kicker) {
  margin: 8px 0 0;
  color: #687871;
}

.search-box {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 360px;
  min-height: 46px;
  padding: 0 14px;
  border-radius: 12px;
  background: #f4f7f5;
  color: #73827c;
}

.search-box input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: #172026;
}

.notice {
  margin: 14px 0 0;
  padding: 12px 14px;
  border-radius: 12px;
  background: rgba(216, 135, 47, 0.12);
  color: #83511f;
  font-size: 13px;
  font-weight: 750;
}

.loading-state {
  margin-top: 18px;
  padding: 40px;
  text-align: center;
}

.uninstall-layout {
  display: grid;
  grid-template-columns: 1fr 380px;
  gap: 18px;
  margin-top: 18px;
}

.app-list {
  overflow: hidden;
}

.app-row {
  display: grid;
  grid-template-columns: 42px 1fr auto;
  align-items: center;
  gap: 14px;
  width: 100%;
  padding: 16px 18px;
  border: 0;
  border-bottom: 1px solid rgba(35, 52, 45, 0.07);
  background: transparent;
  color: inherit;
  text-align: left;
}

.app-row:hover,
.app-row.active {
  background: #f3f7f5;
}

.app-icon {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  background: linear-gradient(135deg, #1f8b72, #2c6f98);
  color: #fff;
  font-weight: 950;
}

.app-icon.large {
  width: 58px;
  height: 58px;
  border-radius: 16px;
  font-size: 24px;
}

.app-copy strong {
  display: block;
  font-size: 15px;
}

.app-copy small {
  display: block;
  margin-top: 4px;
  color: #708079;
  font-size: 12px;
}

.app-size {
  color: #172026;
  font-weight: 900;
}

.empty-row {
  padding: 40px;
  color: #718079;
  text-align: center;
}

.detail-panel {
  height: fit-content;
  padding: 22px;
}

.detail-head {
  display: flex;
  align-items: center;
  gap: 14px;
}

.detail-head h3 {
  margin: 0;
  font-size: 22px;
}

.detail-head p {
  margin: 4px 0 0;
  color: #708079;
  font-size: 12px;
}

.detail-list {
  display: grid;
  gap: 12px;
  margin: 22px 0;
}

.detail-list div {
  display: flex;
  justify-content: space-between;
  gap: 18px;
  padding-bottom: 12px;
  border-bottom: 1px solid rgba(35, 52, 45, 0.08);
}

.detail-list dt {
  color: #708079;
  font-weight: 850;
}

.detail-list dd {
  max-width: 220px;
  margin: 0;
  overflow-wrap: anywhere;
  text-align: right;
  font-weight: 900;
}

.cleanup-map {
  display: grid;
  gap: 8px;
  padding: 14px;
  border-radius: 14px;
  background: #f5f7f6;
}

.cleanup-map p {
  margin: 0;
  color: #708079;
  font-size: 12px;
  font-weight: 850;
}

.cleanup-map span {
  font-size: 13px;
  font-weight: 750;
}

.danger-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  min-height: 46px;
  margin-top: 18px;
  border: 0;
  border-radius: 12px;
  background: #c74747;
  color: #fff;
  font-weight: 900;
}

.danger-action:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
</style>
