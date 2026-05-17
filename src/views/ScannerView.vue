<script setup lang="ts">
import { computed, ref } from "vue";
import {
  demoCleanReport,
  demoScanResult,
  formatBytes,
  invokeOrDemo,
  type CategoryResult,
  type CleanReport,
  type ScanResult,
} from "../lib/demoData";

const phases = ["读取规则", "扫描缓存", "评估风险", "生成报告"];

const isScanning = ref(false);
const scanProgress = ref(0);
const activePhase = ref("等待开始");
const scanResults = ref<CategoryResult[]>([]);
const selectedCategories = ref<Set<string>>(new Set());
const expandedCategory = ref<string | null>(null);
const cleanReport = ref<CleanReport | null>(null);
const dataSource = ref<"native" | "demo">("demo");
const notice = ref<string | null>(null);

const selectedItems = computed(() =>
  scanResults.value.filter((item) => selectedCategories.value.has(item.id)),
);

const selectedTotal = computed(() =>
  selectedItems.value.reduce((sum, item) => sum + item.total_size, 0),
);

const totalCleanable = computed(() =>
  scanResults.value.reduce((sum, item) => sum + item.total_size, 0),
);

const safeCount = computed(() => scanResults.value.filter((item) => item.risk === "low").length);
const mediumCount = computed(() => scanResults.value.filter((item) => item.risk === "medium").length);
const highCount = computed(() => scanResults.value.filter((item) => item.risk === "high").length);

async function startScan() {
  isScanning.value = true;
  scanProgress.value = 0;
  activePhase.value = phases[0];
  scanResults.value = [];
  selectedCategories.value = new Set();
  expandedCategory.value = null;
  cleanReport.value = null;
  notice.value = null;

  for (let index = 0; index < phases.length; index += 1) {
    activePhase.value = phases[index];
    scanProgress.value = Math.round((index / phases.length) * 86);
    await new Promise((resolve) => window.setTimeout(resolve, 260));
  }

  const result = await invokeOrDemo<ScanResult>("scan_system", demoScanResult);
  scanResults.value = result.data.categories;
  selectedCategories.value = new Set(
    result.data.categories.filter((category) => category.risk === "low").map((category) => category.id),
  );
  dataSource.value = result.source;
  notice.value = result.source === "demo" ? "后端扫描命令尚未返回完整分类，当前展示高保真 Demo 数据。" : null;
  scanProgress.value = 100;
  activePhase.value = `完成，用时 ${(result.data.scan_duration_ms / 1000).toFixed(1)} 秒`;
  isScanning.value = false;
}

function toggleCategory(id: string, risk: string) {
  if (risk === "high") return;
  const next = new Set(selectedCategories.value);
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  selectedCategories.value = next;
}

function toggleExpanded(id: string) {
  expandedCategory.value = expandedCategory.value === id ? null : id;
}

function riskLabel(risk: string) {
  if (risk === "low") return "安全";
  if (risk === "medium") return "需确认";
  return "高风险";
}

async function cleanSelected() {
  const paths = selectedItems.value.flatMap((item) => item.files.map((file) => file.path));
  const fallback = {
    ...demoCleanReport,
    cleaned_count: selectedItems.value.reduce((sum, item) => sum + item.file_count, 0),
    freed_bytes: selectedTotal.value,
  };
  const result = await invokeOrDemo<CleanReport>("clean_items", fallback, {
    paths,
    safeMode: true,
  });
  cleanReport.value = result.data;
  notice.value =
    result.source === "demo"
      ? "清理执行仍处于 Demo 模式：前端已完成报告闭环，真实删除逻辑待 Rust 接入。"
      : "已按安全模式移入废纸篓。";
}
</script>

<template>
  <section class="scanner-page">
    <div class="scan-command">
      <div>
        <p class="section-kicker">智能清理</p>
        <h1>{{ scanResults.length ? formatBytes(totalCleanable) : "准备扫描" }}</h1>
        <p>
          默认只勾选低风险项目。中风险需要人工确认，高风险在 demo 中保持锁定，避免产品语义上误导用户。
        </p>
      </div>
      <button type="button" class="primary-action" :disabled="isScanning" @click="startScan">
        <span>{{ isScanning ? "◌" : "⌕" }}</span>
        {{ isScanning ? "扫描中" : scanResults.length ? "重新扫描" : "开始扫描" }}
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
        <span v-for="phase in phases" :key="phase">{{ phase }}</span>
      </div>
    </div>

    <p v-if="notice" class="notice">{{ notice }}</p>

    <div v-if="scanResults.length" class="result-layout">
      <aside class="summary-panel">
        <p>本次选择</p>
        <strong>{{ formatBytes(selectedTotal) }}</strong>
        <span>{{ selectedItems.length }} 类项目 · {{ selectedItems.reduce((sum, item) => sum + item.file_count, 0).toLocaleString() }} 个文件</span>
        <button type="button" :disabled="selectedCategories.size === 0" @click="cleanSelected">
          <span>⌫</span>
          安全清理选中项
        </button>
        <div class="risk-summary">
          <small>安全 {{ safeCount }}</small>
          <small>需确认 {{ mediumCount }}</small>
          <small>锁定 {{ highCount }}</small>
        </div>
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
              文件预览
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
      <h3>还没有扫描报告</h3>
      <p>点一次扫描，就能看到完整的分类、风险、文件预览和清理结果。</p>
    </div>

    <div v-if="cleanReport" class="report-panel">
      <span>✓</span>
      <div>
        <strong>清理报告已生成</strong>
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
  max-width: 1180px;
  margin: 0 auto;
}

.scan-command,
.progress-panel,
.summary-panel,
.category-card,
.empty-state,
.report-panel {
  border: 1px solid rgba(35, 52, 45, 0.09);
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.78);
  box-shadow: 0 18px 52px rgba(28, 49, 42, 0.08);
}

.scan-command {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 28px;
  padding: 28px;
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

.scan-command p:not(.section-kicker),
.empty-state p,
.report-panel p {
  max-width: 660px;
  margin: 10px 0 0;
  color: #687871;
  line-height: 1.75;
}

.primary-action,
.summary-panel button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 46px;
  padding: 0 18px;
  border: 0;
  border-radius: 12px;
  background: #172026;
  color: #fff;
  font-weight: 850;
}

.primary-action:disabled,
.summary-panel button:disabled {
  opacity: 0.56;
  cursor: not-allowed;
}

.progress-panel {
  margin-top: 16px;
  padding: 18px;
}

.progress-copy,
.phase-row {
  display: flex;
  justify-content: space-between;
  gap: 14px;
}

.progress-copy {
  font-size: 14px;
  font-weight: 850;
}

.progress-track {
  height: 10px;
  margin: 12px 0;
  border-radius: 999px;
  background: #dfe8e4;
  overflow: hidden;
}

.progress-track div {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #1f8b72, #2c6f98);
  transition: width 240ms ease;
}

.phase-row {
  color: #7b8984;
  font-size: 12px;
  font-weight: 750;
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

.result-layout {
  display: grid;
  grid-template-columns: 300px 1fr;
  gap: 18px;
  margin-top: 18px;
}

.summary-panel {
  position: sticky;
  top: 24px;
  height: fit-content;
  padding: 22px;
}

.summary-panel p {
  margin: 0;
  color: #7b8984;
  font-size: 13px;
  font-weight: 850;
}

.summary-panel strong {
  display: block;
  margin-top: 8px;
  font-size: 34px;
  letter-spacing: 0;
}

.summary-panel > span {
  display: block;
  margin: 4px 0 18px;
  color: #6f7d77;
  font-size: 13px;
}

.summary-panel button {
  width: 100%;
}

.risk-summary {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin-top: 16px;
}

.risk-summary small {
  padding: 9px 6px;
  border-radius: 10px;
  background: #f3f6f4;
  color: #53625c;
  font-size: 12px;
  font-weight: 850;
  text-align: center;
}

.category-list {
  display: grid;
  gap: 12px;
}

.category-card {
  overflow: hidden;
}

.category-card.selected {
  border-color: rgba(31, 139, 114, 0.42);
}

.category-main {
  display: grid;
  grid-template-columns: 32px 1fr 130px 76px;
  align-items: center;
  gap: 14px;
  width: 100%;
  padding: 18px;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
}

.checkmark {
  width: 28px;
  height: 28px;
  border: 2px solid #c5d1cc;
  border-radius: 9px;
  display: grid;
  place-items: center;
  color: #fff;
}

.selected .checkmark {
  border-color: #1f8b72;
  background: #1f8b72;
}

.high .checkmark {
  background: #e8ecea;
  color: #778680;
}

.category-copy strong,
.category-meta b {
  display: block;
  font-size: 15px;
}

.category-copy small,
.category-meta small {
  display: block;
  margin-top: 4px;
  color: #708079;
  font-size: 12px;
  line-height: 1.45;
}

.category-meta {
  text-align: right;
}

.risk-pill {
  justify-self: end;
  min-width: 64px;
  padding: 6px 8px;
  border-radius: 999px;
  background: rgba(31, 139, 114, 0.11);
  color: #1f6f5e;
  font-size: 12px;
  font-weight: 900;
  text-align: center;
}

.medium .risk-pill {
  background: rgba(216, 135, 47, 0.14);
  color: #916024;
}

.high .risk-pill {
  background: rgba(214, 69, 69, 0.12);
  color: #a43838;
}

.preview-head {
  padding: 0 18px 14px 64px;
}

.preview-head button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 0;
  background: transparent;
  color: #53625c;
  font-size: 12px;
  font-weight: 850;
}

.file-preview {
  display: grid;
  gap: 8px;
  padding: 0 18px 18px 64px;
}

.file-preview div {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 12px;
  border-radius: 10px;
  background: #f5f7f6;
  color: #50605a;
  font-size: 12px;
}

.file-preview span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty-state {
  display: grid;
  place-items: center;
  margin-top: 18px;
  padding: 56px 20px;
  text-align: center;
}

.empty-state > span {
  color: #1f8b72;
  font-size: 44px;
}

.empty-state h3 {
  margin: 14px 0 0;
}

.report-panel {
  display: flex;
  gap: 14px;
  align-items: flex-start;
  margin-top: 18px;
  padding: 18px;
}

.report-panel > span {
  color: #1f8b72;
  font-size: 24px;
}

.report-panel strong {
  font-size: 16px;
}
</style>
