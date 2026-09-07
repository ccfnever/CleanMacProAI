<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { formatBytes, type CategoryResult, type FileInfo } from "../lib/demoData";
import { scanPhases, useScannerStore } from "../stores/scanner";

const scannerStore = useScannerStore();
const {
  activePhase,
  cleanPhase,
  cleanProgress,
  cleanReport,
  dataSource,
  expandedCategory,
  highCount,
  isAllSelectableSelected,
  isCleaning,
  isScanning,
  isSelectionPartial,
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

const {
  cleanSelected,
  invertCategorySelection,
  startScan,
  toggleAllCategories,
  toggleCategory,
  toggleExpanded,
} = scannerStore;
const detailModalCategoryId = ref<string | null>(null);
const openingPath = ref<string | null>(null);

const scanHeadline = computed(() =>
  scanResults.value.length
    ? formatBytes(totalCleanable.value)
    : cleanReport.value ? "本次清理已完成" : "先做一次快速体检",
);

const scanSubcopy = computed(() =>
  scanResults.value.length
    ? `已识别 ${scanResults.value.length} 类可处理项目，共 ${totalFileCount.value.toLocaleString()} 个文件。低风险项目已自动勾选，其他项目交给你确认。`
    : cleanReport.value
      ? "当前扫描结果已在本地同步更新，可以重新扫描确认剩余空间。"
      : "扫描缓存、日志、构建文件和安装残留；先给出风险判断，再决定清理什么。",
);

const selectedSummary = computed(() =>
  selectedItems.value.length
    ? `${selectedItems.value.length} 类 · ${selectedFileCount.value.toLocaleString()} 个文件`
    : "还没有选择要清理的项目",
);

const detailModalCategory = computed(() =>
  scanResults.value.find((item) => item.id === detailModalCategoryId.value) ?? null,
);

const detailModalItems = computed(() =>
  detailModalCategory.value ? detailItems(detailModalCategory.value) : [],
);

function riskLabel(risk: string) {
  if (risk === "low") return "可放心清理";
  if (risk === "medium") return "建议确认";
  return "已锁定";
}

function detailItems(item: CategoryResult): FileInfo[] {
  return [...item.files].sort((left, right) => right.size - left.size);
}

function previewDetailItems(item: CategoryResult): FileInfo[] {
  return detailItems(item).slice(0, 20);
}

function detailKind(item: FileInfo) {
  return item.is_dir ? "文件夹" : "文件";
}

function detailGlyph(item: FileInfo) {
  return item.is_dir ? "▣" : "•";
}

function openDetailModal(categoryId: string) {
  detailModalCategoryId.value = categoryId;
}

function closeDetailModal() {
  detailModalCategoryId.value = null;
}

async function openFolder(path: string) {
  if (openingPath.value) return;
  openingPath.value = path;
  try {
    await invoke("open_in_finder", { path });
  } catch (error) {
    notice.value = "无法打开文件夹：" + (error instanceof Error ? error.message : String(error));
  } finally {
    openingPath.value = null;
  }
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

    <div v-if="isCleaning" class="cleaning-progress" role="status" aria-live="polite">
      <strong>{{ cleanPhase }}</strong>
      <span>{{ cleanProgress }}%</span>
      <div
        class="cleaning-progress-track"
        role="progressbar"
        aria-label="清理进度"
        :aria-valuenow="cleanProgress"
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <span :style="{ width: `${cleanProgress}%` }"></span>
      </div>
    </div>

    <div v-if="cleanReport" class="report-panel report-top">
      <span>✓</span><div><strong>{{ cleanReport.errors.length ? "清理部分完成" : "清理完成" }}</strong>
      <p>释放 {{ formatBytes(cleanReport.freed_bytes) }}，处理 {{ cleanReport.cleaned_count.toLocaleString() }} 个文件，跳过 {{ cleanReport.skipped_count }} 个。项目已移入废纸篓，可在 macOS 废纸篓中自行恢复。</p></div>
    </div>

    <div v-if="scanResults.length" class="result-layout">
      <aside class="summary-panel">
        <p class="summary-label">建议本次释放</p>
        <strong>{{ formatBytes(selectedTotal) }}</strong>
        <span>{{ selectedSummary }}</span>
        <div class="selection-actions">
          <label class="select-all-control">
            <input
              type="checkbox"
              :checked="isAllSelectableSelected"
              :indeterminate="isSelectionPartial"
              :disabled="isCleaning"
              @change="toggleAllCategories"
            >
            <span>{{ isAllSelectableSelected ? "取消全选" : "全选可清理项" }}</span>
          </label>
          <button
            type="button"
            class="invert-selection-button"
            :disabled="isCleaning"
            @click="invertCategorySelection"
          >
            反选
          </button>
        </div>
        <button
          type="button"
          :disabled="selectedCategories.size === 0 || isCleaning || dataSource === 'demo'"
          :title="dataSource === 'demo' ? '演示模式不会执行清理' : undefined"
          @click="cleanSelected"
        >
          <span>{{ isCleaning ? "◌" : "⌫" }}</span>
          {{ isCleaning ? "清理中" : "清理已选项目" }}
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
          <button
            type="button"
            class="category-main"
            :disabled="isCleaning"
            @click="toggleCategory(item.id, item.risk)"
          >
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
              展开详情
            </button>
            <small v-if="item.files.length">{{ item.files.length.toLocaleString() }} 个一级项目</small>
          </div>

          <div v-if="expandedCategory === item.id" class="detail-preview">
            <div v-if="item.files.length" class="detail-list">
              <div v-for="file in previewDetailItems(item)" :key="file.path" class="detail-row">
                <span class="detail-kind">{{ detailGlyph(file) }}</span>
                <span class="detail-path">
                  <strong>{{ file.path }}</strong>
                  <small>{{ detailKind(file) }}</small>
                </span>
                <button
                  type="button"
                  class="open-folder-button"
                  :disabled="dataSource === 'demo' || openingPath === file.path"
                  :title="dataSource === 'demo' ? '请在 macOS App 中打开文件夹' : undefined"
                  :aria-label="'在 Finder 中打开 ' + file.path"
                  @click.stop="openFolder(file.path)"
                >
                  {{ openingPath === file.path ? "打开中" : "打开文件夹" }}
                </button>
                <b>{{ formatBytes(file.size) }}</b>
              </div>
            </div>
            <div v-else class="detail-empty">这个分类没有可展开的一级项目。</div>
            <button
              v-if="item.files.length > 20"
              type="button"
              class="show-all-button"
              @click="openDetailModal(item.id)"
            >
              查看全部 {{ item.files.length.toLocaleString() }} 项
            </button>
          </div>
        </article>
      </div>
    </div>

    <div v-else-if="!isScanning && !cleanReport" class="empty-state">
      <span>⌕</span>
      <h3>还没有扫描记录</h3>
      <p>开始后会自动归类可清理空间，并把风险最低的项目先选好。</p>
    </div>

    <div
      v-if="detailModalCategory"
      class="detail-modal-backdrop"
      role="presentation"
      @click.self="closeDetailModal"
    >
      <section class="detail-modal" role="dialog" aria-modal="true" :aria-label="`${detailModalCategory.name}详情`">
        <header>
          <div>
            <p class="section-kicker">Category Detail</p>
            <h3>{{ detailModalCategory.name }}</h3>
            <span>{{ detailModalItems.length.toLocaleString() }} 个一级项目 · 按大小降序</span>
          </div>
          <button type="button" aria-label="关闭详情" @click="closeDetailModal">×</button>
        </header>

        <div class="modal-detail-list">
          <div v-for="file in detailModalItems" :key="file.path" class="detail-row">
            <span class="detail-kind">{{ detailGlyph(file) }}</span>
            <span class="detail-path">
              <strong>{{ file.path }}</strong>
              <small>{{ detailKind(file) }}</small>
            </span>
            <button
              type="button"
              class="open-folder-button"
              :disabled="dataSource === 'demo' || openingPath === file.path"
              :title="dataSource === 'demo' ? '请在 macOS App 中打开文件夹' : undefined"
              :aria-label="'在 Finder 中打开 ' + file.path"
              @click.stop="openFolder(file.path)"
            >
              {{ openingPath === file.path ? "打开中" : "打开文件夹" }}
            </button>
            <b>{{ formatBytes(file.size) }}</b>
          </div>
        </div>
      </section>
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

.cleaning-progress {
  display: grid;
  grid-template-columns: auto auto;
  gap: 4px 10px;
  margin-top: 12px;
  padding: 12px 14px;
  border: 1px solid rgba(83, 216, 209, 0.35);
  border-radius: 11px;
  background: rgba(35, 130, 132, 0.2);
  color: #fff;
  font-size: 13px;
}

.cleaning-progress > span {
  justify-self: end;
  color: rgba(235, 248, 255, 0.7);
}

.cleaning-progress-track {
  grid-column: 1 / -1;
  height: 6px;
  margin-top: 5px;
  border-radius: 99px;
  background: rgba(235, 248, 255, 0.18);
  overflow: hidden;
}

.cleaning-progress-track span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: #53d8d1;
  transition: width 220ms ease;
}

.report-top {
  margin-top: 12px;
}

.open-folder-button {
  display: inline-flex;
  align-items: center;
  justify-self: end;
  min-width: 52px;
  padding: 2px 4px;
  border: 0;
  background: transparent;
  color: rgba(218, 244, 255, 0.64);
  font-size: 10px;
  font-weight: 750;
  white-space: nowrap;
  transition: color 150ms ease, opacity 150ms ease;
}

.open-folder-button:hover:not(:disabled),
.open-folder-button:focus-visible {
  color: #fff;
}

.open-folder-button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
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

.selection-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 9px;
  min-height: 36px;
  margin: -4px 0 10px;
}

.select-all-control {
  display: flex;
  align-items: center;
  gap: 9px;
  color: rgba(235, 248, 255, 0.78);
  font-size: 12px;
  font-weight: 800;
  cursor: pointer;
}

.select-all-control input {
  width: 17px;
  height: 17px;
  margin: 0;
  accent-color: #35c8c0;
}

.select-all-control:has(input:disabled) {
  cursor: not-allowed;
  opacity: 0.56;
}

.summary-panel .invert-selection-button {
  width: auto;
  min-height: 30px;
  padding: 0 9px;
  border: 1px solid rgba(235, 248, 255, 0.18);
  background: rgba(235, 248, 255, 0.08);
  box-shadow: none;
  color: rgba(235, 248, 255, 0.78);
  font-size: 11px;
}

.summary-panel .invert-selection-button:hover {
  box-shadow: none;
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

.category-main:disabled {
  cursor: wait;
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
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

.preview-head small {
  color: rgba(235, 248, 255, 0.48);
  font-size: 11px;
  font-weight: 750;
}

.detail-preview {
  padding: 0 16px 14px 56px;
}

.detail-list,
.modal-detail-list {
  display: grid;
  gap: 7px;
}

.detail-row {
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr) auto minmax(58px, auto);
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 9px;
  background: rgba(235, 248, 255, 0.1);
  color: rgba(235, 248, 255, 0.82);
  font-size: 12px;
}

.detail-kind {
  display: grid;
  place-items: center;
  color: #6ce6dd;
  font-size: 13px;
}

.detail-path {
  min-width: 0;
}

.detail-path strong {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: rgba(255, 255, 255, 0.92);
  font-size: 12px;
}

.detail-path small {
  display: block;
  margin-top: 2px;
  color: rgba(235, 248, 255, 0.48);
  font-size: 11px;
}

.detail-row b {
  white-space: nowrap;
  color: #fff;
  font-size: 12px;
}

.detail-empty {
  padding: 10px 12px;
  border-radius: 9px;
  background: rgba(235, 248, 255, 0.08);
  color: rgba(235, 248, 255, 0.62);
  font-size: 12px;
}

.show-all-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 34px;
  margin-top: 10px;
  padding: 0 12px;
  border: 1px solid rgba(238, 249, 255, 0.16);
  border-radius: 9px;
  background: rgba(235, 248, 255, 0.12);
  color: #fff;
  font-size: 12px;
  font-weight: 850;
}

.detail-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  padding: 30px;
  background: rgba(13, 31, 51, 0.55);
  backdrop-filter: blur(10px);
}

.detail-modal {
  width: min(760px, 100%);
  max-height: min(680px, calc(100vh - 60px));
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  border: 1px solid rgba(238, 249, 255, 0.18);
  border-radius: 18px;
  background:
    linear-gradient(180deg, rgba(59, 123, 157, 0.98), rgba(36, 77, 112, 0.98)),
    #2d668f;
  box-shadow: 0 28px 80px rgba(12, 28, 58, 0.34);
  overflow: hidden;
}

.detail-modal header {
  display: flex;
  justify-content: space-between;
  gap: 18px;
  padding: 20px 22px 16px;
  border-bottom: 1px solid rgba(238, 249, 255, 0.12);
}

.detail-modal h3 {
  margin: 0;
  color: #fff;
  font-size: 20px;
}

.detail-modal header span {
  display: block;
  margin-top: 5px;
  color: rgba(235, 248, 255, 0.62);
  font-size: 12px;
  font-weight: 750;
}

.detail-modal header button {
  width: 34px;
  height: 34px;
  border: 0;
  border-radius: 10px;
  background: rgba(235, 248, 255, 0.12);
  color: #fff;
  font-size: 24px;
  line-height: 1;
}

.modal-detail-list {
  overflow: auto;
  padding: 16px 22px 22px;
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
