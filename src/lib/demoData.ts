import { invoke } from "@tauri-apps/api/core";

export type RiskLevel = "low" | "medium" | "high";

export interface FileInfo {
  path: string;
  size: number;
  modified_at?: string;
  is_dir?: boolean;
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
}

export type InvokeResult<T> =
  | { source: "native" | "demo" | "empty"; data: T }
  | { source: "error"; error: string };

export interface InstalledApp {
  name: string;
  bundle_id: string;
  app_path: string;
  last_opened_at?: number | null;
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

export interface SpaceMapEntry {
  children?: SpaceMapEntry[];
  name: string;
  path: string;
  logical_size: number;
  allocated_size: number;
  file_count: number;
  directory_count: number;
  modified_at?: string;
  is_dir: boolean;
  is_package: boolean;
  is_cloud_placeholder: boolean;
}

export interface SpaceMapResult {
  incomplete?: boolean;
  root_path: string;
  display_path: string;
  logical_size: number;
  allocated_size: number;
  file_count: number;
  directory_count: number;
  scanned_file_count: number;
  ignored_file_count: number;
  ignored_logical_size: number;
  minimum_file_size: number;
  entries: SpaceMapEntry[];
  skipped_items: number;
  hard_link_duplicates: number;
  symlink_count: number;
  scan_duration_ms: number;
}

export interface SpaceMapProgress {
  is_scanning: boolean;
  root_path: string;
  display_path: string;
  current_path?: string;
  minimum_file_size: number;
  scanned_file_count: number;
  scanned_directory_count: number;
  matched_file_count: number;
  matched_logical_size: number;
  matched_allocated_size: number;
  ignored_file_count: number;
  ignored_logical_size: number;
  skipped_items: number;
  hard_link_duplicates: number;
  symlink_count: number;
  entries: SpaceMapEntry[];
  elapsed_ms: number;
}

export async function invokeOrDemo<T>(
  command: string,
  fallback: T,
  args?: Record<string, unknown>,
  timeoutMs = 0,
): Promise<InvokeResult<T>> {
  const isNativeRuntime = Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
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
      return isNativeRuntime ? { data, source: "empty" } : { data: fallback, source: "demo" };
    }

    return { data, source: "native" };
  } catch (error) {
    if (!isNativeRuntime) return { data: fallback, source: "demo" };
    return { source: "error", error: error instanceof Error ? error.message : String(error) };
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

export const demoSpaceMapResult: SpaceMapResult = {
  root_path: "/Users/demo",
  display_path: "~",
  logical_size: 194_615_705_600,
  allocated_size: 178_670_639_104,
  file_count: 1_284,
  directory_count: 39_842,
  scanned_file_count: 286_431,
  ignored_file_count: 285_147,
  ignored_logical_size: 24_804_802_560,
  minimum_file_size: 104_857_600,
  skipped_items: 0,
  hard_link_duplicates: 138,
  symlink_count: 492,
  scan_duration_ms: 2840,
  entries: [
    { name: "Library", path: "/Users/demo/Library", logical_size: 82_560_573_440, allocated_size: 76_209_750_016, file_count: 492, directory_count: 26_103, modified_at: "2026-09-09T06:30:00Z", is_dir: true, is_package: false, is_cloud_placeholder: false },
    { name: "Projects", path: "/Users/demo/Projects", logical_size: 54_835_773_440, allocated_size: 49_391_173_632, file_count: 338, directory_count: 8_421, modified_at: "2026-09-10T02:18:00Z", is_dir: true, is_package: false, is_cloud_placeholder: false },
    { name: "Pictures", path: "/Users/demo/Pictures", logical_size: 29_183_721_472, allocated_size: 26_982_268_928, file_count: 204, directory_count: 1_053, modified_at: "2026-09-08T13:42:00Z", is_dir: true, is_package: false, is_cloud_placeholder: false },
    { name: "Downloads", path: "/Users/demo/Downloads", logical_size: 18_012_946_432, allocated_size: 17_208_967_168, file_count: 128, directory_count: 218, modified_at: "2026-09-10T01:05:00Z", is_dir: true, is_package: false, is_cloud_placeholder: false },
    { name: "Documents", path: "/Users/demo/Documents", logical_size: 7_516_192_768, allocated_size: 6_744_244_224, file_count: 82, directory_count: 3_481, modified_at: "2026-09-09T10:22:00Z", is_dir: true, is_package: false, is_cloud_placeholder: false },
    { name: "Movies", path: "/Users/demo/Movies", logical_size: 2_506_498_048, allocated_size: 2_134_235_136, file_count: 40, directory_count: 566, modified_at: "2026-08-28T08:00:00Z", is_dir: true, is_package: false, is_cloud_placeholder: false },
  ],
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
        { path: "~/Library/Developer/Xcode/DerivedData/CleanMacProAI-bkpq", size: 2_104_983_552, is_dir: true },
        { path: "~/Library/Developer/Xcode/iOS DeviceSupport/17.4", size: 1_719_582_720, is_dir: true },
        { path: "~/Library/Developer/CoreSimulator/Caches/dyld", size: 842_006_528, is_dir: true },
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
        { path: "~/Library/Caches/Google/Chrome/Default/Cache/Cache_Data", size: 1_633_779_712, is_dir: true },
        { path: "~/Library/Caches/com.apple.Safari/fsCachedData", size: 964_689_920, is_dir: true },
        { path: "~/Library/Caches/Microsoft Edge/Default/Code Cache/js", size: 517_996_544, is_dir: true },
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
        { path: "~/Library/Caches/com.apple.helpd", size: 438_304_768, is_dir: true },
        { path: "~/Library/Caches/com.figma.Desktop", size: 388_923_392, is_dir: true },
        { path: "~/Library/Caches/com.spotify.client", size: 327_155_712, is_dir: true },
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
        { path: "~/Downloads/Xcode_15.4.xip", size: 1_020_813_312, is_dir: false },
        { path: "~/Downloads/Figma-124.7.dmg", size: 156_237_824, is_dir: false },
        { path: "~/Downloads/Node-22.1.0.pkg", size: 87_359_488, is_dir: false },
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
        { path: "~/Library/Mail/V10/MailData/Downloads/assets.zip", size: 128_974_848, is_dir: false },
        { path: "~/Library/Mail/V10/MailData/Downloads/report-final.pdf", size: 46_137_344, is_dir: false },
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
};
