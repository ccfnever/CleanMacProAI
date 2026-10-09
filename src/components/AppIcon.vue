<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(defineProps<{ name: string; size?: number; strokeWidth?: number }>(), {
  size: 20,
  strokeWidth: 1.8,
});

const paths: Record<string, string[]> = {
  power: ["M12 3v9", "M6.3 5.7a9 9 0 1 0 11.4 0"],
  home: ["M3 10.5 12 3l9 7.5", "M5 9.5V21h14V9.5", "M9 21v-7h6v7"],
  scan: ["M4 7V4h3", "M17 4h3v3", "M20 17v3h-3", "M7 20H4v-3", "M8 12h8", "M12 8v8"],
  apps: ["M4 4h6v6H4z", "M14 4h6v6h-6z", "M4 14h6v6H4z", "M14 14h6v6h-6z"],
  settings: ["M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z", "M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2 3.46-.08-.02a1.7 1.7 0 0 0-1.8.26l-.52.3a1.7 1.7 0 0 0-.82 1.72V23h-4v-.08a1.7 1.7 0 0 0-.82-1.72l-.52-.3a1.7 1.7 0 0 0-1.8-.26l-.08.02-2-3.46.06-.06A1.7 1.7 0 0 0 4.6 15v-.6a1.7 1.7 0 0 0-1-1.56L3.5 12l2-3.46.08.02a1.7 1.7 0 0 0 1.8-.26l.52-.3a1.7 1.7 0 0 0 .82-1.72V6h4v.08a1.7 1.7 0 0 0 .82 1.72l.52.3a1.7 1.7 0 0 0 1.8.26l.08-.02 2 3.46-.06.06a1.7 1.7 0 0 0-.34 1.88Z"],
  disk: ["M4 6.5A2.5 2.5 0 0 1 6.5 4h11A2.5 2.5 0 0 1 20 6.5v11a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 4 17.5Z", "M7 16h.01", "M10 16h7", "M8 9h8"],
  shield: ["M12 3 5 6v5c0 4.7 2.8 8.1 7 10 4.2-1.9 7-5.3 7-10V6Z", "m9 12 2 2 4-4"],
  chevron: ["m9 18 6-6-6-6"],
  search: ["m21 21-4.3-4.3", "M19 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0Z"],
  folder: ["M3 6.5A1.5 1.5 0 0 1 4.5 5H10l2 2h7.5A1.5 1.5 0 0 1 21 8.5v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5Z"],
  map: ["M9 18 3.5 21V6L9 3l6 3 5.5-3v15L15 21Z", "M9 3v15", "M15 6v15"],
  file: ["M6 3h8l4 4v14H6Z", "M14 3v5h5"],
  eye: ["M2.5 12s3.5-6 9.5-6 9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6Z", "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"],
  trash: ["M4 7h16", "M9 7V4h6v3", "m7 7 1 13h10l1-13", "M10 11v5", "M14 11v5"],
  refresh: ["M20 6v5h-5", "M4 18v-5h5", "M18.5 9A7 7 0 0 0 6.8 6.8L4 11", "M5.5 15A7 7 0 0 0 17.2 17.2L20 13"],
  check: ["m5 12 4 4L19 6"],
  leaf: ["M20 4C12 4 5 8 5 15c0 3 2 5 5 5 7 0 10-8 10-16Z", "M5 20c2-5 6-8 11-11"],
  paw: ["M8.5 11.5c-2.2 0-4 2.2-4 4.5 0 2.5 2 4 4.3 3.2L12 17.5l3.2 1.7c2.3.8 4.3-.7 4.3-3.2 0-2.3-1.8-4.5-4-4.5-1.5 0-2.4.8-3.5.8s-2-.8-3.5-.8Z", "M6.5 8.5a2 2.5 0 1 0 0-5 2 2.5 0 0 0 0 5Z", "M17.5 8.5a2 2.5 0 1 0 0-5 2 2.5 0 0 0 0 5Z", "M11 7a2 2.7 0 1 0 0-5.4A2 2.7 0 0 0 11 7Z"],
};

const iconPaths = computed(() => paths[props.name] ?? paths.home);
</script>

<template>
  <svg
    aria-hidden="true"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="strokeWidth"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path v-for="path in iconPaths" :key="path" :d="path" />
  </svg>
</template>
