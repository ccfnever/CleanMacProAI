import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  demoCleanReport,
  demoScanResult,
  invokeOrDemo,
  type CategoryResult,
  type CleanReport,
  type ScanResult,
} from "../lib/demoData";

export const scanPhases = ["匹配清理规则", "定位可恢复缓存", "标记风险等级", "整理扫描报告"];

export const useScannerStore = defineStore("scanner", () => {
  const isScanning = ref(false);
  const scanProgress = ref(0);
  const activePhase = ref("等待扫描");
  const scanResults = ref<CategoryResult[]>([]);
  const selectedCategories = ref<Set<string>>(new Set());
  const expandedCategory = ref<string | null>(null);
  const cleanReport = ref<CleanReport | null>(null);
  const dataSource = ref<"native" | "demo">("demo");
  const notice = ref<string | null>(null);
  const isCleaning = ref(false);
  const cleanProgress = ref(0);
  const cleanPhase = ref("等待清理");
  const lastScanDurationMs = ref<number | null>(null);

  const selectableCategoryIds = computed(() =>
    scanResults.value.filter((item) => item.risk !== "high").map((item) => item.id),
  );

  const isAllSelectableSelected = computed(() =>
    selectableCategoryIds.value.length > 0
      && selectableCategoryIds.value.every((id) => selectedCategories.value.has(id)),
  );

  const isSelectionPartial = computed(() =>
    selectedCategories.value.size > 0 && !isAllSelectableSelected.value,
  );

  const selectedItems = computed(() =>
    scanResults.value.filter((item) => selectedCategories.value.has(item.id)),
  );

  const selectedTotal = computed(() =>
    selectedItems.value.reduce((sum, item) => sum + item.total_size, 0),
  );

  const selectedFileCount = computed(() =>
    selectedItems.value.reduce((sum, item) => sum + item.file_count, 0),
  );

  const totalCleanable = computed(() =>
    scanResults.value.reduce((sum, item) => sum + item.total_size, 0),
  );

  const totalFileCount = computed(() =>
    scanResults.value.reduce((sum, item) => sum + item.file_count, 0),
  );

  const safeCount = computed(() => scanResults.value.filter((item) => item.risk === "low").length);
  const mediumCount = computed(() => scanResults.value.filter((item) => item.risk === "medium").length);
  const highCount = computed(() => scanResults.value.filter((item) => item.risk === "high").length);

  function replaceScanResults(result: ScanResult) {
    scanResults.value = result.categories;
    selectedCategories.value = new Set(
      result.categories.filter((category) => category.risk === "low").map((category) => category.id),
    );
    if (
      expandedCategory.value
      && !result.categories.some((category) => category.id === expandedCategory.value)
    ) {
      expandedCategory.value = null;
    }
  }

  async function startScan() {
    isScanning.value = true;
    scanProgress.value = 0;
    activePhase.value = scanPhases[0];
    scanResults.value = [];
    selectedCategories.value = new Set();
    expandedCategory.value = null;
    cleanReport.value = null;
    cleanProgress.value = 0;
    cleanPhase.value = "等待清理";
    notice.value = null;
    lastScanDurationMs.value = null;

    try {
      for (let index = 0; index < scanPhases.length; index += 1) {
        activePhase.value = scanPhases[index];
        scanProgress.value = Math.round((index / scanPhases.length) * 86);
        await new Promise((resolve) => window.setTimeout(resolve, 260));
      }

      const result = await invokeOrDemo<ScanResult>("scan_system", demoScanResult);
      if (result.source === "error") {
        activePhase.value = "扫描失败";
        notice.value = `扫描失败：${result.error}`;
        return;
      }

      dataSource.value = result.source === "demo" ? "demo" : "native";
      replaceScanResults(result.data);
      scanProgress.value = 100;
      lastScanDurationMs.value = result.data.scan_duration_ms;
      if (result.source === "empty") {
        activePhase.value = "扫描完成 · 未发现可清理项目";
        notice.value = "扫描已完成，没有发现可清理项目。";
      } else {
        activePhase.value = `扫描完成 · ${(result.data.scan_duration_ms / 1000).toFixed(1)} 秒`;
        notice.value = result.source === "demo"
          ? "当前使用演示扫描数据；演示模式不会执行清理。"
          : null;
      }
    } finally {
      isScanning.value = false;
    }
  }

  function toggleCategory(id: string, risk: string) {
    if (risk === "high" || isCleaning.value) return;
    const next = new Set(selectedCategories.value);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selectedCategories.value = next;
  }

  function toggleAllCategories() {
    if (isCleaning.value) return;
    selectedCategories.value = isAllSelectableSelected.value
      ? new Set()
      : new Set(selectableCategoryIds.value);
  }

  function invertCategorySelection() {
    if (isCleaning.value) return;
    selectedCategories.value = new Set(
      selectableCategoryIds.value.filter((id) => !selectedCategories.value.has(id)),
    );
  }

  function toggleExpanded(id: string) {
    expandedCategory.value = expandedCategory.value === id ? null : id;
  }

  async function cleanSelected() {
    cleanReport.value = null;
    cleanProgress.value = 0;
    cleanPhase.value = "等待清理";
    if (dataSource.value === "demo") {
      notice.value = "演示模式不会执行清理。请在 macOS App 中完成真实扫描后再试。";
      return;
    }
    const cleanedCategoryIds = new Set(selectedItems.value.map((item) => item.id));
    if (cleanedCategoryIds.size === 0) return;

    isCleaning.value = true;
    cleanProgress.value = 15;
    cleanPhase.value = "正在准备清理";
    const fallback = {
      ...demoCleanReport,
      cleaned_count: selectedFileCount.value,
      freed_bytes: selectedTotal.value,
    };
    try {
      cleanProgress.value = 45;
      cleanPhase.value = "正在移入废纸篓";
      const result = await invokeOrDemo<CleanReport>("clean_categories", fallback, {
        categoryIds: selectedItems.value.map((item) => item.id),
      });
      if (result.source === "error") {
        cleanProgress.value = 0;
        cleanPhase.value = "清理失败";
        notice.value = `清理失败：${result.error}`;
        return;
      }
      if (result.source !== "native") {
        cleanProgress.value = 0;
        cleanPhase.value = "未执行清理";
        notice.value = "未执行清理：当前没有可用的本机清理结果。";
        return;
      }

      cleanProgress.value = 85;
      cleanPhase.value = "正在更新扫描结果";
      cleanReport.value = result.data;
      const refreshedScan = await invokeOrDemo<ScanResult>("scan_system", demoScanResult);
      if (refreshedScan.source === "native" || refreshedScan.source === "empty") {
        replaceScanResults(refreshedScan.data);
        scanProgress.value = 100;
        lastScanDurationMs.value = refreshedScan.data.scan_duration_ms;
        activePhase.value = refreshedScan.data.categories.length
          ? `扫描结果已更新 · ${(refreshedScan.data.scan_duration_ms / 1000).toFixed(1)} 秒`
          : "扫描结果已更新 · 未发现可清理项目";
        notice.value = result.data.errors.length === 0
          ? "清理完成，建议释放空间已按当前扫描结果更新。"
          : `清理部分完成，跳过 ${result.data.skipped_count} 项；建议释放空间已按剩余项目更新。`;
      } else {
        const refreshError = refreshedScan.source === "error"
          ? refreshedScan.error
          : "未取得本机扫描结果";
        if (result.data.errors.length === 0) {
          scanResults.value = scanResults.value.filter((item) => !cleanedCategoryIds.has(item.id));
          selectedCategories.value = new Set();
          if (expandedCategory.value && cleanedCategoryIds.has(expandedCategory.value)) {
            expandedCategory.value = null;
          }
        }
        notice.value = result.data.errors.length === 0
          ? `清理完成，但刷新扫描结果失败：${refreshError}`
          : `清理部分完成，但刷新扫描结果失败：${refreshError}`;
      }
      cleanProgress.value = 100;
      cleanPhase.value = "清理完成";
    } finally {
      isCleaning.value = false;
    }
  }

  return {
    activePhase,
    cleanPhase,
    cleanProgress,
    cleanReport,
    dataSource,
    expandedCategory,
    highCount,
    invertCategorySelection,
    isCleaning,
    isAllSelectableSelected,
    isScanning,
    isSelectionPartial,
    lastScanDurationMs,
    mediumCount,
    notice,
    safeCount,
    scanProgress,
    scanResults,
    selectedCategories,
    selectedFileCount,
    selectedItems,
    selectedTotal,
    startScan,
    toggleCategory,
    toggleAllCategories,
    toggleExpanded,
    cleanSelected,
    totalCleanable,
    totalFileCount,
  };
});
