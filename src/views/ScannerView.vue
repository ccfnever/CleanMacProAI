<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import AppIcon from "../components/AppIcon.vue";
import { formatBytes, type CategoryResult, type FileInfo } from "../lib/demoData";
import { scanPhases, useScannerStore } from "../stores/scanner";

type RiskFilter = "all" | "low" | "medium" | "high";

const scannerStore = useScannerStore();
const {
  activePhase,
  cleanPhase,
  cleanProgress,
  cleanReport,
  dataSource,
  deletionMode,
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

const cleanDialog = ref<HTMLDialogElement | null>(null);
const permanentAcknowledged = ref(false);
function requestClean() {
  permanentAcknowledged.value = false;
  cleanDialog.value?.showModal();
}
async function confirmClean() {
  if (deletionMode.value === "permanent" && !permanentAcknowledged.value) return;
  cleanDialog.value?.close();
  await cleanSelected(permanentAcknowledged.value);
}
const activeFilter = ref<RiskFilter>("all");
const query = ref("");
const openingPath = ref<string | null>(null);
const showAllCategoryIds = ref<Set<string>>(new Set());

const filterOptions = computed(() => [
  { id: "all" as const, label: "全部", count: scanResults.value.length },
  { id: "low" as const, label: "可放心清理", count: safeCount.value },
  { id: "medium" as const, label: "建议确认", count: mediumCount.value },
  { id: "high" as const, label: "已锁定", count: highCount.value },
]);

const filteredResults = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  return scanResults.value.filter((item) => {
    if (activeFilter.value !== "all" && item.risk !== activeFilter.value) return false;
    if (!keyword) return true;
    return [item.name, item.description, ...item.files.map((file) => file.path)]
      .some((value) => value.toLowerCase().includes(keyword));
  });
});

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

function categoryTone(item: CategoryResult) {
  if (item.risk === "medium") return "tone-sand";
  if (item.risk === "high") return "tone-rose";
  return item.id.includes("browser") ? "tone-blue" : "tone-map";
}

function categoryIcon(item: CategoryResult) {
  if (item.risk === "high") return "shield";
  if (item.id.includes("browser")) return "search";
  if (item.id.includes("xcode")) return "apps";
  return "folder";
}

function detailItems(item: CategoryResult): FileInfo[] {
  return [...item.files].sort((left, right) => right.size - left.size);
}

function visibleDetailItems(item: CategoryResult): FileInfo[] {
  const files = detailItems(item);
  return showAllCategoryIds.value.has(item.id) ? files : files.slice(0, 12);
}

function detailKind(item: FileInfo) {
  return item.is_dir ? "文件夹" : "文件";
}

function toggleShowAll(categoryId: string) {
  const next = new Set(showAllCategoryIds.value);
  if (next.has(categoryId)) next.delete(categoryId);
  else next.add(categoryId);
  showAllCategoryIds.value = next;
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
    <section v-if="isScanning" class="scan-stage" aria-live="polite">
      <div class="scan-emblem">
        <AppIcon name="scan" :size="34" />
        <span class="spinner"></span>
      </div>
      <h1>{{ activePhase }}</h1>
      <div
        class="scan-meter"
        role="progressbar"
        aria-label="扫描进度"
        :aria-valuenow="scanProgress"
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <span :style="{ width: `${scanProgress}%` }"></span>
      </div>
      <p>{{ scanProgress.toFixed(0) }}% · 扫描只分析文件，不会自动清理</p>
      <div class="phase-row">
        <span v-for="phase in scanPhases" :key="phase">{{ phase }}</span>
      </div>
    </section>

    <template v-else-if="scanResults.length">
      <header class="content-heading">
        <div>
          <p class="section-kicker">扫描结果</p>
          <h1>这些项目正在占用空间</h1>
          <p>逐项查看来源和风险，再决定要清理什么。</p>
        </div>
        <button type="button" class="secondary-action" @click="startScan">
          <AppIcon name="refresh" :size="15" />
          重新扫描
        </button>
      </header>

      <div v-if="cleanReport" class="report-panel">
        <span class="report-icon"><AppIcon name="check" :size="18" /></span>
        <div>
          <strong>{{ cleanReport.errors.length ? "清理部分完成" : "清理完成" }}</strong>
          <p>
            已处理 {{ formatBytes(cleanReport.processed_bytes ?? cleanReport.freed_bytes) }}，处理
            {{ cleanReport.cleaned_count.toLocaleString() }} 个项目，跳过 {{ cleanReport.skipped_count }} 个。
            {{ cleanReport.deletion_mode === 'permanent' ? '项目已永久删除，无法从废纸篓恢复。' : '项目已移入废纸篓；清空废纸篓后才会释放空间。' }}
          </p>
          <details v-if="cleanReport.errors.length">
            <summary>查看未处理项目</summary>
            <ul><li v-for="(error, index) in cleanReport.errors" :key="index">{{ error.path }}：{{ error.reason }}</li></ul>
          </details>
        </div>
      </div>

      <p v-if="notice" class="notice" role="status">{{ notice }}</p>

      <section class="scan-summary">
        <div class="summary-copy">
          <span class="icon-tile tone-map"><AppIcon name="check" :size="21" /></span>
          <div>
            <strong>{{ scanResults.length }} 项待检查</strong>
            <span>低风险项目已默认选中，其余项目由你确认</span>
          </div>
        </div>
        <div class="summary-amount">
          <strong>{{ formatBytes(totalCleanable) }}</strong>
          <small>{{ totalFileCount.toLocaleString() }} 个文件 · 发现的文件大小</small>
        </div>
      </section>

      <div v-if="isCleaning" class="cleaning-progress" role="status" aria-live="polite">
        <span>{{ cleanPhase }}</span>
        <strong>{{ cleanProgress }}%</strong>
        <div><i :style="{ width: `${cleanProgress}%` }"></i></div>
      </div>

      <div class="toolbar">
        <div class="filter-tabs" aria-label="按风险筛选">
          <button
            v-for="item in filterOptions"
            :key="item.id"
            type="button"
            :class="{ active: activeFilter === item.id }"
            @click="activeFilter = item.id"
          >
            {{ item.label }} <span>{{ item.count }}</span>
          </button>
        </div>
        <label class="search-box">
          <AppIcon name="search" :size="15" />
          <input v-model="query" type="search" placeholder="搜索项目或路径" />
        </label>
      </div>

      <section class="table-shell" aria-label="可清理项目">
        <div class="table-head" aria-hidden="true">
          <span>选择</span><span>项目</span><span>大小</span><span>建议</span><span></span>
        </div>

        <article
          v-for="item in filteredResults"
          :key="item.id"
          :class="['category-row', { selected: selectedCategories.has(item.id), expanded: expandedCategory === item.id }]"
        >
          <div class="category-grid">
            <input
              type="checkbox"
              :checked="selectedCategories.has(item.id)"
              :disabled="isCleaning || item.risk === 'high'"
              :aria-label="`选择${item.name}`"
              @change="toggleCategory(item.id, item.risk)"
            />

            <button type="button" class="item-identity" @click="toggleExpanded(item.id)">
              <span :class="['icon-tile', categoryTone(item)]">
                <AppIcon :name="categoryIcon(item)" :size="20" />
              </span>
              <span>
                <strong>{{ item.name }}</strong>
                <small :title="item.files[0]?.path || item.description">
                  {{ item.files[0]?.path || item.description }}
                </small>
              </span>
            </button>

            <strong class="item-size">{{ formatBytes(item.total_size) }}</strong>
            <span :class="['risk', item.risk]">
              <AppIcon :name="item.risk === 'low' ? 'check' : 'shield'" :size="12" />
              {{ riskLabel(item.risk) }}
            </span>
            <button
              type="button"
              class="expand-button"
              :aria-label="`${expandedCategory === item.id ? '收起' : '展开'}${item.name}详情`"
              :aria-expanded="expandedCategory === item.id"
              @click="toggleExpanded(item.id)"
            >
              <AppIcon name="chevron" :size="16" />
            </button>
          </div>

          <div v-if="expandedCategory === item.id" class="inline-detail">
            <header class="detail-summary">
              <div>
                <strong>包含内容</strong>
                <p>{{ item.description }}</p>
              </div>
              <div class="detail-facts">
                <span>{{ item.files.length.toLocaleString() }} 个一级项目</span>
                <span>{{ item.file_count.toLocaleString() }} 个文件</span>
                <span>{{ formatBytes(item.total_size) }}</span>
              </div>
            </header>

            <div v-if="item.files.length" class="detail-items">
              <div v-for="file in visibleDetailItems(item)" :key="file.path" class="detail-item">
                <span class="detail-file-icon"><AppIcon name="folder" :size="16" /></span>
                <span class="detail-file-copy">
                  <strong>{{ file.path.split('/').pop() || file.path }}</strong>
                  <small :title="file.path">{{ file.path }}</small>
                </span>
                <span class="detail-kind">{{ detailKind(file) }}</span>
                <button
                  type="button"
                  class="finder-button"
                  :disabled="dataSource === 'demo' || openingPath === file.path"
                  :title="dataSource === 'demo' ? '请在 macOS App 中打开文件夹' : undefined"
                  @click.stop="openFolder(file.path)"
                >
                  <AppIcon name="folder" :size="13" />
                  <span>{{ openingPath === file.path ? "打开中" : "打开位置" }}</span>
                </button>
                <b>{{ formatBytes(file.size) }}</b>
              </div>
            </div>
            <p v-else class="detail-empty">这个分类没有可展开的一级项目。</p>
            <button
              v-if="item.files.length > 12"
              type="button"
              class="show-all-button"
              @click="toggleShowAll(item.id)"
            >
              {{ showAllCategoryIds.has(item.id) ? "收起更多" : `继续展开 ${item.files.length.toLocaleString()} 项` }}
            </button>
          </div>
        </article>

        <div v-if="filteredResults.length === 0" class="empty compact">
          <AppIcon name="search" :size="24" />
          <p>没有匹配的项目</p>
        </div>
      </section>

      <footer class="selection-bar">
        <div class="selection-info">
          <label>
            <input
              type="checkbox"
              :checked="isAllSelectableSelected"
              :indeterminate="isSelectionPartial"
              :disabled="isCleaning"
              @change="toggleAllCategories"
            />
            <span>{{ isAllSelectableSelected ? "取消全选" : "全选可清理项" }}</span>
          </label>
          <button type="button" :disabled="isCleaning" @click="invertCategorySelection">反选</button>
          <div>
            <strong>{{ formatBytes(selectedTotal) }}</strong>
            <small>{{ selectedSummary }}</small>
          </div>
        </div>
        <div class="selection-actions">
          <label>处理方式
            <select v-model="deletionMode" :disabled="isCleaning">
              <option value="trash">移入废纸篓（默认）</option>
              <option value="permanent">永久删除</option>
            </select>
          </label>
          <button
            type="button"
            class="primary-action"
            :disabled="selectedCategories.size === 0 || isCleaning || dataSource === 'demo'"
            :title="dataSource === 'demo' ? '演示模式不会执行清理' : undefined"
            @click="requestClean"
          >
            <AppIcon name="trash" :size="16" />
            {{ isCleaning ? "清理中" : "清理已选项目" }}
          </button>
        </div>
      </footer>
    </template>

    <section v-else class="empty-state">
      <span class="icon-tile tone-map"><AppIcon name="scan" :size="30" /></span>
      <h1>{{ cleanReport ? "本次清理已完成" : "准备好检查一下了吗？" }}</h1>
      <p>{{ cleanReport ? "可以重新扫描，确认现在还剩下哪些可处理内容。" : "扫描不会修改文件，也不会自动执行清理。" }}</p>
      <button type="button" class="primary-action" @click="startScan">
        <AppIcon :name="cleanReport ? 'refresh' : 'scan'" :size="17" />
        {{ cleanReport ? "重新扫描" : "开始扫描" }}
      </button>
    </section>
    <dialog ref="cleanDialog" class="clean-dialog" aria-labelledby="clean-dialog-title">
      <h2 id="clean-dialog-title">{{ deletionMode === 'permanent' ? '确认永久删除已选项目？' : '将已选项目移入废纸篓？' }}</h2>
      <p>{{ selectedSummary }} · 数据大小 {{ formatBytes(selectedTotal) }}</p>
      <p>{{ deletionMode === 'permanent' ? '文件将直接删除，无法从废纸篓恢复。' : '可以在废纸篓仍保留文件时恢复；移入废纸篓不会立即释放空间。' }}</p>
      <p v-if="deletionMode === 'trash' && selectedCategories.has('trash')">废纸篓内容会跳过；清空废纸篓需要选择永久删除。</p>
      <label v-if="deletionMode === 'permanent'"><input v-model="permanentAcknowledged" type="checkbox" /> 我理解永久删除无法从废纸篓恢复</label>
      <div class="clean-dialog-actions">
        <button type="button" class="secondary-action" @click="cleanDialog?.close()">取消</button>
        <button type="button" class="primary-action" :disabled="deletionMode === 'permanent' && !permanentAcknowledged" @click="confirmClean">{{ deletionMode === 'permanent' ? '确认永久删除' : '确认移入废纸篓' }}</button>
      </div>
    </dialog>
  </section>
</template>

<style scoped>
.clean-dialog { width: min(480px, calc(100vw - 40px)); padding: 24px; border: 1px solid var(--border); border-radius: 16px; color: var(--text); background: var(--surface); }
.clean-dialog::backdrop { background: rgba(40,32,27,.38); }
.clean-dialog p { font-size: 13px; line-height: 1.7; }
.clean-dialog label { display: flex; align-items: center; gap: 8px; font-size: 13px; }
.clean-dialog-actions { display: flex; justify-content: flex-end; gap: 12px; margin-top: 24px; }
.selection-actions select { padding: 6px; border: 1px solid var(--border); border-radius: 6px; background: var(--surface); color: var(--text); }

.scanner-page {
  max-width: 1240px;
  margin: 20px auto 0;
  color: var(--text);
}

.content-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  margin-bottom: 22px;
}

.section-kicker {
  margin: 0 0 5px;
  color: var(--accent);
  font-size: 10px;
  font-weight: 600;
  letter-spacing: .06em;
}

.content-heading h1,
.scan-stage h1,
.empty-state h1 {
  margin: 0;
  color: var(--text);
  font-size: 27px;
  font-weight: 600;
  line-height: 1.3;
  letter-spacing: -.6px;
}

.content-heading > div > p:last-child {
  margin: 7px 0 0;
  color: var(--text-soft);
  font-size: 12px;
}

.secondary-action,
.primary-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border-radius: 9px;
  font-weight: 550;
  transition: filter 160ms ease, transform 160ms ease;
}

.secondary-action {
  min-height: 38px;
  padding: 0 14px;
  border: 1px solid var(--border);
  color: var(--text);
  background: var(--surface);
  font-size: 12px;
}

.primary-action {
  min-height: 44px;
  padding: 0 18px;
  border: 0;
  color: #fff;
  background: var(--accent);
  font-size: 12px;
}

.secondary-action:hover:not(:disabled),
.primary-action:hover:not(:disabled) { filter: brightness(.96); transform: translateY(-1px); }
.secondary-action:active:not(:disabled),
.primary-action:active:not(:disabled) { transform: translateY(0); }

.scan-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: 12px 16px;
  border: 1px solid var(--border);
  border-radius: 13px;
  background: var(--surface);
}

.summary-copy { display: flex; align-items: center; gap: 13px; }
.summary-copy strong { display: block; font-size: 16px; font-weight: 550; }
.summary-copy > div > span { display: block; margin-top: 4px; color: var(--text-soft); font-size: 11px; }
.summary-amount { text-align: right; }
.summary-amount strong { display: block; font-size: 24px; font-weight: 550; font-variant-numeric: tabular-nums; }
.summary-amount small { display: block; margin-top: 2px; color: var(--text-soft); font-size: 10px; }

.icon-tile {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  width: 42px;
  height: 42px;
  border-radius: 12px;
  color: var(--accent);
  background: var(--accent-soft);
}
.tone-map { color: #4c785d; background: #e8f0e5; }
.tone-blue { color: #456ba2; background: #e8eef8; }
.tone-sand { color: #9a6a2d; background: #f5ead9; }
.tone-rose { color: #a04d48; background: #f6e7e3; }

.notice,
.report-panel,
.cleaning-progress {
  margin: 12px 0 0;
  border-radius: 9px;
  font-size: 11px;
}
.notice { padding: 10px 13px; border: 1px solid color-mix(in srgb, var(--warning) 24%, transparent); color: var(--text-soft); background: color-mix(in srgb, var(--warning) 8%, var(--surface)); }
.report-panel { display: flex; align-items: flex-start; gap: 10px; padding: 12px 14px; border: 1px solid color-mix(in srgb, var(--success) 24%, transparent); background: color-mix(in srgb, var(--success) 8%, var(--surface)); }
.report-icon { display: grid; place-items: center; flex: 0 0 28px; width: 28px; height: 28px; border-radius: 8px; color: var(--success); background: color-mix(in srgb, var(--success) 13%, transparent); }
.report-panel strong { font-weight: 600; }
.report-panel p { margin: 3px 0 0; color: var(--text-soft); line-height: 1.6; }
.cleaning-progress { display: grid; grid-template-columns: 1fr auto; gap: 7px 12px; padding: 11px 13px; border: 1px solid color-mix(in srgb, var(--success) 24%, transparent); background: var(--surface); }
.cleaning-progress > span { color: var(--text-soft); }
.cleaning-progress > div { grid-column: 1 / -1; height: 5px; overflow: hidden; border-radius: 99px; background: var(--surface-strong); }
.cleaning-progress i { display: block; height: 100%; border-radius: inherit; background: var(--success); transition: width 220ms ease; }

.toolbar { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin: 12px 0; }
.filter-tabs { display: flex; gap: 4px; padding: 3px; border-radius: 8px; background: var(--sidebar-bg); }
.filter-tabs button { display: inline-flex; align-items: center; gap: 7px; min-height: 32px; padding: 0 12px; border: 0; border-radius: 6px; color: var(--text-soft); background: transparent; font-size: 11px; }
.filter-tabs button span { color: var(--text-faint); font-size: 9px; font-variant-numeric: tabular-nums; }
.filter-tabs button.active { color: var(--text); background: var(--surface); box-shadow: 0 1px 4px var(--shadow-soft); }
.search-box { position: relative; display: flex; align-items: center; }
.search-box > svg { position: absolute; left: 11px; color: var(--text-faint); pointer-events: none; }
.search-box input { width: 195px; min-height: 36px; padding: 8px 10px 8px 34px; border: 1px solid var(--border); border-radius: 8px; outline: 0; color: var(--text); background: var(--surface); font-size: 11px; }
.search-box input::placeholder { color: var(--text-faint); }

.table-shell { overflow: hidden; border: 1px solid var(--border); border-radius: 13px; background: var(--surface); box-shadow: var(--shadow-soft); }
.table-head,
.category-grid { display: grid; grid-template-columns: 44px minmax(0, 1fr) 106px 112px 42px; align-items: center; }
.table-head { min-height: 39px; color: var(--text-faint); background: color-mix(in srgb, var(--surface-soft) 55%, var(--surface)); font-size: 10px; }
.table-head span { padding: 0 10px; }
.category-row { border-top: 1px solid var(--border); }
.category-row:first-of-type { border-top: 0; }
.category-grid { min-height: 64px; transition: background 160ms ease; }
.category-row.selected .category-grid { background: color-mix(in srgb, var(--accent-soft) 18%, transparent); }
.category-row.expanded .category-grid { background: color-mix(in srgb, var(--accent-soft) 28%, var(--surface)); }
.category-grid > input { justify-self: center; width: 16px; height: 16px; margin: 0; accent-color: var(--accent); }
.item-identity { display: flex; align-items: center; gap: 12px; min-width: 0; padding: 8px 10px; border: 0; color: var(--text); background: transparent; text-align: left; }
.item-identity > span:last-child { min-width: 0; }
.item-identity strong { display: block; font-size: 13px; font-weight: 550; }
.item-identity small { display: block; max-width: 460px; margin-top: 4px; overflow: hidden; color: var(--text-soft); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.item-size { padding: 0 10px; font-size: 13px; font-weight: 550; font-variant-numeric: tabular-nums; white-space: nowrap; }
.risk { display: inline-flex; align-items: center; justify-self: start; gap: 4px; padding: 4px 7px; border-radius: 5px; font-size: 10px; font-weight: 500; white-space: nowrap; }
.risk.low { color: #2c6245; background: #e8f1e7; }
.risk.medium { color: #82500f; background: #fbefda; }
.risk.high { color: #a03b38; background: #f9e8e5; }
.expand-button { display: grid; place-items: center; width: 34px; height: 34px; padding: 0; border: 0; border-radius: 8px; color: var(--text-soft); background: transparent; }
.expand-button:hover { background: var(--surface-soft); }
.expand-button svg { transition: transform 180ms ease; }
.category-row.expanded .expand-button svg { transform: rotate(90deg); }

.inline-detail { margin: 0 10px 12px 56px; overflow: hidden; border: 1px solid var(--border); border-radius: 12px; background: var(--surface); }
.detail-summary { display: flex; align-items: flex-start; justify-content: space-between; gap: 24px; padding: 15px 18px; background: color-mix(in srgb, var(--accent-soft) 22%, var(--surface)); }
.detail-summary strong { font-size: 11px; font-weight: 600; }
.detail-summary p { margin: 5px 0 0; color: var(--text-soft); font-size: 10px; line-height: 1.6; }
.detail-facts { display: flex; justify-content: flex-end; gap: 12px; flex-wrap: wrap; color: var(--text-soft); font-size: 10px; white-space: nowrap; }
.detail-items { padding: 0 18px; }
.detail-item { display: grid; grid-template-columns: 24px minmax(0, 1fr) 70px 82px 72px; align-items: center; gap: 10px; min-height: 55px; border-top: 1px solid var(--border); font-size: 10px; }
.detail-item:first-child { border-top: 0; }
.detail-file-icon { color: var(--accent); }
.detail-file-copy { min-width: 0; }
.detail-file-copy strong { display: block; overflow: hidden; font-size: 11px; font-weight: 550; text-overflow: ellipsis; white-space: nowrap; }
.detail-file-copy small { display: block; margin-top: 2px; overflow: hidden; color: var(--text-soft); text-overflow: ellipsis; white-space: nowrap; }
.detail-kind { color: var(--text-soft); }
.finder-button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; min-height: 30px; padding: 0 8px; border: 1px solid var(--border); border-radius: 7px; color: var(--text-soft); background: transparent; font-size: 10px; }
.detail-item > b { text-align: right; font-size: 11px; font-weight: 550; font-variant-numeric: tabular-nums; }
.detail-empty { margin: 0; padding: 16px 18px; color: var(--text-soft); font-size: 10px; }
.show-all-button { width: calc(100% - 36px); min-height: 32px; margin: 0 18px 12px; border: 1px solid var(--border); border-radius: 8px; color: var(--accent); background: var(--surface-soft); font-size: 10px; }

.empty.compact { display: grid; place-items: center; gap: 8px; padding: 38px; color: var(--text-faint); }
.empty.compact p { margin: 0; font-size: 11px; }

.selection-bar { position: sticky; z-index: 12; bottom: -44px; display: flex; align-items: center; justify-content: space-between; gap: 16px; margin: 22px -32px -44px; padding: 15px 32px; border-top: 1px solid var(--border); background: color-mix(in srgb, var(--surface) 94%, transparent); box-shadow: 0 -4px 15px color-mix(in srgb, var(--shadow-soft) 45%, transparent); backdrop-filter: blur(16px); }
.selection-info { display: flex; align-items: center; gap: 12px; }
.selection-info label { display: flex; align-items: center; gap: 7px; color: var(--text-soft); font-size: 10px; cursor: pointer; }
.selection-info label input { width: 16px; height: 16px; margin: 0; accent-color: var(--accent); }
.selection-info > button { padding: 6px; border: 0; color: var(--accent); background: transparent; font-size: 10px; }
.selection-info > div { padding-left: 12px; border-left: 1px solid var(--border); }
.selection-info strong { display: block; font-size: 19px; font-weight: 550; font-variant-numeric: tabular-nums; }
.selection-info small { display: block; margin-top: 2px; color: var(--text-soft); font-size: 9px; }
.selection-actions { display: flex; align-items: center; gap: 13px; }
.selection-actions > span { display: flex; align-items: center; gap: 5px; color: var(--text-soft); font-size: 9px; }

.scan-stage { max-width: 430px; margin: 72px auto; text-align: center; }
.scan-emblem { position: relative; display: grid; place-items: center; width: 94px; height: 94px; margin: auto; color: var(--accent); }
.spinner { position: absolute; inset: 0; border: 3px solid var(--accent-soft); border-top-color: var(--accent); border-radius: 50%; animation: spin 1.2s linear infinite; }
.scan-stage h1 { margin-top: 28px; font-size: 20px; }
.scan-meter { height: 6px; margin: 20px 0 0; overflow: hidden; border-radius: 99px; background: var(--surface-strong); }
.scan-meter span { display: block; height: 100%; border-radius: inherit; background: var(--accent); transition: width 260ms ease; }
.scan-stage > p { margin: 12px 0; color: var(--text-soft); font-size: 11px; }
.phase-row { display: flex; justify-content: space-between; gap: 8px; margin-top: 16px; color: var(--text-faint); font-size: 9px; }

.empty-state { display: grid; place-items: center; padding: 76px 20px; color: var(--text-soft); text-align: center; }
.empty-state > .icon-tile { width: 64px; height: 64px; border-radius: 18px; }
.empty-state h1 { margin-top: 20px; font-size: 20px; }
.empty-state p { margin: 10px 0 0; font-size: 11px; }
.empty-state .primary-action { margin-top: 22px; }

@keyframes spin { to { transform: rotate(360deg); } }

@media (max-width: 1080px) {
  .table-head,
  .category-grid { grid-template-columns: 42px minmax(0, 1fr) 88px 42px; }
  .table-head span:nth-child(4),
  .risk { display: none; }
  .item-identity small { max-width: 330px; }
  .selection-bar { margin-inline: -22px; padding-inline: 22px; }
}
</style>
