import { invoke } from "@tauri-apps/api/core";

export type RiskLevel = "low" | "medium" | "high";

export interface FileInfo {
  path: string;
  size: number;
  modified_at?: string;
}

export interface CategoryResult {
  id: string;
  name: string;
  description: string;
  risk: RiskLevel;
  file_count: number;
  total_size: number;
  files: FileInfo[];
}

export interface ScanResult {
  total_size: number;
  categories: CategoryResult[];
  scan_duration_ms: number;
}

export interface CleanReport {
  cleaned_count: number;
  freed_bytes: number;
  skipped_count: number;
  errors: Array<{ path: string; reason: string }>;
  snapshot_id: string;
}

export interface InstalledApp {
  name: string;
  bundle_id: string;
  app_path: string;
  icon_path?: string;
  icon_data_url?: string;
  app_size: number;
  related_size: number;
  related_count: number;
  related_files: FileInfo[];
  is_system_app: boolean;
}

export interface DiskInfo {
  volume_name: string;
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  usage_percent: number;
}

export async function invokeOrDemo<T>(
  command: string,
  fallback: T,
  args?: Record<string, unknown>,
  timeoutMs = 0,
): Promise<{ data: T; source: "native" | "demo"; error?: string }> {
  try {
    const nativeCall = invoke<T>(command, args);
    const data = timeoutMs > 0
      ? await Promise.race([
          nativeCall,
          new Promise<T>((_, reject) => {
            window.setTimeout(() => reject(new Error(`${command} timed out`)), timeoutMs);
          }),
        ])
      : await nativeCall;
    const isEmptyArray = Array.isArray(data) && data.length === 0;
    const scanData = data as unknown as Partial<ScanResult>;
    const isEmptyScan =
      command === "scan_system" &&
      Array.isArray(scanData.categories) &&
      scanData.categories.length === 0;

    if (isEmptyArray || isEmptyScan) {
      return { data: fallback, source: "demo" };
    }

    return { data, source: "native" };
  } catch (error) {
    return {
      data: fallback,
      source: "demo",
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** index;
  const precision = index >= 3 ? 2 : index === 0 ? 0 : 1;
  return `${value.toFixed(precision)} ${units[index]}`;
}

export const demoDiskInfo: DiskInfo = {
  volume_name: "Macintosh HD",
  total_bytes: 500_277_790_720,
  available_bytes: 86_430_826_496,
  used_bytes: 413_846_964_224,
  usage_percent: 82.7,
};

export const demoScanResult: ScanResult = {
  total_size: 18_098_675_712,
  scan_duration_ms: 4200,
  categories: [
    {
      id: "xcode_derived",
      name: "Xcode 构建缓存",
      description: "DerivedData、Archives 和模拟器缓存，可安全重建。",
      risk: "low",
      file_count: 18842,
      total_size: 8_724_152_320,
      files: [
        { path: "~/Library/Developer/Xcode/DerivedData/CleanMacProAI-bkpq", size: 2_104_983_552 },
        { path: "~/Library/Developer/Xcode/iOS DeviceSupport/17.4", size: 1_719_582_720 },
        { path: "~/Library/Developer/CoreSimulator/Caches/dyld", size: 842_006_528 },
      ],
    },
    {
      id: "browser_cache",
      name: "浏览器缓存",
      description: "Chrome、Safari、Edge 的网页缓存，删除后网页会按需重新下载。",
      risk: "low",
      file_count: 23421,
      total_size: 4_486_578_176,
      files: [
        { path: "~/Library/Caches/Google/Chrome/Default/Cache/Cache_Data", size: 1_633_779_712 },
        { path: "~/Library/Caches/com.apple.Safari/fsCachedData", size: 964_689_920 },
        { path: "~/Library/Caches/Microsoft Edge/Default/Code Cache/js", size: 517_996_544 },
      ],
    },
    {
      id: "system_cache",
      name: "系统和应用缓存",
      description: "常规缓存文件，不包含偏好设置、钥匙串和用户文档。",
      risk: "low",
      file_count: 12970,
      total_size: 2_834_546_688,
      files: [
        { path: "~/Library/Caches/com.apple.helpd", size: 438_304_768 },
        { path: "~/Library/Caches/com.figma.Desktop", size: 388_923_392 },
        { path: "~/Library/Caches/com.spotify.client", size: 327_155_712 },
      ],
    },
    {
      id: "downloads_leftover",
      name: "下载残留安装包",
      description: "下载目录里的 DMG、PKG、ZIP，建议确认不再需要后清理。",
      risk: "medium",
      file_count: 47,
      total_size: 1_624_178_688,
      files: [
        { path: "~/Downloads/Figma-124.7.dmg", size: 156_237_824 },
        { path: "~/Downloads/Xcode_15.4.xip", size: 1_020_813_312 },
        { path: "~/Downloads/Node-22.1.0.pkg", size: 87_359_488 },
      ],
    },
    {
      id: "mail_attachments",
      name: "邮件附件缓存",
      description: "本地下载过的 Mail 附件缓存，需要重新从服务器下载。",
      risk: "high",
      file_count: 214,
      total_size: 429_219_840,
      files: [
        { path: "~/Library/Mail/V10/MailData/Downloads/report-final.pdf", size: 46_137_344 },
        { path: "~/Library/Mail/V10/MailData/Downloads/assets.zip", size: 128_974_848 },
      ],
    },
  ],
};

export const demoApps: InstalledApp[] = [
  {
    name: "Xcode",
    bundle_id: "com.apple.dt.Xcode",
    app_path: "/Applications/Xcode.app",
    app_size: 12_884_901_888,
    related_size: 45_097_156_608,
    related_count: 3421,
    related_files: [
      { path: "~/Library/Developer/Xcode/DerivedData", size: 34_522_513_408 },
      { path: "~/Library/Caches/com.apple.dt.Xcode", size: 6_891_012_096 },
      { path: "~/Library/Preferences/com.apple.dt.Xcode.plist", size: 286_720 },
    ],
    is_system_app: false,
  },
  {
    name: "Google Chrome",
    bundle_id: "com.google.Chrome",
    app_path: "/Applications/Google Chrome.app",
    app_size: 714_572_800,
    related_size: 4_294_967_296,
    related_count: 15234,
    related_files: [
      { path: "~/Library/Caches/Google/Chrome", size: 3_214_180_352 },
      { path: "~/Library/Application Support/Google/Chrome", size: 1_080_524_800 },
      { path: "~/Library/Preferences/com.google.Chrome.plist", size: 180_224 },
    ],
    is_system_app: false,
  },
  {
    name: "Visual Studio Code",
    bundle_id: "com.microsoft.VSCode",
    app_path: "/Applications/Visual Studio Code.app",
    app_size: 429_496_729,
    related_size: 2_147_483_648,
    related_count: 892,
    related_files: [
      { path: "~/Library/Application Support/Code", size: 1_711_046_656 },
      { path: "~/Library/Caches/com.microsoft.VSCode", size: 436_142_080 },
      { path: "~/Library/Preferences/com.microsoft.VSCode.plist", size: 294_912 },
    ],
    is_system_app: false,
  },
  {
    name: "Slack",
    bundle_id: "com.tinyspeck.slackmacgap",
    app_path: "/Applications/Slack.app",
    app_size: 618_659_840,
    related_size: 1_319_411_712,
    related_count: 3712,
    related_files: [
      { path: "~/Library/Application Support/Slack", size: 916_455_424 },
      { path: "~/Library/Caches/com.tinyspeck.slackmacgap", size: 402_751_488 },
      { path: "~/Library/Preferences/com.tinyspeck.slackmacgap.plist", size: 204_800 },
    ],
    is_system_app: false,
  },
];

export const demoCleanReport: CleanReport = {
  cleaned_count: 55280,
  freed_bytes: 17_669_455_872,
  skipped_count: 3,
  errors: [],
  snapshot_id: "demo-snapshot-20260517",
};
