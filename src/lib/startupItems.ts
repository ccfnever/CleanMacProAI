export interface StartupItem {
  id: string;
  name: string;
  label: string;
  kind: 'user_agent' | 'shared_agent' | 'system_daemon';
  path: string;
  program: string;
  arguments: string[];
  enabled: boolean | null;
  missing_target: boolean;
  manageable: boolean;
  requires_authorization: boolean;
  management_note: string;
}
export interface StartupScan { items: StartupItem[]; warnings: string[] }
export type StartupFilter = 'all' | 'enabled' | 'disabled' | 'missing';
export const startupKindLabels: Record<StartupItem['kind'], string> = {
  user_agent: '用户后台启动项', shared_agent: '共享后台启动项', system_daemon: '系统后台启动项',
};
export function matchesStartup(item: StartupItem, filter: StartupFilter, query: string, kind: string): boolean {
  const stateMatches = filter === 'all' || (filter === 'enabled' && item.enabled === true)
    || (filter === 'disabled' && item.enabled === false) || (filter === 'missing' && item.missing_target);
  const keyword = query.trim().toLocaleLowerCase();
  return stateMatches && (kind === 'all' || item.kind === kind)
    && (!keyword || [item.name, item.label, item.path, item.program].some(text => text.toLocaleLowerCase().includes(keyword)));
}
export function startupPreview(): StartupScan {
  const rows: [string, string, StartupItem['kind'], boolean, boolean][] = [
    ['Docker', 'com.docker.helper', 'system_daemon', true, false],
    ['Dropbox', 'com.dropbox.DropboxMacUpdate.agent', 'user_agent', true, false],
    ['Google Chrome', 'com.google.keystone.agent', 'shared_agent', true, false],
    ['Notion', 'notion.id.helper', 'user_agent', false, false],
    ['Microsoft AutoUpdate', 'com.microsoft.update.agent', 'shared_agent', true, false],
    ['Spotify', 'com.spotify.client.startup', 'user_agent', false, false],
    ['旧版同步助手', 'com.example.sync.helper', 'user_agent', true, true],
  ];
  return { warnings: [], items: rows.map(([name, label, kind, enabled, missing_target]) => {
    const path = `${kind === 'user_agent' ? '/Users/preview/Library/LaunchAgents' : kind === 'shared_agent' ? '/Library/LaunchAgents' : '/Library/LaunchDaemons'}/${label}.plist`;
    return { id: path, name, label, kind, enabled, missing_target, path, program: `/Applications/${name}.app/Contents/MacOS/${name}`, arguments: [], manageable: true, requires_authorization: kind === 'system_daemon', management_note: kind === 'system_daemon' ? '影响所有用户，切换时需要管理员授权' : '调整当前用户的自动启动许可' };
  }) };
}
