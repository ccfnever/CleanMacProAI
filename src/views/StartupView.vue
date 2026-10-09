<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { invoke, isTauri } from '@tauri-apps/api/core';
import AppIcon from '../components/AppIcon.vue';
import { matchesStartup, startupKindLabels, startupPreview, type StartupFilter, type StartupItem, type StartupScan } from '../lib/startupItems';

const preview = !isTauri();
const items = ref<StartupItem[]>([]);
const warnings = ref<string[]>([]);
const loading = ref(true);
const pending = ref<string | null>(null);
const error = ref('');
const notice = ref('');
const query = ref('');
const filter = ref<StartupFilter>('all');
const kind = ref('all');
const expanded = ref(new Set<string>());
const scannedAt = ref('');
let disposed = false;
onUnmounted(() => { disposed = true; });
const filters = computed(() => [
  { id: 'all' as const, label: '全部', count: items.value.length },
  { id: 'enabled' as const, label: '已启用', count: items.value.filter(i => i.enabled === true).length },
  { id: 'disabled' as const, label: '已停用', count: items.value.filter(i => i.enabled === false).length },
  { id: 'missing' as const, label: '待检查', count: items.value.filter(i => i.missing_target).length },
]);
const visibleItems = computed(() => items.value.filter(i => matchesStartup(i, filter.value, query.value, kind.value)));
const enabledCount = computed(() => items.value.filter(i => i.enabled === true).length);
const manageableCount = computed(() => items.value.filter(i => i.manageable).length);
const unknownCount = computed(() => items.value.filter(i => i.enabled === null).length);

async function scan() {
  if (pending.value) return;
  loading.value = true;
  error.value = '';
  notice.value = '';
  try {
    // A native error is always shown; sample data is only used in the browser preview.
    const result = preview ? startupPreview() : await invoke<StartupScan>('list_startup_items');
    if (disposed) return;
    items.value = result.items;
    warnings.value = result.warnings;
    expanded.value = new Set([...expanded.value].filter(id => result.items.some(item => item.id === id)));
    scannedAt.value = new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
  } catch (e) {
    if (!disposed) error.value = `扫描未完成：${String(e)}。请重新扫描。`;
  } finally { if (!disposed) loading.value = false; }
}
function expand(id: string) {
  const next = new Set(expanded.value);
  if (next.has(id)) next.delete(id); else next.add(id);
  expanded.value = next;
}
async function toggle(item: StartupItem) {
  if (pending.value || loading.value || !item.manageable || item.enabled === null) return;
  pending.value = item.id;
  error.value = '';
  notice.value = '';
  const enabled = !item.enabled;
  if (!preview && item.requires_authorization) notice.value = '正在等待 macOS 管理员授权，请在系统弹窗中完成或取消。';
  try {
    const updated = preview ? { ...item, enabled } : await invoke<StartupItem>('set_startup_item_enabled', { path: item.path, enabled });
    if (disposed) return;
    items.value = items.value.map(row => row.id === item.id ? updated : row);
    notice.value = preview ? `已${enabled ? '启用' : '停用'}「${item.name}」的预览开关，未修改本机。`
      : `已${enabled ? '启用' : '停用'}「${item.name}」的自动启动。此设置用于后续登录或启动；当前进程保持原状。`;
  } catch (e) {
    if (!disposed) {
      notice.value = '';
      error.value = String(e).includes('已取消管理员授权') ? '已取消管理员授权，启动项状态未更改。' : `未能更新「${item.name}」：${String(e)}。请重新扫描确认系统状态。`;
    }
  } finally { if (!disposed) pending.value = null; }
}
async function openSettings() {
  error.value = '';
  if (preview) { notice.value = '桌面版可打开系统设置 → 通用 → 登录项与扩展。'; return; }
  try { await invoke('open_startup_settings'); } catch (e) { error.value = String(e); }
}
async function reveal(item: StartupItem) {
  if (preview) { notice.value = '预览路径仅用于演示；桌面版可在 Finder 中显示配置文件。'; return; }
  try { await invoke('open_in_finder', { path: item.path }); } catch (e) { error.value = `无法显示配置：${String(e)}`; }
}
function resetFilters() { filter.value = 'all'; query.value = ''; kind.value = 'all'; }
onMounted(scan);
</script>

<template>
  <section
    class="startup-page"
    aria-label="启动项管理"
  >
    <header class="startup-intro">
      <div>
        <p class="section-kicker">
          登录与后台启动
        </p>
        <h1>让每一次开机，轻一点。</h1>
        <p class="intro-copy">
          看清哪些程序会自动启动，按需保留，随时恢复。
        </p>
      </div>
      <button
        class="action-button"
        type="button"
        :disabled="loading || !!pending"
        @click="scan"
      >
        <AppIcon
          name="refresh"
          :size="15"
        />{{ loading ? '正在扫描' : '重新扫描' }}
      </button>
    </header>

    <section
      class="summary-panel"
      aria-label="扫描摘要"
    >
      <div class="summary-lead">
        <span class="summary-icon"><AppIcon
          name="power"
          :size="24"
        /></span><div><strong>{{ loading && !items.length ? '—' : items.length }} <small>个后台启动项</small></strong><p>{{ scannedAt ? `${preview ? '预览数据' : '最近扫描'} · ${scannedAt}` : '正在读取启动配置' }}<span v-if="unknownCount"> · {{ unknownCount }} 项状态未知</span></p></div>
      </div>
      <div class="summary-number">
        <strong>{{ enabledCount }}</strong><span>允许自动启动</span>
      </div>
      <div class="summary-number">
        <strong>{{ manageableCount }}</strong><span>可切换启动项</span>
      </div>
      <div class="summary-note">
        <AppIcon
          name="shield"
          :size="16"
        /><p>只调整启动设置<br>保留应用与配置文件</p>
      </div>
    </section>

    <div
      v-if="preview"
      class="message preview-note"
    >
      <AppIcon
        name="eye"
        :size="16"
      />当前为交互预览，列表为示例数据，开关不会修改本机。
    </div>
    <div
      v-if="error"
      class="message error-note"
      role="alert"
    >
      {{ error }}
    </div>
    <div
      v-if="notice"
      class="message success-note"
      role="status"
    >
      {{ notice }}
    </div>
    <details
      v-if="warnings.length"
      class="scan-warnings"
    >
      <summary>部分项目未能读取 · {{ warnings.length }} 条提示</summary><p
        v-for="warning in warnings"
        :key="warning"
      >
        {{ warning }}
      </p>
    </details>

    <section
      class="startup-list"
      :aria-busy="loading"
    >
      <div class="list-controls">
        <div
          class="filter-tabs"
          aria-label="启动状态筛选"
        >
          <button
            v-for="tab in filters"
            :key="tab.id"
            type="button"
            :class="{ active: filter === tab.id }"
            :aria-pressed="filter === tab.id"
            @click="filter = tab.id"
          >
            {{ tab.label }}<span>{{ tab.count }}</span>
          </button>
        </div>
        <label class="search-box"><AppIcon
          name="search"
          :size="16"
        /><input
          v-model="query"
          type="search"
          placeholder="搜索名称或标识符"
          aria-label="搜索启动项"
        ></label>
      </div>
      <div class="list-caption">
        <span>{{ visibleItems.length }} 个项目 <span v-if="filter === 'missing'">· 启动目标不存在，请检查配置</span></span><label>类型<select
          v-model="kind"
          aria-label="筛选启动项类型"
        ><option value="all">全部类型</option><option
          v-for="(label, id) in startupKindLabels"
          :key="id"
          :value="id"
        >{{ label }}</option></select></label>
      </div>
      <div
        v-if="loading"
        class="loading-state"
        role="status"
      >
        <div
          v-for="n in 5"
          :key="n"
          class="skeleton-row"
        >
          <i /><span /><b />
        </div><p>正在读取本机启动配置…</p>
      </div>
      <div
        v-else-if="!visibleItems.length"
        class="empty-state"
      >
        <AppIcon
          :name="items.length ? 'search' : 'power'"
          :size="30"
        /><h3>{{ error && !items.length ? '暂时无法读取启动项' : items.length ? '没有匹配的启动项' : '没有发现后台启动配置' }}</h3><p>{{ items.length ? '试试其他关键词或筛选条件。' : '登录时打开的应用也可以在系统设置中查看。' }}</p><button
          v-if="items.length"
          class="action-button"
          type="button"
          @click="resetFilters"
        >
          清除筛选
        </button><button
          v-else
          class="action-button"
          type="button"
          @click="error ? scan() : openSettings()"
        >
          {{ error ? '重试扫描' : '查看系统登录项' }}
        </button>
      </div>
      <template v-else>
        <article
          v-for="item in visibleItems"
          :key="item.id"
          :class="['startup-row', { expanded: expanded.has(item.id) }]"
        >
          <div class="row-main">
            <button
              class="identity"
              type="button"
              :aria-expanded="expanded.has(item.id)"
              :aria-controls="`startup-detail-${items.indexOf(item)}`"
              @click="expand(item.id)"
            >
              <span class="item-icon"><AppIcon
                :name="item.kind === 'system_daemon' ? 'settings' : 'apps'"
                :size="19"
              /></span>
              <span class="identity-copy"><strong>{{ item.name }}</strong><small>{{ item.label }}</small></span>
            </button>
            <span
              class="kind-label"
              :title="item.management_note"
            >{{ startupKindLabels[item.kind] }}<small v-if="item.requires_authorization">需管理员授权</small></span>
            <span :class="['state-label', { missing: item.missing_target }]">{{ item.missing_target ? '目标缺失' : item.enabled === null ? '状态未知' : item.enabled ? '已启用' : '已停用' }}</span>
            <button
              class="startup-switch"
              type="button"
              role="switch"
              :aria-checked="item.enabled === true"
              :aria-label="`${item.enabled ? '停用' : '启用'} ${item.name} 的自动启动`"
              :disabled="!!pending || loading || !item.manageable || item.enabled === null"
              :title="item.management_note"
              @click="toggle(item)"
            >
              <span /><i v-if="pending === item.id">·</i>
            </button>
            <button
              :class="['expand-button', { open: expanded.has(item.id) }]"
              type="button"
              :aria-expanded="expanded.has(item.id)"
              :aria-label="`${expanded.has(item.id) ? '收起' : '展开'} ${item.name} 的详情`"
              :aria-controls="`startup-detail-${items.indexOf(item)}`"
              @click="expand(item.id)"
            >
              <AppIcon
                name="chevron"
                :size="16"
              />
            </button>
          </div>
          <div
            v-if="expanded.has(item.id)"
            :id="`startup-detail-${items.indexOf(item)}`"
            class="item-detail"
          >
            <dl>
              <div><dt>配置文件</dt><dd>{{ item.path }}</dd></div><div><dt>启动目标</dt><dd>{{ item.program || '未提供独立执行路径' }}</dd></div><div v-if="item.arguments.length">
                <dt>启动参数</dt><dd>{{ item.arguments.join(' · ') }}</dd>
              </div><div><dt>生效范围</dt><dd>{{ item.kind === 'user_agent' ? '当前用户登录后' : item.kind === 'shared_agent' ? '各用户登录后' : '系统启动后' }}</dd></div>
            </dl>
            <p
              v-if="item.missing_target"
              class="target-note"
            >
              配置指向的执行文件不存在，可能是卸载残留或临时不可用的路径。确认用途后再决定是否停用。
            </p>
            <div class="detail-footer">
              <p>{{ item.management_note }}。{{ item.manageable ? '开关控制后续自动启动，不会立即启动或结束当前进程。' : '处理后请重新扫描。' }}</p><button
                class="text-button"
                type="button"
                @click="reveal(item)"
              >
                <AppIcon
                  name="folder"
                  :size="14"
                />在 Finder 中显示
              </button>
            </div>
          </div>
        </article>
      </template>
    </section>
    <aside class="settings-panel">
      <span class="settings-icon"><AppIcon
        name="settings"
        :size="20"
      /></span><div><strong>还有登录时打开的应用？</strong><p>这里扫描用户与第三方后台启动配置。登录项、应用内注册的后台服务和系统授权，请在 macOS 设置中查看。</p></div><button
        class="action-button"
        type="button"
        @click="openSettings"
      >
        打开登录项设置 <span aria-hidden="true">↗</span>
      </button>
    </aside>
    <p class="page-footnote">
      自动启动许可不代表程序当前正在运行。停用同步、更新或硬件相关服务前，请先查看其用途。
    </p>
  </section>
</template>

<style scoped>
.startup-page { max-width: 1240px; margin: 26px auto 0; }
.startup-intro { display: flex; justify-content: space-between; align-items: center; gap: 20px; margin-bottom: 22px; }
.section-kicker { margin: 0 0 8px; color: var(--accent); font-size: 10px; font-weight: 700; letter-spacing: .08em; }
h1 { margin: 0; font-size: 27px; font-weight: 600; letter-spacing: -.03em; line-height: 1.3; }
.intro-copy { margin: 8px 0 0; color: var(--text-soft); font-size: 12px; }
.action-button { display: inline-flex; align-items: center; justify-content: center; gap: 7px; flex-shrink: 0; min-height: 35px; padding: 0 12px; border: 1px solid var(--border); border-radius: 10px; color: var(--accent); background: var(--surface); font-size: 11px; font-weight: 600; transition: background 160ms, transform 160ms; }
.action-button:hover:not(:disabled) { background: var(--accent-soft); }
.action-button:active:not(:disabled) { transform: translateY(1px); }
button:disabled { cursor: wait; opacity: .55; }
.summary-panel { display: flex; align-items: center; gap: 32px; padding: 23px 25px; margin-bottom: 20px; border: 1px solid var(--border); border-radius: 18px; background: var(--surface); box-shadow: var(--shadow-soft); }
.summary-lead { display: flex; align-items: center; gap: 14px; flex: 1; }
.summary-icon { display: grid; place-items: center; width: 50px; height: 50px; border-radius: 15px; color: var(--accent); background: var(--accent-soft); }
.summary-lead strong { font-size: 30px; font-weight: 600; font-variant-numeric: tabular-nums; }
.summary-lead small { color: var(--text-soft); font-size: 12px; font-weight: 400; }
.summary-lead p { margin: 2px 0 0; color: var(--text-faint); font-size: 10px; }
.summary-number { display: grid; gap: 3px; padding-left: 25px; border-left: 1px solid var(--border); }
.summary-number strong { font-size: 24px; font-weight: 600; font-variant-numeric: tabular-nums; }
.summary-number span { color: var(--text-soft); font-size: 10px; }
.summary-note { display: flex; align-items: center; gap: 8px; color: var(--success); }
.summary-note p { margin: 0; font-size: 10px; line-height: 1.7; }
.message { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; padding: 11px 14px; border: 1px solid var(--border); border-radius: 10px; font-size: 11px; background: var(--surface-soft); }
.error-note { color: var(--danger); border-color: color-mix(in srgb, var(--danger) 30%, var(--border)); }
.success-note { color: var(--success); }
.preview-note { color: var(--text-soft); }
.scan-warnings { margin-bottom: 12px; padding: 11px 14px; border-radius: 10px; color: var(--warning); background: var(--surface-soft); font-size: 11px; overflow-wrap: anywhere; }
.scan-warnings summary { cursor: pointer; }
.startup-list { overflow: hidden; border: 1px solid var(--border); border-radius: 18px; background: var(--surface); box-shadow: var(--shadow-soft); }
.list-controls { display: flex; justify-content: space-between; align-items: center; gap: 14px; padding: 15px; }
.filter-tabs { display: flex; gap: 3px; }
.filter-tabs button { display: flex; align-items: center; gap: 7px; min-height: 33px; padding: 0 11px; border: 0; border-radius: 9px; color: var(--text-soft); background: transparent; font-size: 11px; transition: background 160ms; }
.filter-tabs button:hover { background: var(--surface-soft); }
.filter-tabs button.active { color: var(--accent-strong); background: var(--accent-soft); font-weight: 600; }
.filter-tabs span { font-size: 10px; font-variant-numeric: tabular-nums; opacity: .75; }
.search-box { display: flex; align-items: center; gap: 8px; width: 230px; height: 34px; padding: 0 10px; border: 1px solid var(--border); border-radius: 10px; background: var(--surface-soft); color: var(--text-faint); }
.search-box input { width: 100%; min-width: 0; border: 0; background: transparent; color: var(--text); font-size: 11px; }
.search-box input::placeholder { color: var(--text-faint); }
.list-caption { display: flex; justify-content: space-between; align-items: center; padding: 9px 17px; border-block: 1px solid var(--border); background: var(--surface-soft); color: var(--text-soft); font-size: 10px; }
.list-caption label { display: flex; align-items: center; gap: 7px; }
.list-caption select { padding: 4px 6px; border: 1px solid var(--border); border-radius: 7px; color: var(--text); background: var(--surface); font-size: 10px; }
.startup-row + .startup-row { border-top: 1px solid var(--border); }
.row-main { display: grid; grid-template-columns: minmax(0, 1fr) 118px 65px 70px 24px; align-items: center; gap: 13px; min-height: 73px; padding: 10px 17px; }
.startup-row.expanded { background: color-mix(in srgb, var(--accent-soft) 16%, var(--surface)); }
.identity { display: flex; align-items: center; gap: 12px; min-width: 0; padding: 0; border: 0; border-radius: 8px; color: var(--text); background: transparent; text-align: left; }
.item-icon { display: grid; place-items: center; flex: 0 0 38px; height: 38px; border: 1px solid var(--border); border-radius: 11px; color: var(--accent); background: var(--surface-soft); }
.identity-copy { display: grid; gap: 3px; min-width: 0; }
.identity-copy strong, .identity-copy small { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.identity-copy strong { font-size: 12px; font-weight: 600; }
.identity-copy small { color: var(--text-faint); font-size: 10px; }
.kind-label { padding: 4px 7px; border-radius: 7px; text-align: center; color: var(--text-soft); background: var(--surface-soft); font-size: 9px; }
.state-label { text-align: right; font-size: 10px; color: var(--text-faint); }
.state-label.missing { color: var(--warning); }
.startup-switch { position: relative; justify-self: center; width: 37px; height: 22px; padding: 3px; border: 0; border-radius: 20px; background: var(--surface-strong); box-shadow: inset 0 0 0 1px var(--border); transition: background 180ms; }
.startup-switch span { display: block; width: 16px; height: 16px; border-radius: 50%; background: var(--surface); box-shadow: 0 1px 3px #0002; transition: transform 180ms; }
.startup-switch[aria-checked="true"] { background: var(--accent); }
.startup-switch[aria-checked="true"] span { transform: translateX(15px); }
.startup-switch i { position: absolute; inset: 0; font-style: normal; color: var(--text); }
.kind-label small { display: block; margin-top: 2px; color: var(--text-faint); font-size: 8px; }
.startup-switch:disabled { cursor: not-allowed; }
.expand-button { display: grid; place-items: center; width: 24px; height: 27px; padding: 0; border: 0; border-radius: 6px; color: var(--text-faint); background: transparent; }
.expand-button svg { transition: transform 180ms; }
.expand-button.open svg { transform: rotate(90deg); }
.expand-button:hover { color: var(--accent); background: var(--accent-soft); }
.item-detail { margin: 0 17px 15px 67px; padding: 14px 16px; border: 1px solid var(--border); border-radius: 11px; background: var(--surface-soft); }
dl { display: grid; gap: 8px; margin: 0; }
dl > div { display: grid; grid-template-columns: 67px minmax(0, 1fr); gap: 12px; font-size: 10px; }
dt { color: var(--text-faint); }
dd { margin: 0; color: var(--text-soft); overflow-wrap: anywhere; user-select: text; }
.target-note { color: var(--warning); font-size: 10px; }
.detail-footer { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 13px; padding-top: 11px; border-top: 1px solid var(--border); }
.detail-footer p { margin: 0; color: var(--text-faint); font-size: 10px; }
.text-button { display: inline-flex; align-items: center; gap: 5px; flex-shrink: 0; padding: 3px; border: 0; border-radius: 5px; color: var(--accent); background: transparent; font-size: 10px; }
.empty-state { display: grid; justify-items: center; padding: 48px 20px; color: var(--text-faint); }
.empty-state h3 { margin: 13px 0 4px; color: var(--text); font-size: 14px; }
.empty-state p { margin: 0 0 17px; font-size: 11px; }
.loading-state { padding: 12px 17px; }
.loading-state p { color: var(--text-faint); text-align: center; font-size: 11px; }
.skeleton-row { display: flex; align-items: center; gap: 14px; height: 65px; }
.skeleton-row i, .skeleton-row span, .skeleton-row b { border-radius: 8px; background: var(--surface-strong); animation: breathe 1.2s ease-in-out infinite alternate; }
.skeleton-row i { width: 38px; height: 38px; }
.skeleton-row span { width: 38%; height: 16px; }
.skeleton-row b { width: 70px; height: 20px; margin-left: auto; }
@keyframes breathe { to { opacity: .4; } }
.settings-panel { display: flex; align-items: center; gap: 14px; margin-top: 18px; padding: 18px 20px; border: 1px solid var(--border); border-radius: 15px; background: color-mix(in srgb, var(--surface) 60%, transparent); }
.settings-icon { color: var(--accent); }
.settings-panel > div { flex: 1; }
.settings-panel strong { font-size: 12px; font-weight: 600; }
.settings-panel p { max-width: 680px; margin: 4px 0 0; color: var(--text-soft); font-size: 10px; line-height: 1.7; }
.page-footnote { margin: 14px 4px; color: var(--text-faint); font-size: 10px; }
@media (max-width: 1120px) {
  .summary-panel { gap: 20px; padding: 20px; }
  .summary-note { display: none; }
  .list-controls { flex-wrap: wrap; }
  .search-box { flex: 1; min-width: 170px; }
  .row-main { gap: 8px; grid-template-columns: minmax(0, 1fr) 106px 56px 64px 24px; }
  .detail-footer { align-items: flex-start; }
}
</style>
