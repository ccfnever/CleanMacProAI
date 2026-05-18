<script setup lang="ts">
import { ref } from "vue";
import { invokeOrDemo } from "../lib/demoData";

const safeMode = ref(true);
const showHiddenFiles = ref(false);
const autoCleanCache = ref(false);
const autoUpdate = ref(true);
const language = ref("zh-CN");
const scanDepth = ref("balanced");
const permissionStatus = ref("建议开启，用于扫描系统级缓存和日志。");

async function openFullDiskAccess() {
  const result = await invokeOrDemo<boolean>("request_permissions", false);
  permissionStatus.value = result.source === "native"
    ? "已打开系统设置。添加 CleanMacProAI 后请重启应用。"
    : "浏览器预览无法打开系统设置，请在 macOS App 中操作。";
}
</script>

<template>
  <section class="settings-page">
    <div class="settings-hero">
      <div>
        <p class="section-kicker">设置</p>
        <h1>默认保守，必要时再深入。</h1>
        <p>清理产品最重要的是边界感：先保护用户数据，再谈释放空间。</p>
      </div>
    </div>

    <div class="settings-grid">
      <section class="panel">
        <div class="panel-head">
          <span>✓</span>
          <div>
            <h3>清理安全</h3>
            <p>控制删除方式和扫描边界。</p>
          </div>
        </div>

        <label class="setting-row">
          <span>
            <strong>安全模式</strong>
            <small>所有项目先移入废纸篓，不直接删除。</small>
          </span>
          <input v-model="safeMode" type="checkbox" />
        </label>

        <label class="setting-row">
          <span>
            <strong>显示隐藏文件</strong>
            <small>扫描报告中显示隐藏目录命中的文件。</small>
          </span>
          <input v-model="showHiddenFiles" type="checkbox" />
        </label>

        <label class="setting-row">
          <span>
            <strong>启动后自动扫描缓存</strong>
            <small>仅扫描低风险缓存，不自动执行清理。</small>
          </span>
          <input v-model="autoCleanCache" type="checkbox" />
        </label>
      </section>

      <section class="panel permission-panel">
        <div class="panel-head">
          <span>◆</span>
          <div>
            <h3>macOS 权限</h3>
            <p>完整扫描需要完全磁盘访问权限，尤其是系统缓存、日志和部分应用残留。</p>
          </div>
        </div>

        <div class="permission-box">
          <div>
            <strong>完全磁盘访问</strong>
            <small>{{ permissionStatus }}</small>
          </div>
          <button type="button" @click="openFullDiskAccess">打开系统设置</button>
        </div>
      </section>

      <section class="panel">
        <div class="panel-head">
          <span>⚙</span>
          <div>
            <h3>常规偏好</h3>
            <p>控制更新、语言和扫描深度。</p>
          </div>
        </div>

        <label class="setting-row">
          <span>
            <strong>自动更新</strong>
            <small>有新版本时提示下载并安装。</small>
          </span>
          <input v-model="autoUpdate" type="checkbox" />
        </label>

        <label class="select-row">
          <span>
            <strong>语言</strong>
            <small>界面显示语言。</small>
          </span>
          <select v-model="language">
            <option value="zh-CN">简体中文</option>
            <option value="en">English</option>
          </select>
        </label>

        <label class="select-row">
          <span>
            <strong>扫描深度</strong>
            <small>Demo 中会影响后续扫描策略。</small>
          </span>
          <select v-model="scanDepth">
            <option value="safe">仅安全项</option>
            <option value="balanced">平衡</option>
            <option value="deep">深度</option>
          </select>
        </label>
      </section>

      <section class="panel full">
        <div class="panel-head">
          <span>i</span>
          <div>
            <h3>产品状态</h3>
            <p>当前版本更像可演示原型，Rust 扫描/卸载引擎还需要继续补齐。</p>
          </div>
        </div>

        <div class="about-grid">
          <div>
            <small>版本</small>
            <strong>0.1.0</strong>
          </div>
          <div>
            <small>运行策略</small>
            <strong>本地优先</strong>
          </div>
          <div>
            <small>清理规则</small>
            <strong>YAML 可审计</strong>
          </div>
          <div>
            <small>默认删除</small>
            <strong>移入废纸篓</strong>
          </div>
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.settings-page {
  max-width: 1180px;
  margin: 0 auto;
}

.settings-hero,
.panel {
  border: 1px solid rgba(35, 52, 45, 0.09);
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.78);
  box-shadow: 0 18px 52px rgba(28, 49, 42, 0.08);
}

.settings-hero {
  padding: 28px;
}

.section-kicker {
  margin: 0 0 8px;
  color: #1f8b72;
  font-size: 12px;
  font-weight: 850;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

h1 {
  margin: 0;
  font-size: 40px;
  letter-spacing: 0;
}

.settings-hero p:not(.section-kicker),
.panel-head p {
  margin: 8px 0 0;
  color: #687871;
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 18px;
  margin-top: 18px;
}

.panel {
  padding: 22px;
}

.panel.full {
  grid-column: 1 / -1;
}

.permission-panel {
  grid-column: 1 / -1;
}

.panel-head {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  margin-bottom: 18px;
}

.panel-head > span {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border-radius: 12px;
  background: rgba(31, 139, 114, 0.12);
  color: #1f8b72;
  font-size: 20px;
}

.panel h3 {
  margin: 0;
  font-size: 18px;
}

.setting-row,
.select-row,
.permission-box {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding: 16px 0;
  border-top: 1px solid rgba(35, 52, 45, 0.08);
}

.setting-row strong,
.select-row strong {
  display: block;
  font-size: 14px;
}

.setting-row small,
.select-row small,
.permission-box small {
  display: block;
  margin-top: 4px;
  color: #73827c;
  font-size: 12px;
  line-height: 1.5;
}

.permission-box {
  padding: 16px;
  border-top: 1px solid rgba(35, 52, 45, 0.08);
  border-radius: 14px;
  background: #f5f7f6;
}

.permission-box strong {
  display: block;
  font-size: 14px;
}

.permission-box button {
  min-height: 40px;
  padding: 0 14px;
  border: 0;
  border-radius: 11px;
  background: #172026;
  color: #fff;
  font-weight: 850;
}

input[type="checkbox"] {
  width: 42px;
  height: 24px;
  appearance: none;
  border-radius: 999px;
  background: #cad5d0;
  position: relative;
  transition: background 180ms ease;
}

input[type="checkbox"]::after {
  content: "";
  position: absolute;
  top: 3px;
  left: 3px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #fff;
  transition: transform 180ms ease;
}

input[type="checkbox"]:checked {
  background: #1f8b72;
}

input[type="checkbox"]:checked::after {
  transform: translateX(18px);
}

select {
  min-width: 132px;
  min-height: 38px;
  padding: 0 12px;
  border: 1px solid rgba(35, 52, 45, 0.14);
  border-radius: 10px;
  background: #f6f8f7;
  color: #172026;
  outline: 0;
}

.about-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}

.about-grid div {
  padding: 16px;
  border-radius: 14px;
  background: #f5f7f6;
}

.about-grid small {
  display: block;
  color: #73827c;
  font-weight: 800;
}

.about-grid strong {
  display: block;
  margin-top: 6px;
  font-size: 17px;
}
</style>
