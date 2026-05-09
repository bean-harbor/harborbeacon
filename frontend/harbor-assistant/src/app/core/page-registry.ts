import { prefersChineseUi } from './ui-locale';

export type HarborAssistantPageId =
  | 'overview'
  | 'im-gateway'
  | 'account-management'
  | 'tasks-approvals'
  | 'devices-aiot'
  | 'home-assistant'
  | 'harboros'
  | 'models-policies'
  | 'system-settings';

export interface HarborAssistantPageDefinition {
  id: HarborAssistantPageId;
  path: string;
  label: string;
  labelZh: string;
  tagline: string;
  taglineZh: string;
  accent: 'teal' | 'amber' | 'sky' | 'rose';
}

export const HARBOR_ASSISTANT_PAGES: readonly HarborAssistantPageDefinition[] = [
  { id: 'overview', path: 'overview', label: 'Overview', labelZh: '总览', tagline: 'Release readiness and status digest', taglineZh: '发布 readiness 与状态摘要', accent: 'teal' },
  { id: 'im-gateway', path: 'im-gateway', label: 'IM Gateway', labelZh: 'IM 网关', tagline: 'Bridge state and source-bound delivery', taglineZh: '微信、飞书与回传状态', accent: 'sky' },
  { id: 'account-management', path: 'account-management', label: 'Account Management', labelZh: '账号与通知', tagline: 'Members, roles, and binding availability', taglineZh: '成员、角色与通知目标', accent: 'amber' },
  { id: 'tasks-approvals', path: 'tasks-approvals', label: 'Tasks & Approvals', labelZh: '任务与审批', tagline: 'High-risk actions and audited review', taglineZh: '高风险操作与审计', accent: 'rose' },
  { id: 'devices-aiot', path: 'devices-aiot', label: 'Devices & AIoT', labelZh: '设备与 AIoT', tagline: 'Discovery, preview, and device governance', taglineZh: '发现、预览与设备管理', accent: 'teal' },
  { id: 'home-assistant', path: 'home-assistant', label: 'Home Assistant', labelZh: 'Home Assistant', tagline: 'Bridge connection, entity sync, and managed install', taglineZh: '桥接连接、实体同步与托管安装', accent: 'sky' },
  { id: 'harboros', path: 'harboros', label: 'HarborOS', labelZh: 'HarborOS', tagline: 'System-domain boundaries and live/proof split', taglineZh: '系统域状态与边界', accent: 'sky' },
  { id: 'models-policies', path: 'models-policies', label: 'Models & Policies', labelZh: '模型与策略', tagline: 'Endpoint status, policy, and fallback order', taglineZh: '模型端点、策略与 fallback', accent: 'amber' },
  { id: 'system-settings', path: 'system-settings', label: 'System Settings', labelZh: '系统设置', tagline: 'Routing, gateway status, and blockers', taglineZh: '路由、网关状态与阻塞项', accent: 'rose' }
] as const;

export function pageById(pageId: HarborAssistantPageId): HarborAssistantPageDefinition {
  const page = HARBOR_ASSISTANT_PAGES.find((candidate) => candidate.id === pageId);
  if (!page) {
    throw new Error(`Unknown Harbor Assistant page id: ${pageId}`);
  }
  return page;
}

export function localizedHarborAssistantPages(): HarborAssistantPageDefinition[] {
  const useChinese = prefersChineseUi();
  return HARBOR_ASSISTANT_PAGES.map((page) => ({
    ...page,
    label: useChinese ? page.labelZh : page.label,
    tagline: useChinese ? page.taglineZh : page.tagline
  }));
}
