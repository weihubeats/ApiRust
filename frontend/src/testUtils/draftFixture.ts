import type { Endpoint } from '../types/foxApi'

/** 构造一个最小可用的接口草稿（含请求配置），供组件测试使用。 */
export function makeDraft(overrides: Partial<Endpoint> = {}): Endpoint {
  return {
    id: 'ep-test-1',
    project_id: 'proj-test-1',
    folder_id: null,
    name: '测试接口',
    method: 'GET',
    path: '/users',
    description: '',
    status: 'designing',
    sort_order: 0,
    request: {
      params: [],
      headers: [{ key: 'X-Token', value: 'abc', enabled: true, description: '' }],
      path_variables: [],
      auth: { type: 'bearer', token: 'tok123' },
      body: { mode: 'none' },
      timeout_ms: 30000,
      follow_redirects: true,
      tests: null,
    },
    created_at: '2026-01-01T00:00:00.000Z',
    updated_at: '2026-01-01T00:00:00.000Z',
    ...overrides,
  }
}