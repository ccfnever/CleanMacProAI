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
  const lastScanDurationMs = ref<number | null>(null);

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

  async function startScan() {
    isScanning.value = true;
    scanProgress.value = 0;
    activePhase.value = scanPhases[0];
    scanResults.value = [];
    selectedCategories.value = new Set();
    expandedCategory.value = null;
    cleanReport.value = null;
    notice.value = null;
    lastScanDurationMs.value = null;

    for (let index = 0; index < scanPhases.length; index += 1) {
      activePhase.value = scanPhases[index];
      scanProgress.value = Math.round((index / scanPhases.length) * 86);
      await new Promise((resolve) => window.setTimeout(resolve, 260));
    }

    const result = await invokeOrDemo<ScanResult>("scan_system", demoScanResult);
    scanResults.value = result.data.categories;
    selectedCategories.value = new Set(
      result.data.categories.filter((category) => category.risk === "low").map((category) => category.id),
    );
    dataSource.value = result.source;
    notice.value = result.source === "demo" ? "当前使用演示扫描数据；在 macOS App 内运行时会调用本机扫描引擎。" : null;
    scanProgress.value = 100;
    lastScanDurationMs.value = result.data.scan_duration_ms;
    activePhase.value = `扫描完成 · ${(result.data.scan_duration_ms / 1000).toFixed(1)} 秒`;
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

  async function cleanSelected() {
    isCleaning.value = true;
    const fallback = {
      ...demoCleanReport,
      cleaned_count: selectedFileCount.value,
      freed_bytes: selectedTotal.value,
    };
    const result = await invokeOrDemo<CleanReport>("clean_categories", fallback, {
      categoryIds: selectedItems.value.map((item) => item.id),
      safeMode: true,
    });
    cleanReport.value = result.data;
    notice.value =
      result.source === "demo"
        ? "清理执行处于演示模式；在 macOS App 内运行可调用本机清理命令。"
        : "已按安全模式处理选中项目。建议重新扫描确认可用空间变化。";
    isCleaning.value = false;
  }

  return {
    activePhase,
    cleanReport,
    dataSource,
    expandedCategory,
    highCount,
    isCleaning,
    isScanning,
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
    toggleExpanded,
    cleanSelected,
    totalCleanable,
    totalFileCount,
  };
});
