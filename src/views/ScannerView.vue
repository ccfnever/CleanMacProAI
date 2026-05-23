<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { formatBytes } from "../lib/demoData";
import { scanPhases, useScannerStore } from "../stores/scanner";

const scannerStore = useScannerStore();
const {
  activePhase,
  cleanReport,
  expandedCategory,
  highCount,
  isCleaning,
  isScanning,
  mediumCount,
  notice,
  safeCount,
  scanProgress,
  scanResults,
  selectedCategories,
  selectedFileCount,
  selectedItems,
  selectedTotal,
  totalCleanable,
  totalFileCount,
} = storeToRefs(scannerStore);

const { cleanSelected, startScan, toggleCategory, toggleExpanded } = scannerStore;

const scanHeadline = computed(() =>
  scanResults.value.length ? formatBytes(totalCleanable.value) : "先做一次快速体检",
);

const scanSubcopy = computed(() =>
  scanResults.value.length
    ? `已识别 ${scanResults.value.length} 类可处理项目，共 ${totalFileCount.value.toLocaleString()} 个文件。低风险项目已自动勾选，其他项目交给你确认。`
    : "扫描缓存、日志、构建文件和安装残留；先给出风险判断，再决定清理什么。",
);

const selectedSummary = computed(() =>
  selectedItems.value.length
    ? `${selectedItems.value.length} 类 · ${selectedFileCount.value.toLocaleString()} 个文件`
    : "还没有选择要清理的项目",
);

function riskLabel(risk: string) {
  if (risk === "low") return "可放心清理";
  if (risk === "medium") return "建议确认";
  return "已锁定";
}
</script>

<template>
  <section class="scanner-page">
    <div class="scan-command">
      <div class="scan-copy">
        <p class="section-kicker">Smart Scan</p>
        <h1>{{ scanHeadline }}</h1>
        <p>{{ scanSubcopy }}</p>
      </div>
      <button type="button" class="primary-action" :disabled="isScanning" @click="startScan">
        <span>{{ isScanning ? "◌" : "⌕" }}</span>
        {{ isScanning ? "正在扫描" : scanResults.length ? "重新扫描" : "开始扫描" }}
      </button>
    </div>

    <div class="progress-panel">
      <div class="progress-copy">
        <strong>{{ activePhase }}</strong>
        <span>{{ scanProgress.toFixed(0) }}%</span>
      </div>
      <div class="progress-track">
        <div :style="{ width: `${scanProgress}%` }"></div>
      </div>
      <div class="phase-row">
        <span v-for="phase in scanPhases" :key="phase">{{ phase }}</span>
      </div>
    </div>

    <p v-if="notice" class="notice">{{ notice }}</p>

    <div v-if="scanResults.length" class="result-layout">
      <aside class="summary-panel">
        <p class="summary-label">建议本次释放</p>
        <strong>{{ formatBytes(selectedTotal) }}</strong>
        <span>{{ selectedSummary }}</span>
        <button type="button" :disabled="selectedCategories.size === 0 || isCleaning" @click="cleanSelected">
          <span>{{ isCleaning ? "◌" : "⌫" }}</span>
          {{ isCleaning ? "清理中" : "清理已选安全项" }}
        </button>
        <div class="risk-summary">
          <small><b>{{ safeCount }}</b> 安全</small>
          <small><b>{{ mediumCount }}</b> 确认</small>
          <small><b>{{ highCount }}</b> 锁定</small>
        </div>
        <p class="summary-note">默认不碰文档、偏好设置、钥匙串和个人数据。</p>
      </aside>

      <div class="category-list">
        <article
          v-for="item in scanResults"
          :key="item.id"
          :class="['category-card', item.risk, { selected: selectedCategories.has(item.id) }]"
        >
          <button type="button" class="category-main" @click="toggleCategory(item.id, item.risk)">
            <span class="checkmark">
              <span v-if="selectedCategories.has(item.id)">✓</span>
              <span v-else-if="item.risk === 'high'">!</span>
            </span>
            <span class="category-copy">
              <strong>{{ item.name }}</strong>
              <small>{{ item.description }}</small>
            </span>
            <span class="category-meta">
              <b>{{ formatBytes(item.total_size) }}</b>
              <small>{{ item.file_count.toLocaleString() }} 个文件</small>
            </span>
            <span class="risk-pill">{{ riskLabel(item.risk) }}</span>
          </button>

          <div class="preview-head">
            <button type="button" @click="toggleExpanded(item.id)">
              <span>{{ expandedCategory === item.id ? "⌃" : "⌄" }}</span>
              查看样例文件
            </button>
          </div>

          <div v-if="expandedCategory === item.id" class="file-preview">
            <div v-for="file in item.files" :key="file.path">
              <span>{{ file.path }}</span>
              <strong>{{ formatBytes(file.size) }}</strong>
            </div>
          </div>
        </article>
      </div>
    </div>

    <div v-else-if="!isScanning" class="empty-state">
      <span>⌕</span>
      <h3>还没有扫描记录</h3>
      <p>开始后会自动归类可清理空间，并把风险最低的项目先选好。</p>
    </div>

    <div v-if="cleanReport" class="report-panel">
      <span>✓</span>
      <div>
        <strong>清理完成</strong>
        <p>
          释放 {{ formatBytes(cleanReport.freed_bytes) }}，处理 {{ cleanReport.cleaned_count.toLocaleString() }} 个文件，
          跳过 {{ cleanReport.skipped_count }} 个。快照：{{ cleanReport.snapshot_id }}
        </p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.scanner-page {
  max-width: 1120px;
  margin: 0 auto;
}

.scan-command,
.progress-panel,
.summary-panel,
.category-card,
.empty-state,
.report-panel {
  border: 1px solid rgba(238, 249, 255, 0.14);
  border-radius: 16px;
  background: rgba(26, 69, 103, 0.3);
  box-shadow: 0 18px 44px rgba(22, 41, 88, 0.14);
  backdrop-filter: blur(16px);
}

.scan-command {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 22px;
  padding: 22px 24px;
}

.scan-copy {
  min-width: 0;
}

.section-kicker {
  margin: 0 0 6px;
  color: rgba(171, 247, 232, 0.9);
  font-size: 11px;
  font-weight: 850;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

h1 {
  margin: 0;
  font-size: 34px;
  line-height: 1.05;
  letter-spacing: 0;
  color: #fff;
}

.scan-command p:not(.section-kicker),
.empty-state p,
.report-panel p {
  max-width: 680px;
  margin: 8px 0 0;
  color: rgba(235, 248, 255, 0.7);
  font-size: 14px;
  line-height: 1.55;
}

.primary-action,
.summary-panel button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 42px;
  padding: 0 16px;
  border: 0;
  border-radius: 11px;
  background: #ffffff;
  color: #315c7d;
  font-size: 14px;
  font-weight: 850;
  box-shadow: 0 14px 28px rgba(20, 48, 91, 0.18);
  transition: transform 160ms ease, box-shadow 160ms ease;
  white-space: nowrap;
}

.primary-action:hover,
.summary-panel button:hover {
  transform: translateY(-1px);
  box-shadow: 0 18px 32px rgba(20, 48, 91, 0.2);
}

.primary-action:disabled,
.summary-panel button:disabled {
  opacity: 0.56;
  cursor: not-allowed;
  transform: none;
}

.progress-panel {
  margin-top: 12px;
  padding: 14px 16px;
}

.progress-copy,
.phase-row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

.progress-copy {
  font-size: 13px;
  font-weight: 850;
  color: #fff;
}

.progress-track {
  height: 8px;
  margin: 10px 0;
  border-radius: 999px;
  background: rgba(235, 248, 255, 0.2);
  overflow: hidden;
}

.progress-track div {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #53d8d1, #9ae6ff);
  transition: width 240ms ease;
}

.phase-row {
  color: rgba(235, 248, 255, 0.58);
  font-size: 11px;
  font-weight: 750;
}

.notice {
  margin: 12px 0 0;
  padding: 10px 12px;
  border: 1px solid rgba(255, 215, 92, 0.2);
  border-radius: 11px;
  background: rgba(255, 215, 92, 0.14);
  color: #fff4bf;
  font-size: 12px;
  font-weight: 750;
}

.result-layout {
  display: grid;
  grid-template-columns: 268px minmax(0, 1fr);
  gap: 14px;
  margin-top: 14px;
  align-items: start;
}

.summary-panel {
  position: sticky;
  top: 20px;
  height: fit-content;
  padding: 18px;
}

.summary-label {
  margin: 0;
  color: rgba(235, 248, 255, 0.62);
  font-size: 12px;
  font-weight: 850;
}

.summary-panel strong {
  display: block;
  margin-top: 6px;
  font-size: 30px;
  line-height: 1.08;
  letter-spacing: 0;
  color: #fff;
}

.summary-panel > span {
  display: block;
  margin: 4px 0 14px;
  color: rgba(235, 248, 255, 0.68);
  font-size: 12px;
  line-height: 1.4;
}

.summary-panel button {
  width: 100%;
}

.risk-summary {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 7px;
  margin-top: 12px;
}

.risk-summary small {
  display: grid;
  gap: 1px;
  padding: 8px 5px;
  border-radius: 9px;
  background: rgba(235, 248, 255, 0.11);
  color: rgba(235, 248, 255, 0.8);
  font-size: 11px;
  font-weight: 800;
  text-align: center;
}

.risk-summary b {
  color: #fff;
  font-size: 13px;
}

.summary-note {
  margin: 12px 0 0;
  color: rgba(235, 248, 255, 0.56);
  font-size: 12px;
  line-height: 1.5;
}

.category-list {
  display: grid;
  gap: 10px;
  min-width: 0;
}

.category-card {
  overflow: hidden;
}

.category-card.selected {
  border-color: rgba(83, 216, 209, 0.5);
  background: rgba(44, 109, 139, 0.34);
}

.category-main {
  display: grid;
  grid-template-columns: 28px minmax(0, 1fr) 116px 88px;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 14px 16px 10px;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
}

.checkmark {
  width: 26px;
  height: 26px;
  border: 2px solid rgba(221, 239, 251, 0.54);
  border-radius: 8px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 14px;
  font-weight: 900;
}

.selected .checkmark {
  border-color: #53d8d1;
  background: #35c8c0;
}

.high .checkmark {
  background: rgba(235, 248, 255, 0.12);
  color: rgba(235, 248, 255, 0.62);
}

.category-copy {
  min-width: 0;
}

.category-copy strong,
.category-meta b {
  display: block;
  color: #fff;
  font-size: 14px;
  line-height: 1.25;
}

.category-copy small,
.category-meta small {
  display: block;
  margin-top: 3px;
  color: rgba(235, 248, 255, 0.6);
  font-size: 12px;
  line-height: 1.35;
}

.category-meta {
  text-align: right;
}

.risk-pill {
  justify-self: end;
  min-width: 76px;
  padding: 6px 8px;
  border-radius: 999px;
  background: rgba(83, 216, 209, 0.14);
  color: #bffaf6;
  font-size: 11px;
  font-weight: 900;
  text-align: center;
}

.medium .risk-pill {
  background: rgba(255, 215, 92, 0.16);
  color: #fff2ad;
}

.high .risk-pill {
  background: rgba(255, 122, 140, 0.14);
  color: #ffd4db;
}

.preview-head {
  padding: 0 16px 12px 56px;
}

.preview-head button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 0;
  background: transparent;
  color: rgba(235, 248, 255, 0.68);
  font-size: 12px;
  font-weight: 850;
}

.file-preview {
  display: grid;
  gap: 7px;
  padding: 0 16px 14px 56px;
}

.file-preview div {
  display: flex;
  justify-content: space-between;
  gap: 14px;
  padding: 8px 10px;
  border-radius: 9px;
  background: rgba(235, 248, 255, 0.1);
  color: rgba(235, 248, 255, 0.82);
  font-size: 12px;
}

.file-preview span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-preview strong {
  white-space: nowrap;
}

.empty-state {
  display: grid;
  place-items: center;
  margin-top: 14px;
  padding: 42px 20px;
  text-align: center;
}

.empty-state > span {
  color: #6ce6dd;
  font-size: 36px;
}

.empty-state h3 {
  margin: 10px 0 0;
  color: #fff;
  font-size: 18px;
}

.report-panel {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  margin-top: 14px;
  padding: 14px 16px;
}

.report-panel > span {
  color: #6ce6dd;
  font-size: 22px;
}

.report-panel strong {
  font-size: 15px;
  color: #fff;
}

@media (max-width: 1080px) {
  .scan-command {
    grid-template-columns: 1fr;
  }

  .primary-action {
    justify-self: start;
  }

  .result-layout {
    grid-template-columns: 1fr;
  }

  .summary-panel {
    position: static;
  }
}
</style>
