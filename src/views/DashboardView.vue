<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import AppIcon from "../components/AppIcon.vue";
import foxImage from "../assets/themes/fox.png";
import forestImage from "../assets/themes/forest.png";
import { formatBytes, type DiskInfo } from "../lib/demoData";
import { useScannerStore } from "../stores/scanner";
import { useThemeStore } from "../stores/theme";

const props = defineProps<{ diskInfo: DiskInfo; dataSource: "native" | "demo" }>();
const emit = defineEmits<{ navigate: [view: string] }>();

const scannerStore = useScannerStore();
const themeStore = useThemeStore();
const { scanResults, totalCleanable, totalFileCount } = storeToRefs(scannerStore);
const { currentTheme } = storeToRefs(themeStore);

const hasScanResults = computed(() => scanResults.value.length > 0);
const heroImage = computed(() => currentTheme.value === "pet" ? foxImage : forestImage);
const diskStatus = computed(() => {
  if (props.diskInfo.usage_percent >= 90) return "空间有点拥挤";
  if (props.diskInfo.usage_percent >= 80) return "可以整理一下";
  return "今天状态不错";
});
const themeGreeting = computed(() => ({
  pet: "小狐狸会陪你看清每一项，再决定是否清理。",
  nature: "像整理林间小径一样，让空间恢复呼吸。",
  classic: "用克制、清楚的方式管理你的 Mac 空间。",
})[currentTheme.value]);
</script>

<template>
  <section class="dashboard-page">
    <section class="hero-panel" :style="{ '--hero-image': `url(${heroImage})` }">
      <div class="hero-copy">
        <span class="hero-tag"><AppIcon name="shield" :size="14" /> 本地扫描 · 默认保守</span>
        <h1>{{ diskStatus }}，<br>一起把 Mac 收拾舒服。</h1>
        <p>{{ themeGreeting }}</p>
        <div class="hero-actions">
          <button type="button" class="primary-action" @click="emit('navigate', 'scanner')">
            <AppIcon :name="hasScanResults ? 'check' : 'scan'" :size="17" />
            {{ hasScanResults ? "查看扫描结果" : "开始安心扫描" }}
          </button>
          <button type="button" class="secondary-action" @click="emit('navigate', 'uninstaller')">
            <AppIcon name="apps" :size="16" /> 管理应用
          </button>
        </div>
      </div>
      <div class="hero-caption">
        <span>{{ themeStore.currentThemeLabel }}</span>
        <small>主题会同步改变整套界面的色彩与氛围</small>
      </div>
    </section>

    <section class="intro-panel">
      <div class="intro-copy">
        <p class="section-kicker">为什么是 CleanMacProAI</p>
        <h2>先说明白，再帮你动手。</h2>
        <p>
          它会把缓存、日志、安装残留和应用关联文件分开说明，显示来源、大小和风险。
          低风险项目可以快速处理，需要判断的内容会保留，高风险数据默认不选。
        </p>
      </div>
      <div class="scan-status">
        <span class="status-icon"><AppIcon :name="hasScanResults ? 'check' : 'scan'" :size="19" /></span>
        <div>
          <strong>{{ hasScanResults ? `发现 ${formatBytes(totalCleanable)}` : "还没有扫描记录" }}</strong>
          <small>{{ hasScanResults ? `${scanResults.length} 类 · ${totalFileCount.toLocaleString()} 个文件，点开可逐项查看` : "一次扫描只做分析，不会自动删除任何内容" }}</small>
        </div>
        <button type="button" @click="emit('navigate', 'scanner')">{{ hasScanResults ? "继续查看" : "去扫描" }} <span>→</span></button>
      </div>
    </section>

    <section class="principles" aria-label="产品原则">
      <article>
        <span><AppIcon name="shield" :size="19" /></span>
        <div><strong>安全有边界</strong><p>个人文档、钥匙串和偏好设置默认不进入自动选择。</p></div>
      </article>
      <article>
        <span><AppIcon name="folder" :size="19" /></span>
        <div><strong>详情在原位展开</strong><p>不用在窗口间来回跳，路径、大小和类型都在列表内查看。</p></div>
      </article>
      <article>
        <span><AppIcon name="trash" :size="19" /></span>
        <div><strong>按操作明确处理</strong><p>安心清理直接删除已选项目；空间地图与应用卸载移入废纸篓。</p></div>
      </article>
    </section>
  </section>
</template>

<style scoped>
.dashboard-page { max-width: 1240px; margin: 20px auto 0; }
.hero-panel {
  --hero-image: none;
  position: relative;
  display: flex;
  align-items: stretch;
  min-height: 390px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--surface) 60%, var(--border));
  border-radius: 24px;
  color: #fff;
  background-image: var(--hero-overlay), var(--hero-image);
  background-position: center, center 48%;
  background-size: cover;
  box-shadow: var(--shadow);
}
.hero-panel::after { content: ""; position: absolute; inset: 0; pointer-events: none; border-radius: inherit; box-shadow: inset 0 1px 0 rgba(255,255,255,.35); }
:global(:root[data-theme="nature"]) .hero-panel { background-position: center, center 54%; }
:global(:root[data-theme="classic"]) .hero-panel { background-position: center, center 54%; filter: saturate(.7); }
.hero-copy { position: relative; z-index: 1; align-self: center; width: min(610px, 62%); padding: 48px 52px; }
.hero-tag { display: inline-flex; align-items: center; gap: 7px; min-height: 28px; padding: 0 11px; border: 1px solid rgba(255,255,255,.28); border-radius: 999px; background: rgba(255,255,255,.12); font-size: 10px; font-weight: 750; backdrop-filter: blur(12px); }
.hero-copy h1 { margin: 18px 0 0; font-family: var(--font-ui); font-size: clamp(34px, 3.4vw, 52px); font-weight: 600; line-height: 1.12; letter-spacing: -.035em; text-wrap: balance; }
.hero-copy > p { max-width: 480px; margin: 16px 0 0; color: rgba(255,255,255,.82); font-size: 14px; line-height: 1.75; }
.hero-actions { display: flex; gap: 10px; margin-top: 27px; }
.hero-actions button { display: inline-flex; align-items: center; justify-content: center; gap: 8px; min-height: 42px; padding: 0 17px; border-radius: 11px; font-size: 12px; font-weight: 800; transition: transform 150ms ease, background 150ms ease; }
.hero-actions button:hover { transform: translateY(-1px); }
.primary-action { border: 0; color: var(--accent-strong); background: #fff; box-shadow: 0 10px 28px rgba(0,0,0,.15); }
.secondary-action { border: 1px solid rgba(255,255,255,.32); color: #fff; background: rgba(255,255,255,.1); backdrop-filter: blur(12px); }
.secondary-action:hover { background: rgba(255,255,255,.17); }
.hero-caption { position: absolute; z-index: 1; right: 18px; bottom: 16px; display: grid; max-width: 250px; padding: 10px 12px; border: 1px solid rgba(255,255,255,.2); border-radius: 10px; color: #fff; background: rgba(28,30,28,.2); text-align: right; backdrop-filter: blur(12px); }
.hero-caption span { font-size: 11px; font-weight: 800; }
.hero-caption small { margin-top: 2px; color: rgba(255,255,255,.68); font-size: 9px; }

.intro-panel { display: grid; grid-template-columns: minmax(0, 1fr) 430px; gap: 28px; align-items: end; margin-top: 16px; padding: 27px 30px; border: 1px solid var(--border); border-radius: 18px; background: var(--surface); box-shadow: var(--shadow-soft); }
.section-kicker { margin: 0 0 7px; color: var(--accent); font-size: 10px; font-weight: 850; letter-spacing: .09em; text-transform: uppercase; }
.intro-copy h2 { margin: 0; color: var(--text); font-family: var(--font-ui); font-size: 25px; font-weight: 600; letter-spacing: -.02em; }
.intro-copy > p:last-child { max-width: 650px; margin: 10px 0 0; color: var(--text-soft); font-size: 12px; line-height: 1.8; }
.scan-status { display: grid; grid-template-columns: 38px 1fr auto; align-items: center; gap: 11px; padding: 14px; border-radius: 13px; background: var(--surface-soft); }
.status-icon { display: grid; place-items: center; width: 38px; height: 38px; border-radius: 11px; color: var(--accent); background: var(--accent-soft); }
.scan-status strong, .scan-status small { display: block; }
.scan-status strong { color: var(--text); font-size: 12px; }
.scan-status small { margin-top: 3px; color: var(--text-faint); font-size: 9px; line-height: 1.4; }
.scan-status button { padding: 7px 0 7px 10px; border: 0; color: var(--accent); background: transparent; font-size: 10px; font-weight: 800; white-space: nowrap; }

.principles { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin-top: 12px; }
.principles article { display: flex; gap: 12px; min-height: 96px; padding: 18px; border: 1px solid var(--border); border-radius: 16px; background: color-mix(in srgb, var(--surface) 80%, transparent); }
.principles article > span { display: grid; place-items: center; flex: 0 0 36px; width: 36px; height: 36px; border-radius: 10px; color: var(--accent); background: var(--accent-soft); }
.principles strong { color: var(--text); font-size: 12px; }
.principles p { margin: 5px 0 0; color: var(--text-soft); font-size: 10px; line-height: 1.55; }

@media (max-width: 1120px) {
  .hero-panel { min-height: 350px; }
  .hero-copy { width: 68%; padding: 40px; }
  .intro-panel { grid-template-columns: 1fr; align-items: stretch; }
}
</style>
