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

const navItems = [
  { id: "dashboard", label: "概览", icon: "⌘" },
  { id: "scanner", label: "智能清理", icon: "⌁" },
  { id: "uninstaller", label: "应用卸载", icon: "□" },
  { id: "settings", label: "设置", icon: "⚙" },
];

const availableText = computed(() => formatBytes(props.diskInfo.available_bytes));
const usageWidth = computed(() => `${Math.min(Math.max(props.diskInfo.usage_percent, 0), 100)}%`);

function selectView(id: string) {
  emit("update:current-view", id);
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
      <button
        v-for="item in navItems"
        :key="item.id"
        type="button"
        :class="['nav-item', { active: currentView === item.id }]"
        @click="selectView(item.id)"
      >
        <span class="nav-glyph">{{ item.icon }}</span>
        {{ item.label }}
      </button>
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
  </aside>
</template>

<style scoped>
.sidebar {
  width: 260px;
  padding: 18px;
  border-right: 1px solid rgba(35, 52, 45, 0.1);
  background: rgba(248, 250, 248, 0.78);
  backdrop-filter: blur(24px);
  display: flex;
  flex-direction: column;
  gap: 18px;
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
}

p {
  margin: 0;
}

.brand-row p {
  margin-top: 2px;
  color: #75847e;
  font-size: 12px;
}

.nav-list {
  display: grid;
  gap: 7px;
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
  color: #51615b;
  font-size: 14px;
  font-weight: 750;
  text-align: left;
  transition: background 160ms ease, color 160ms ease, transform 160ms ease;
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.76);
  color: #172026;
}

.nav-item.active {
  background: #172026;
  color: #fff;
  box-shadow: 0 16px 30px rgba(23, 32, 38, 0.16);
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
  border: 1px solid rgba(35, 52, 45, 0.09);
  border-radius: 16px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.88), rgba(255, 255, 255, 0.58)),
    #fff;
  box-shadow: 0 18px 44px rgba(26, 45, 38, 0.08);
}

.status-head {
  display: flex;
  justify-content: space-between;
  color: #2a3a34;
  font-size: 13px;
  font-weight: 800;
}

.meter {
  height: 9px;
  margin: 12px 0 9px;
  border-radius: 999px;
  background: #dce5e0;
  overflow: hidden;
}

.meter div {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #1f8b72, #ed9b40, #d64545);
}

.status-card p {
  color: #172026;
  font-size: 18px;
  font-weight: 850;
}

.status-card small {
  display: block;
  margin-top: 6px;
  color: #7b8984;
  font-size: 12px;
}
</style>
