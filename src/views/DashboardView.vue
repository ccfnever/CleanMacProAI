<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { formatBytes, type DiskInfo, type RiskLevel } from "../lib/demoData";
import { useScannerStore } from "../stores/scanner";

const props = defineProps<{
  diskInfo: DiskInfo;
  dataSource: "native" | "demo";
}>();

const emit = defineEmits<{
  navigate: [view: string];
}>();

const scannerStore = useScannerStore();
const { scanResults, totalCleanable, totalFileCount } = storeToRefs(scannerStore);

const hasScanResults = computed(() => scanResults.value.length > 0);
const availableRatio = computed(() =>
  props.diskInfo.total_bytes > 0 ? (props.diskInfo.available_bytes / props.diskInfo.total_bytes) * 100 : 0,
);
const diskStatus = computed(() => {
  if (props.diskInfo.usage_percent >= 90) return "空间紧张";
  if (props.diskInfo.usage_percent >= 80) return "建议清理";
  return "状态良好";
});
const engineStatus = computed(() =>
  props.dataSource === "native" ? "已连接本机磁盘" : "等待本机运行环境",
);

const quickWins = computed(() => [
  {
    label: "可用空间",
    value: formatBytes(props.diskInfo.available_bytes),
    hint: `${availableRatio.value.toFixed(0)}% 仍可使用`,
    icon: "⌁",
  },
  {
    label: "可清理项",
    value: hasScanResults.value ? formatBytes(totalCleanable.value) : "待扫描",
    hint: hasScanResults.value
      ? `${scanResults.value.length} 类 · ${totalFileCount.value.toLocaleString()} 个文件`
      : "扫描后显示真实结果",
    icon: "◇",
  },
  {
    label: "清理方式",
    value: "安全模式",
    hint: "选中项先移入废纸篓",
    icon: "✓",
  },
]);

const diskOffset = computed(() => 314 - (314 * props.diskInfo.usage_percent) / 100);
const riskRows = computed(() =>
  [...scanResults.value]
    .sort((left, right) => right.total_size - left.total_size)
    .slice(0, 4)
    .map((item) => ({
      name: item.name,
      size: formatBytes(item.total_size),
      level: riskLabel(item.risk),
      color: riskColor(item.risk),
    })),
);

function riskLabel(risk: RiskLevel) {
  if (risk === "low") return "可放心清理";
  if (risk === "medium") return "建议确认";
  return "已锁定";
}

function riskColor(risk: RiskLevel) {
  if (risk === "low") return "#35c8c0";
  if (risk === "medium") return "#e2aa48";
  return "#e46f82";
}
</script>

<template>
  <section class="dashboard-page">
    <div class="hero-panel">
      <div class="hero-copy">
        <p class="section-kicker">Mac Storage Control</p>
        <h1>{{ diskStatus }}，来一次深度清理吧。</h1>
        <p>
          CleanMacProAI 只基于本机扫描结果给出建议。低风险缓存可直接处理，
          需要判断的项目会保留给你确认，高风险内容默认锁定。
        </p>

        <div class="hero-actions">
          <button type="button" class="primary-action" @click="emit('navigate', 'scanner')">
            <span>▶</span>
            {{ hasScanResults ? "查看扫描结果" : "开始智能扫描" }}
          </button>
          <button type="button" class="secondary-action" @click="emit('navigate', 'uninstaller')">
            <span>□</span>
            管理应用残留
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
        <small>{{ engineStatus }}</small>
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
            <h3>{{ hasScanResults ? "本次扫描发现" : "等待智能扫描" }}</h3>
          </div>
          <button type="button" @click="emit('navigate', 'scanner')">
            {{ hasScanResults ? "查看" : "扫描" }}
          </button>
        </div>
        <div v-if="riskRows.length" class="risk-list">
          <div v-for="row in riskRows" :key="row.name" class="risk-row">
            <span :style="{ background: row.color }"></span>
            <div>
              <strong>{{ row.name }}</strong>
              <small>{{ row.level }}</small>
            </div>
            <b>{{ row.size }}</b>
          </div>
        </div>
        <div v-else class="scan-empty">
          <strong>暂无扫描结果</strong>
          <p>完成一次智能扫描后，这里会显示真实的分类、大小和风险等级。</p>
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

.disk-orbit > small {
  position: absolute;
  bottom: 20px;
  color: rgba(235, 248, 255, 0.58);
  font-size: 12px;
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

.scan-empty {
  margin-top: 18px;
  padding: 18px;
  border: 1px dashed rgba(238, 249, 255, 0.22);
  border-radius: 13px;
  background: rgba(235, 248, 255, 0.08);
}

.scan-empty strong {
  color: #fff;
  font-size: 14px;
}

.scan-empty p {
  margin: 6px 0 0;
  color: rgba(235, 248, 255, 0.62);
  font-size: 13px;
  line-height: 1.55;
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
