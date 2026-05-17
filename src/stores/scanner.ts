import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export const useScannerStore = defineStore("scanner", () => {
  const isScanning = ref(false);
  const progress = ref(0);
  const results = ref<any[]>([]);
  const totalCleanable = ref(0);

  async function startScan() {
    isScanning.value = true;
    progress.value = 0;

    try {
      const result = await invoke<any>("scan_system");
      results.value = result.categories || [];
      totalCleanable.value = result.total_size || 0;
    } catch (e) {
      console.error("Scan failed:", e);
    } finally {
      isScanning.value = false;
    }
  }

  return { isScanning, progress, results, totalCleanable, startScan };
});
