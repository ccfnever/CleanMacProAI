<script setup lang="ts">
import { computed } from "vue";
import { formatBytes, type DiskInfo } from "../lib/demoData";

const props = defineProps<{
  currentView: string;
  diskInfo: DiskInfo;
  dataSource: "native" | "demo";
}>();

const emit = defineEmits<{
  "update:current-view": [view: string];
}>();

const navSections = [
  {
    title: "",
    items: [{ id: "scanner", label: "智能扫描", icon: "▱" }],
  },
  {
    title: "清理",
    items: [
      { id: "dashboard", label: "系统垃圾", icon: "◷" },
      { id: "mail", label: "邮件附件", icon: "□" },
      { id: "trash", label: "废纸篓", icon: "▿" },
    ],
  },
  {
    title: "保护",
    items: [
      { id: "malware", label: "移除恶意软件", icon: "⌘" },
      { id: "privacy", label: "隐私", icon: "◴" },
    ],
  },
  {
    title: "速度",
    items: [
      { id: "optimize", label: "优化", icon: "≡" },
      { id: "settings", label: "维护", icon: "⌕" },
    ],
  },
  {
    title: "应用程序",
    items: [
      { id: "uninstaller", label: "卸载器", icon: "△" },
      { id: "updater", label: "更新程序", icon: "↻" },
      { id: "extensions", label: "扩展", icon: "⊞" },
    ],
  },
  {
    title: "文件",
    items: [
      { id: "space", label: "空间透镜", icon: "◌" },
      { id: "large", label: "大型和旧文件", icon: "▭" },
      { id: "shredder", label: "碎纸机", icon: "▥" },
    ],
  },
];

const availableText = computed(() => formatBytes(props.diskInfo.available_bytes));
const usageWidth = computed(() => `${Math.min(Math.max(props.diskInfo.usage_percent, 0), 100)}%`);

function selectView(id: string) {
  if (["dashboard", "scanner", "uninstaller", "settings"].includes(id)) {
    emit("update:current-view", id);
  }
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand-block">
      <div class="traffic-lights">
        <span></span>
        <span></span>
        <span></span>
      </div>

      <div class="brand-row">
        <div class="brand-mark">
          <span>⌁</span>
        </div>
        <div>
          <h1>CleanMacProAI</h1>
          <p>轻量 Mac 清理台</p>
        </div>
      </div>
    </div>

    <nav class="nav-list" aria-label="主导航">
      <div v-for="section in navSections" :key="section.title || 'scan'" class="nav-section">
        <p v-if="section.title">{{ section.title }}</p>
        <button
          v-for="item in section.items"
          :key="item.id"
          type="button"
          :class="['nav-item', { active: currentView === item.id }]"
          @click="selectView(item.id)"
        >
          <span class="nav-glyph">{{ item.icon }}</span>
          {{ item.label }}
        </button>
      </div>
    </nav>

    <div class="status-card">
      <div class="status-head">
        <span>{{ diskInfo.volume_name }}</span>
        <strong>{{ diskInfo.usage_percent.toFixed(0) }}%</strong>
      </div>
      <div class="meter">
        <div :style="{ width: usageWidth }"></div>
      </div>
      <p>{{ availableText }} 可用</p>
      <small>{{ dataSource === "native" ? "来自本机容量信息" : "Demo 数据，后端可接入" }}</small>
    </div>
    <button type="button" class="unlock-button">解锁完整版</button>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 260px;
  padding: 18px;
  border-right: 1px solid rgba(207, 228, 247, 0.18);
  background:
    linear-gradient(180deg, rgba(101, 168, 205, 0.84), rgba(71, 82, 139, 0.96)),
    #547ba6;
  backdrop-filter: blur(24px);
  color: #eaf7ff;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.brand-block {
  padding: 4px 4px 10px;
}

.traffic-lights {
  display: flex;
  gap: 8px;
  margin-bottom: 22px;
}

.traffic-lights span {
  width: 11px;
  height: 11px;
  border-radius: 50%;
}

.traffic-lights span:nth-child(1) {
  background: #ff5f57;
}

.traffic-lights span:nth-child(2) {
  background: #ffbd2e;
}

.traffic-lights span:nth-child(3) {
  background: #28c840;
}

.brand-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.brand-mark {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  background: linear-gradient(135deg, #1f8b72, #1d5f91);
  color: #fff;
  display: grid;
  place-items: center;
  font-size: 22px;
  box-shadow: 0 12px 26px rgba(31, 139, 114, 0.28);
}

h1 {
  margin: 0;
  font-size: 15px;
  letter-spacing: 0;
  color: #fff;
}

p {
  margin: 0;
}

.brand-row p {
  margin-top: 2px;
  color: rgba(235, 248, 255, 0.66);
  font-size: 12px;
}

.nav-list {
  display: grid;
  gap: 9px;
  overflow: auto;
}

.nav-section {
  display: grid;
  gap: 4px;
}

.nav-section p {
  margin: 0 0 2px 2px;
  color: rgba(228, 244, 255, 0.58);
  font-size: 12px;
  font-weight: 850;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 11px;
  width: 100%;
  min-height: 44px;
  padding: 0 13px;
  border: 0;
  border-radius: 10px;
  background: transparent;
  color: rgba(239, 250, 255, 0.86);
  font-size: 14px;
  font-weight: 750;
  text-align: left;
  transition: background 160ms ease, color 160ms ease, transform 160ms ease;
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.nav-item.active {
  background: rgba(45, 78, 125, 0.54);
  color: #fff;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.08);
}

.nav-glyph {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  font-size: 18px;
}

.status-card {
  margin-top: auto;
  padding: 16px;
  border: 1px solid rgba(238, 249, 255, 0.16);
  border-radius: 16px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.16), rgba(255, 255, 255, 0.08)),
    rgba(26, 59, 99, 0.28);
  box-shadow: 0 18px 44px rgba(31, 39, 83, 0.16);
}

.status-head {
  display: flex;
  justify-content: space-between;
  color: #fff;
  font-size: 13px;
  font-weight: 800;
}

.meter {
  height: 9px;
  margin: 12px 0 9px;
  border-radius: 999px;
  background: rgba(238, 249, 255, 0.24);
  overflow: hidden;
}

.meter div {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #1f8b72, #ed9b40, #d64545);
}

.status-card p {
  color: #fff;
  font-size: 18px;
  font-weight: 850;
}

.status-card small {
  display: block;
  margin-top: 6px;
  color: rgba(235, 248, 255, 0.66);
  font-size: 12px;
}

.unlock-button {
  min-height: 40px;
  border: 0;
  border-radius: 10px;
  background: #f2cf52;
  color: #405276;
  font-weight: 950;
}
</style>
