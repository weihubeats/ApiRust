/**
 * TestCaseDrawer 单测：打开回填 / 修改后保存 / 原地运行（结果留在抽屉内，不切 Tab）。
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import TestCaseDrawer from './TestCaseDrawer.vue'
import type { TestCase } from '../types/foxApi'

function makeCase(): TestCase {
  return {
    id: 'c1',
    request_id: 'ep-1',
    name: '内部划转-SGB',
    category: '正向',
    method: 'POST',
    url_path: '/funds/transfer',
    params: [{ key: 'env', value: 'prod', enabled: true, description: '' }],
    headers: [{ key: 'X-Trace', value: 'on', enabled: true, description: '' }],
    body_type: 'json',
    body_content: '{"amount":100}',
    last_run_status: 'Untested',
    created_at: '2026-01-01T00:00:00.000Z',
  }
}

function mountDrawer(overrides: { open?: boolean; testCase?: TestCase | null } = {}) {
  const onRun = vi.fn()
  const onSave = vi.fn()
  const wrapper = mount(TestCaseDrawer, {
    props: {
      open: overrides.open ?? true,
      endpointId: 'ep-1',
      testCase: overrides.testCase ?? makeCase(),
      onRun,
      onSave,
    },
  })
  return { wrapper, onRun, onSave }
}

describe('TestCaseDrawer', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
  })

  it('打开时回填用例内容（名称 / 分组 / Path / Body）', async () => {
    mountDrawer()
    const inputs = Array.from(document.querySelectorAll<HTMLInputElement>('.drw-input'))
    expect(inputs.map((i) => i.value)).toEqual(
      expect.arrayContaining(['内部划转-SGB', '/funds/transfer']),
    )
    // Body 位于请求参数 Tab（Params/Headers/Body）内，由 CodeMirror 承载
    const bodyTab = Array.from(document.querySelectorAll('.drw-tab')).find((b) => b.textContent === 'Body')
    ;(bodyTab as HTMLButtonElement).click()
    await vi.waitFor(() => {
      const content = document.querySelector<HTMLElement>('.cm-content')
      expect(content?.textContent?.replace(/\s+/g, '')).toBe('{"amount":100}')
    })
  })

  it('Method 联动：POST 默认 Body Tab + 自动补 Content-Type；GET 默认 Params Tab + Body 提示', async () => {
    mountDrawer()
    // POST 用例 → 默认落到 Body Tab，且自动补 Content-Type: application/json
    await vi.waitFor(() => {
      const active = document.querySelector<HTMLElement>('.drw-tab.active')
      expect(active?.textContent).toBe('Body')
    })
    expect(document.querySelector('.cm-content')).toBeTruthy()
    // 自动补 Content-Type: application/json（切到 Headers Tab，输入框值可见）
    const headersTab = Array.from(document.querySelectorAll('.drw-tab')).find((b) => b.textContent === 'Headers')
    ;(headersTab as HTMLButtonElement).click()
    await vi.waitFor(() => {
      const keys = Array.from(
        document.querySelectorAll<HTMLInputElement>('.kvt-row .kvt-key'),
      ).map((i) => i.value)
      expect(keys).toContain('Content-Type')
    })
    // 切到 GET → 默认 Params Tab，Body Tab 出现「不携带 Body」提示
    const methodTrigger = document.querySelector<HTMLElement>('.drw-method .cs-trigger')
    ;(methodTrigger as HTMLElement).click()
    await vi.waitFor(() => {
      const opt = Array.from(document.querySelectorAll('.cs-opt')).find(
        (el) => el.textContent?.trim() === 'GET',
      )
      expect(opt).toBeTruthy()
      ;(opt as HTMLElement).click()
    })
    await vi.waitFor(() => {
      const active = document.querySelector<HTMLElement>('.drw-tab.active')
      expect(active?.textContent).toBe('Params')
    })
    const bodyTab = Array.from(document.querySelectorAll('.drw-tab')).find((b) => b.textContent === 'Body')
    ;(bodyTab as HTMLButtonElement).click()
    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('GET 请求通常不携带 Body')
    })
  })

  it('请求区 / 响应区存在拖拽分割条，双击恢复 50% 比例', async () => {
    mountDrawer()
    const splitter = document.querySelector<HTMLElement>('.drw-splitter')
    expect(splitter).toBeTruthy()
    const reqSec = document.querySelector<HTMLElement>('.drw-req-sec')
    expect(reqSec?.style.flexBasis).toBe('55%')
    ;(splitter as HTMLElement).dispatchEvent(new MouseEvent('dblclick'))
    await new Promise((r) => setTimeout(r, 0))
    expect(reqSec?.style.flexBasis).toBe('50%')
  })

  it('「保存修改」携带完整配置（含名称 / 分组），保存成功后关闭', async () => {
    const { wrapper, onSave } = mountDrawer()
    const saveBtn = Array.from(document.querySelectorAll('.drw-foot button')).find(
      (b) => b.textContent?.trim() === '保存修改',
    ) as HTMLButtonElement
    expect(saveBtn.disabled).toBe(false)
    saveBtn.click()
    await vi.waitFor(() => {
      expect(onSave).toHaveBeenCalledWith(
        expect.objectContaining({
          name: '内部划转-SGB',
          category: '正向',
          method: 'POST',
          urlPath: '/funds/transfer',
          bodyType: 'json',
          bodyContent: '{"amount":100}',
        }),
      )
    })
    expect(wrapper.emitted('update:open')).toBeTruthy()
  })

  it('「立即运行」调用 onRun 并展示响应状态与 Body（Success）', async () => {
    const { onRun } = mountDrawer()
    onRun.mockResolvedValue({
      status: 200,
      headers: [],
      body: '{"ok":true}',
      content_type: 'application/json',
      duration_ms: 320,
      size_bytes: 10,
      truncated: false,
    })
    const runBtn = Array.from(document.querySelectorAll('.drw-foot button')).find(
      (b) => b.textContent?.includes('立即运行'),
    ) as HTMLButtonElement
    runBtn.click()
    await vi.waitFor(() => {
      expect(onRun).toHaveBeenCalledWith(
        expect.objectContaining({ method: 'POST', urlPath: '/funds/transfer' }),
      )
    })
    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('200 OK')
      expect(document.body.textContent).toContain('320ms')
      expect(document.body.textContent).toContain('"ok": true')
    })
  })

  it('运行失败展示 Failed 与错误信息', async () => {
    const { onRun } = mountDrawer()
    onRun.mockResolvedValue({
      status: 500,
      headers: [],
      body: 'boom',
      content_type: 'text/plain',
      duration_ms: 12,
      size_bytes: 4,
      truncated: false,
    })
    const runBtn = Array.from(document.querySelectorAll('.drw-foot button')).find(
      (b) => b.textContent?.includes('立即运行'),
    ) as HTMLButtonElement
    runBtn.click()
    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('500 Internal Error')
      expect(document.body.textContent).toContain('12ms')
    })
  })

  it('名称 / Path 为空时保存按钮禁用', async () => {
    const c = makeCase()
    c.name = ''
    mountDrawer({ testCase: c })
    const saveBtn = Array.from(document.querySelectorAll('.drw-foot button')).find(
      (b) => b.textContent?.trim() === '保存修改',
    ) as HTMLButtonElement
    expect(saveBtn.disabled).toBe(true)
  })
})