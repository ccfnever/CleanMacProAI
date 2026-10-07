import { defineStore } from "pinia";
import { ref } from "vue";
import { demoDiskInfo, invokeOrDemo, type DiskInfo } from "../lib/demoData";

export const useDiskStore = defineStore("disk", () => {
  const diskInfo = ref<DiskInfo>(demoDiskInfo);
  const dataSource = ref<"native" | "demo">("demo");
  const diskNotice = ref<string | null>(null);

  async function refreshDiskInfo() {
    const result = await invokeOrDemo<DiskInfo>("get_disk_info", demoDiskInfo);
    if (result.source === "error") {
      diskNotice.value = `无法更新磁盘信息：${result.error}`;
      return;
    }
    diskInfo.value = result.data;
    dataSource.value = result.source === "native" ? "native" : "demo";
    diskNotice.value = null;
  }

  return { diskInfo, dataSource, diskNotice, refreshDiskInfo };
});
