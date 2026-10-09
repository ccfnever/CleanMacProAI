<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { storeToRefs } from "pinia";
import AppIcon from "../components/AppIcon.vue";
import { invokeOrDemo } from "../lib/demoData";
import { useThemeStore, type ThemeName } from "../stores/theme";

const appVersion = ref("浏览器预览");
onMounted(async () => {
  try { appVersion.value = await getVersion(); } catch { /* Browser preview has no native app version. */ }
});
const permissionStatus = ref("建议开启，用于扫描系统级缓存和日志。");
const themeStore = useThemeStore();
const { currentTheme } = storeToRefs(themeStore);

const themeOptions: Array<{ id: ThemeName; name: string; description: string; icon: string }> = [
  { id: "pet", name: "萌宠暖阳", description: "奶油底色、陶土棕与小狐狸陪伴", icon: "paw" },
  { id: "nature", name: "自然森林", description: "鼠尾草绿、林间光影与舒展留白", icon: "leaf" },
  { id: "classic", name: "经典雾蓝", description: "冷静中性、雾蓝层次与专业质感", icon: "disk" },
];

async function openFullDiskAccess() {
  const result = await invokeOrDemo<boolean>("request_permissions", false);
  if (result.source === "error") {
    permissionStatus.value = `无法打开系统设置：${result.error}`;
    return;
  }
  permissionStatus.value = result.source === "native"
    ? "已打开系统设置。添加 CleanMacProAI 后请重启应用。"
    : "浏览器预览无法打开系统设置，请在 macOS App 中操作。";
}
</script>

<template>
  <section class="settings-page">
    <header class="settings-intro">
      <p class="section-kicker">外观与偏好</p>
      <h1>让工具更像你的空间。</h1>
      <p>主题会立即作用于整套界面并自动记住；扫描与清理的安全边界保持不变。</p>
    </header>

    <section class="panel theme-panel">
      <div class="panel-head">
        <span><AppIcon name="leaf" :size="19" /></span>
        <div><h2>主题皮肤</h2><p>色彩、背景图和交互状态会一起切换。</p></div>
      </div>
      <div class="theme-grid">
        <button v-for="theme in themeOptions" :key="theme.id" type="button" :class="['theme-card', theme.id, { active: currentTheme === theme.id }]" @click="themeStore.applyTheme(theme.id)">
          <span class="theme-preview"><i></i><i></i><i></i><b><AppIcon :name="theme.icon" :size="21" /></b></span>
          <span class="theme-copy"><strong>{{ theme.name }}</strong><small>{{ theme.description }}</small></span>
          <span class="theme-check">{{ currentTheme === theme.id ? "✓" : "" }}</span>
        </button>
      </div>
    </section>

    <section class="panel about-panel">
      <div class="panel-head"><span><AppIcon name="settings" :size="19" /></span><div><h2>应用信息</h2><p>本地运行 · 清理需手动确认</p></div></div>
      <div class="version-row"><span><strong>当前版本</strong><small>主题偏好自动保存；删除默认移入废纸篓。</small></span><b>{{ appVersion }}</b></div>
    </section>

    <section class="panel permission-panel">
      <div class="permission-icon"><AppIcon name="disk" :size="21" /></div>
      <div><h2>完全磁盘访问</h2><p>{{ permissionStatus }}</p></div>
      <button type="button" @click="openFullDiskAccess">打开系统设置</button>
    </section>
  </section>
</template>

<style scoped>
.about-panel { margin-top: 12px; }
.settings-page { max-width: 1240px; margin: 20px auto 0; color: var(--text); }
.settings-intro { padding: 24px 28px; border: 1px solid var(--border); border-radius: 18px; background: var(--surface); box-shadow: var(--shadow-soft); }
.section-kicker { margin: 0 0 7px; color: var(--accent); font-size: 9px; font-weight: 850; letter-spacing: .09em; text-transform: uppercase; }
.settings-intro h1 { margin: 0; color: var(--text); font-family: var(--font-ui); font-size: 30px; font-weight: 600; letter-spacing: -.02em; }
.settings-intro > p:last-child { margin: 8px 0 0; color: var(--text-soft); font-size: 11px; }
.panel { padding: 21px; border: 1px solid var(--border); border-radius: 17px; background: var(--surface); box-shadow: var(--shadow-soft); }
.theme-panel { margin-top: 12px; }
.panel-head { display: flex; align-items: flex-start; gap: 10px; margin-bottom: 16px; }
.panel-head > span, .permission-icon { display: grid; place-items: center; flex: 0 0 36px; width: 36px; height: 36px; border-radius: 10px; color: var(--accent); background: var(--accent-soft); }
.panel h2 { margin: 0; color: var(--text); font-size: 14px; }
.panel-head p, .permission-panel p { margin: 4px 0 0; color: var(--text-soft); font-size: 10px; }
.theme-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }
.theme-card { position: relative; display: grid; grid-template-columns: 70px 1fr 20px; align-items: center; gap: 12px; min-height: 82px; padding: 10px; border: 1px solid var(--border); border-radius: 13px; color: var(--text); background: var(--surface-soft); text-align: left; transition: transform 150ms ease, border 150ms ease, box-shadow 150ms ease; }
.theme-card:hover { transform: translateY(-1px); border-color: var(--border-strong); }
.theme-card.active { border-color: color-mix(in srgb, var(--accent) 48%, var(--border)); box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 9%, transparent); }
.theme-preview { position: relative; display: block; width: 70px; height: 58px; overflow: hidden; border-radius: 10px; background: #e9dfd3; }
.theme-preview > i { position: absolute; width: 36px; height: 7px; left: 8px; border-radius: 4px; background: rgba(109,76,54,.18); }
.theme-preview > i:nth-child(1) { top: 10px; width: 24px; }
.theme-preview > i:nth-child(2) { top: 22px; }
.theme-preview > i:nth-child(3) { top: 34px; width: 29px; }
.theme-preview b { position: absolute; right: 7px; bottom: 7px; display: grid; place-items: center; width: 29px; height: 29px; border-radius: 9px; color: #fff; background: #9c5e38; }
.theme-card.nature .theme-preview { background: #dce6d8; }
.theme-card.nature .theme-preview > i { background: rgba(51,84,59,.18); }
.theme-card.nature .theme-preview b { background: #4e7658; }
.theme-card.classic .theme-preview { background: #dfe5ea; }
.theme-card.classic .theme-preview > i { background: rgba(50,73,91,.18); }
.theme-card.classic .theme-preview b { background: #4e6f88; }
.theme-copy strong, .theme-copy small { display: block; }
.theme-copy strong { font-size: 11px; }
.theme-copy small { margin-top: 4px; color: var(--text-faint); font-size: 9px; line-height: 1.4; }
.theme-check { display: grid; place-items: center; width: 20px; height: 20px; border-radius: 50%; color: #fff; background: var(--accent); font-size: 10px; font-weight: 900; }
.settings-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-top: 12px; }
.setting-row, .select-row, .version-row { display: flex; align-items: center; justify-content: space-between; gap: 18px; min-height: 61px; padding: 12px 0; border-top: 1px solid var(--border); }
.setting-row strong, .select-row strong, .version-row strong { display: block; color: var(--text); font-size: 11px; }
.setting-row small, .select-row small, .version-row small { display: block; margin-top: 3px; color: var(--text-faint); font-size: 9px; line-height: 1.4; }
input[type="checkbox"] { position: relative; flex: 0 0 38px; width: 38px; height: 22px; margin: 0; appearance: none; border-radius: 999px; background: var(--surface-strong); transition: background 150ms ease; }
input[type="checkbox"]::after { content: ""; position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: #fff; box-shadow: 0 2px 5px rgba(0,0,0,.14); transition: transform 150ms ease; }
input[type="checkbox"]:checked { background: var(--accent); }
input[type="checkbox"]:checked::after { transform: translateX(16px); }
select { min-width: 112px; height: 34px; padding: 0 9px; border: 1px solid var(--border); border-radius: 9px; color: var(--text); background: var(--surface-soft); font-size: 10px; }
.version-row b { color: var(--text-soft); font-size: 11px; }
.permission-panel { display: grid; grid-template-columns: 38px minmax(0, 1fr) auto; align-items: center; gap: 12px; margin-top: 12px; }
.permission-panel h2 { margin: 0; }
.permission-panel button { min-height: 36px; padding: 0 13px; border: 0; border-radius: 9px; color: #fff; background: var(--accent); font-size: 10px; font-weight: 800; }
@media (max-width: 1050px) { .theme-card { grid-template-columns: 54px 1fr 20px; } .theme-preview { width: 54px; } .settings-grid { grid-template-columns: 1fr; } }
</style>
