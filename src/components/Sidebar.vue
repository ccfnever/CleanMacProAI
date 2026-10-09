<script setup lang="ts">
import { computed } from "vue";
import AppIcon from "./AppIcon.vue";
import { formatBytes, type DiskInfo } from "../lib/demoData";

const props = defineProps<{
  currentView: string;
  diskInfo: DiskInfo;
  dataSource: "native" | "demo";
}>();

const emit = defineEmits<{ "update:current-view": [view: string] }>();

const navItems = [
  { id: "dashboard", label: "概览", description: "状态与建议", icon: "home" },
  { id: "scanner", label: "安心清理", description: "扫描与确认", icon: "shield" },
  { id: "space-map", label: "空间地图", description: "目录占用分析", icon: "map" },
  { id: "uninstaller", label: "应用管理", description: "应用与残留", icon: "apps" },
  { id: "startup", label: "启动项管理", description: "登录与后台启动", icon: "power" },
  { id: "settings", label: "外观与设置", description: "主题和权限", icon: "settings" },
];

const availableText = computed(() => formatBytes(props.diskInfo.available_bytes));
const totalText = computed(() => formatBytes(props.diskInfo.total_bytes));
const usageWidth = computed(() => `${Math.min(Math.max(props.diskInfo.usage_percent, 0), 100)}%`);

function selectView(id: string) {
  if (["dashboard", "scanner", "space-map", "uninstaller", "startup", "settings"].includes(id)) {
    emit("update:current-view", id);
  }
}
</script>

<template>
  <aside class="sidebar">
    <div class="traffic-lights" aria-hidden="true"><span></span><span></span><span></span></div>

    <div class="brand-row">
      <div class="brand-mark"><AppIcon name="paw" :size="23" /></div>
      <div>
        <h1>CleanMacProAI</h1>
        <p>轻巧、透明的 Mac 管家</p>
      </div>
    </div>

    <p class="nav-label">工作台</p>
    <nav class="nav-list" aria-label="主导航">
      <button
        v-for="item in navItems"
        :key="item.id"
        type="button"
        :class="['nav-item', { active: currentView === item.id }]"
        :aria-current="currentView === item.id ? 'page' : undefined"
        @click="selectView(item.id)"
      >
        <span class="nav-glyph"><AppIcon :name="item.icon" :size="19" /></span>
        <span class="nav-copy"><strong>{{ item.label }}</strong><small>{{ item.description }}</small></span>
        <span class="nav-arrow">›</span>
      </button>
    </nav>

    <section class="status-card" aria-label="磁盘容量">
      <div class="status-head">
        <span><AppIcon name="disk" :size="15" /> {{ diskInfo.volume_name }}</span>
        <strong>{{ diskInfo.usage_percent.toFixed(0) }}%</strong>
      </div>
      <div class="meter" role="progressbar" aria-label="磁盘使用率" :aria-valuenow="diskInfo.usage_percent" aria-valuemin="0" aria-valuemax="100">
        <div :style="{ width: usageWidth }"></div>
      </div>
      <p><strong>{{ availableText }}</strong> 可用</p>
      <small>总容量 {{ totalText }} · {{ dataSource === "native" ? "实时读取" : "预览数据" }}</small>
    </section>
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex: 0 0 216px;
  flex-direction: column;
  width: 216px;
  min-height: 100vh;
  padding: 19px 14px 16px;
  border-right: 1px solid var(--border);
  color: var(--text);
  background: var(--sidebar-bg);
  transition: background 240ms ease;
}

.traffic-lights { display: flex; gap: 7px; padding: 0 7px; }
.traffic-lights span { width: 10px; height: 10px; border-radius: 50%; }
.traffic-lights span:nth-child(1) { background: #ff5f57; }
.traffic-lights span:nth-child(2) { background: #febc2e; }
.traffic-lights span:nth-child(3) { background: #28c840; }

.brand-row { display: flex; align-items: center; gap: 10px; margin: 23px 5px 28px; }
.brand-mark { display: grid; place-items: center; flex: 0 0 39px; width: 39px; height: 39px; border-radius: 13px; color: #fff; background: var(--accent); box-shadow: 0 10px 24px color-mix(in srgb, var(--accent) 25%, transparent); }
.brand-row h1 { margin: 0; color: var(--text); font-size: 13px; letter-spacing: -.02em; }
.brand-row p { margin: 3px 0 0; color: var(--text-faint); font-size: 9px; white-space: nowrap; }
.nav-label { margin: 0 9px 8px; color: var(--text-faint); font-size: 10px; font-weight: 800; letter-spacing: .08em; }
.nav-list { display: grid; gap: 4px; }
.nav-item { display: grid; grid-template-columns: 32px 1fr auto; align-items: center; gap: 8px; width: 100%; min-height: 52px; padding: 5px 8px; border: 1px solid transparent; border-radius: 12px; color: var(--text-soft); background: transparent; text-align: left; transition: 150ms ease; }
.nav-item:hover { color: var(--text); background: color-mix(in srgb, var(--surface) 55%, transparent); }
.nav-item.active { border-color: color-mix(in srgb, var(--accent) 15%, var(--border)); color: var(--accent-strong); background: var(--surface); box-shadow: var(--shadow-soft); }
.nav-glyph { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 9px; background: color-mix(in srgb, var(--surface) 60%, transparent); }
.nav-item.active .nav-glyph { color: var(--accent); background: var(--accent-soft); }
.nav-copy { min-width: 0; }
.nav-copy strong, .nav-copy small { display: block; }
.nav-copy strong { font-size: 12px; }
.nav-copy small { margin-top: 2px; overflow: hidden; color: var(--text-faint); font-size: 9px; text-overflow: ellipsis; white-space: nowrap; }
.nav-arrow { color: var(--text-faint); font-size: 18px; opacity: 0; transform: translateX(-3px); transition: 150ms ease; }
.nav-item.active .nav-arrow, .nav-item:hover .nav-arrow { opacity: 1; transform: none; }

.status-card { margin-top: auto; padding: 14px; border: 1px solid var(--border); border-radius: 15px; background: color-mix(in srgb, var(--surface) 74%, transparent); box-shadow: var(--shadow-soft); }
.status-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; color: var(--text-soft); font-size: 10px; }
.status-head span { display: flex; align-items: center; gap: 6px; min-width: 0; }
.status-head strong { color: var(--text); font-size: 11px; }
.meter { height: 6px; margin: 11px 0 10px; overflow: hidden; border-radius: 999px; background: var(--surface-strong); }
.meter div { height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--success), var(--warning)); }
.status-card p { margin: 0; color: var(--text-soft); font-size: 10px; }
.status-card p strong { color: var(--text); font-size: 16px; }
.status-card small { display: block; margin-top: 4px; color: var(--text-faint); font-size: 9px; line-height: 1.4; }

@media (max-width: 1060px) {
  .sidebar { flex-basis: 190px; width: 190px; }
  .brand-row p, .nav-copy small { display: none; }
}
</style>
