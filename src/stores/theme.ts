import { defineStore } from "pinia";
import { computed, ref } from "vue";

export type ThemeName = "pet" | "nature" | "classic";

const themeMeta: Record<ThemeName, { label: string; shortLabel: string }> = {
  pet: { label: "萌宠暖阳", shortLabel: "萌宠" },
  nature: { label: "自然森林", shortLabel: "自然" },
  classic: { label: "经典雾蓝", shortLabel: "经典" },
};

function initialTheme(): ThemeName {
  if (typeof window === "undefined") return "pet";
  const saved = window.localStorage.getItem("cleanmac-theme");
  return saved === "nature" || saved === "classic" || saved === "pet" ? saved : "pet";
}

export const useThemeStore = defineStore("theme", () => {
  const currentTheme = ref<ThemeName>(initialTheme());
  const currentThemeLabel = computed(() => themeMeta[currentTheme.value].label);

  function applyTheme(theme: ThemeName) {
    currentTheme.value = theme;
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem("cleanmac-theme", theme);
  }

  function initializeTheme() {
    document.documentElement.dataset.theme = currentTheme.value;
  }

  return { currentTheme, currentThemeLabel, themeMeta, applyTheme, initializeTheme };
});
