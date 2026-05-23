<script setup lang="ts">
import { computed } from "vue";
import { demoApps, demoScanResult, formatBytes, type DiskInfo } from "../lib/demoData";

const props = defineProps<{
  diskInfo: DiskInfo;
  dataSource: "native" | "demo";
}>();

const emit = defineEmits<{
  navigate: [view: string];
}>();

const quickWins = computed(() => [
  {
    label: "预计可清理",
    value: formatBytes(demoScanResult.total_size),
    hint: "低风险项已默认勾选",
    icon: "⌁",
  },
  {
    label: "应用残留",
    value: formatBytes(demoApps.reduce((sum, app) => sum + app.related_size, 0)),
    hint: "来自应用卸载模块",
    icon: "□",
  },
  {
    label: "安全规则",
    value: "12 类",
    hint: "高风险默认锁定",
    icon: "✓",
  },
]);

const diskOffset = computed(() => 314 - (314 * props.diskInfo.usage_percent) / 100);
const riskRows = computed(() => [
  { name: "系统缓存", size: "2.83 GB", level: "安全", color: "#1f8b72" },
  { name: "下载残留", size: "1.62 GB", level: "需确认", color: "#d8872f" },
  { name: "邮件附件", size: "409 MB", level: "高风险", color: "#d64545" },
]);
</script>

<template>
  <section class="dashboard-page">
    <div class="hero-panel">
      <div class="hero-copy">
        <p class="section-kicker">安全清理控制台</p>
        <h1>给磁盘做一次大扫除吧。</h1>
        <p>
          当前 demo 已把磁盘状态、风险分层、扫描结果和应用残留串成完整路径。
          {{ dataSource === "native" ? "容量信息已来自本机。" : "现在处于浏览器 Demo 模式。" }}
        </p>

        <div class="hero-actions">
          <button type="button" class="primary-action" @click="emit('navigate', 'scanner')">
            <span>▶</span>
            开始智能扫描
          </button>
          <button type="button" class="secondary-action" @click="emit('navigate', 'uninstaller')">
            <span>□</span>
            查看应用残留
          </button>
        </div>
      </div>

      <div class="disk-orbit" aria-label="磁盘使用率">
        <svg viewBox="0 0 120 120">
          <circle cx="60" cy="60" r="50" class="track" />
          <circle
            cx="60"
            cy="60"
            r="50"
            class="progress"
            :stroke-dashoffset="diskOffset"
          />
        </svg>
        <div class="disk-center">
          <strong>{{ diskInfo.usage_percent.toFixed(0) }}%</strong>
          <span>已使用</span>
        </div>
      </div>
    </div>

    <div class="metric-grid">
      <article v-for="item in quickWins" :key="item.label" class="metric-card">
        <span>{{ item.icon }}</span>
        <p>{{ item.label }}</p>
        <strong>{{ item.value }}</strong>
        <small>{{ item.hint }}</small>
      </article>
    </div>

    <div class="two-column">
      <section class="panel">
        <div class="panel-head">
          <div>
            <p class="section-kicker">扫描预案</p>
            <h3>今天最值得处理的项目</h3>
          </div>
          <button type="button" @click="emit('navigate', 'scanner')">进入</button>
        </div>
        <div class="risk-list">
          <div v-for="row in riskRows" :key="row.name" class="risk-row">
            <span :style="{ background: row.color }"></span>
            <div>
              <strong>{{ row.name }}</strong>
              <small>{{ row.level }}</small>
            </div>
            <b>{{ row.size }}</b>
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-head">
          <div>
            <p class="section-kicker">容量</p>
            <h3>{{ diskInfo.volume_name }}</h3>
          </div>
        </div>
        <dl class="capacity-list">
          <div>
            <dt>总容量</dt>
            <dd>{{ formatBytes(diskInfo.total_bytes) }}</dd>
          </div>
          <div>
            <dt>已使用</dt>
            <dd>{{ formatBytes(diskInfo.used_bytes) }}</dd>
          </div>
          <div>
            <dt>可用空间</dt>
            <dd>{{ formatBytes(diskInfo.available_bytes) }}</dd>
          </div>
        </dl>
      </section>
    </div>
  </section>
</template>

<style scoped>
.dashboard-page {
  max-width: 1180px;
  margin: 0 auto;
}

.hero-panel {
  display: grid;
  grid-template-columns: 1fr 320px;
  gap: 28px;
  min-height: 280px;
  padding: 34px;
  border: 1px solid rgba(238, 249, 255, 0.18);
  border-radius: 24px;
  background:
    linear-gradient(135deg, rgba(255, 255, 255, 0.2), rgba(255, 255, 255, 0.08)),
    rgba(24, 75, 113, 0.32);
  box-shadow: 0 24px 80px rgba(22, 41, 88, 0.22);
  backdrop-filter: blur(18px);
}

.section-kicker {
  margin: 0 0 8px;
  color: rgba(171, 247, 232, 0.92);
  font-size: 12px;
  font-weight: 850;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

h1 {
  max-width: 680px;
  margin: 0;
  color: #fff;
  font-size: 38px;
  line-height: 1.04;
  letter-spacing: 0;
}

.hero-copy > p:not(.section-kicker) {
  max-width: 660px;
  margin: 16px 0 0;
  color: rgba(235, 248, 255, 0.74);
  font-size: 15px;
  line-height: 1.8;
}

.hero-actions {
  display: flex;
  gap: 12px;
  margin-top: 28px;
}

.primary-action,
.secondary-action,
.panel-head button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 44px;
  padding: 0 18px;
  border: 0;
  border-radius: 12px;
  font-weight: 850;
}

.primary-action {
  background: #ffffff;
  color: #315c7d;
  box-shadow: 0 16px 34px rgba(20, 48, 91, 0.18);
}

.primary-action:hover,
.secondary-action:hover,
.panel-head button:hover {
  transform: translateY(-1px);
}

.primary-action {
  transition: transform 160ms ease, box-shadow 160ms ease;
}

.secondary-action,
.panel-head button {
  transition: transform 160ms ease, background 160ms ease;
}

.secondary-action,
.panel-head button {
  background: rgba(235, 248, 255, 0.16);
  color: #fff;
}

.disk-orbit {
  position: relative;
  display: grid;
  place-items: center;
}

.disk-orbit svg {
  width: 220px;
  height: 220px;
  transform: rotate(-90deg);
}

.track,
.progress {
  fill: none;
  stroke-width: 11;
}

.track {
  stroke: rgba(235, 248, 255, 0.22);
}

.progress {
  stroke: #56e0d4;
  stroke-linecap: round;
  stroke-dasharray: 314;
  transition: stroke-dashoffset 320ms ease;
}

.disk-center {
  position: absolute;
  display: grid;
  place-items: center;
}

.disk-center strong {
  font-size: 44px;
  letter-spacing: 0;
  color: #fff;
}

.disk-center span {
  color: rgba(235, 248, 255, 0.7);
  font-size: 13px;
  font-weight: 800;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  margin-top: 18px;
}

.metric-card,
.panel {
  border: 1px solid rgba(238, 249, 255, 0.16);
  border-radius: 18px;
  background: rgba(28, 73, 109, 0.28);
  box-shadow: 0 18px 52px rgba(22, 41, 88, 0.16);
  backdrop-filter: blur(16px);
}

.metric-card {
  padding: 20px;
}

.metric-card > span {
  color: #6ce6dd;
  font-size: 22px;
}

.metric-card p {
  margin: 12px 0 3px;
  color: rgba(235, 248, 255, 0.66);
  font-size: 13px;
  font-weight: 800;
}

.metric-card strong {
  display: block;
  font-size: 28px;
  letter-spacing: 0;
  color: #fff;
}

.metric-card small {
  color: rgba(235, 248, 255, 0.58);
}

.two-column {
  display: grid;
  grid-template-columns: 1.2fr 0.8fr;
  gap: 18px;
  margin-top: 18px;
}

.panel {
  padding: 22px;
}

.panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.panel h3 {
  margin: 0;
  font-size: 18px;
  color: #fff;
}

.risk-list,
.capacity-list {
  display: grid;
  gap: 12px;
  margin-top: 18px;
}

.risk-row {
  display: grid;
  grid-template-columns: 10px 1fr auto;
  align-items: center;
  gap: 12px;
  padding: 14px;
  border-radius: 13px;
  background: rgba(235, 248, 255, 0.1);
}

.risk-row > span {
  width: 10px;
  height: 38px;
  border-radius: 999px;
}

.risk-row strong,
.risk-row b {
  display: block;
  font-size: 14px;
  color: #fff;
}

.risk-row small {
  color: rgba(235, 248, 255, 0.6);
}

.capacity-list {
  margin-bottom: 0;
}

.capacity-list div {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  padding-bottom: 12px;
  border-bottom: 1px solid rgba(238, 249, 255, 0.12);
}

.capacity-list dt {
  color: rgba(235, 248, 255, 0.62);
  font-weight: 800;
}

.capacity-list dd {
  margin: 0;
  font-weight: 900;
  color: #fff;
}
</style>
