import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ResponsePanel from './ResponsePanel.vue'
import type { ExecuteResponse } from '../types/foxApi'
import { collectErrors, stubScrollIntoView } from '../testUtils/componentTest'

function makeResponse(body: string): ExecuteResponse {
  return {
    status: 201,
    headers: [['content-type', 'application/json']],
    body,
    content_type: 'application/json',
    duration_ms: 769,
    size_bytes: 83,
    truncated: false,
  }
}

/** 通过 ⌘F 打开查找条并输入关键词。 */
async function openFind(wrapper: ReturnType<typeof mount>): Promise<void> {
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'f', metaKey: true }))
  await wrapper.vm.$nextTick()
  const input = wrapper.find('.findbar-input')
  expect(input.exists()).toBe(true)
  await input.setValue('id')
  await flushPromises()
}

describe('ResponsePanel：状态栏元数据', () => {
  it('渲染 状态码 / 耗时 / 大小，且带 tone-ok 强调', () => {
    const wrapper = mount(ResponsePanel, { props: { response: makeResponse('{}') } })
    expect(wrapper.find('.rp').classes()).toContain('tone-ok')
    expect(wrapper.text()).toContain('201 Created')
    expect(wrapper.text()).toContain('769.00 ms')
    expect(wrapper.text()).toContain('83 B')
    expect(wrapper.text()).toContain('application/json')
    wrapper.unmount()
  })

  it('4xx/5xx 状态呈现 err 强调', () => {
    const resp = makeResponse('{}')
    resp.status = 500
    const wrapper = mount(ResponsePanel, { props: { response: resp } })
    expect(wrapper.find('.rp').classes()).toContain('tone-err')
    expect(wrapper.text()).toContain('Internal Server Error')
    wrapper.unmount()
  })
})

describe('ResponsePanel：查找（Find in Response）', () => {
  beforeEach(() => {
    stubScrollIntoView()
  })
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('⌘F 打开查找条 → 高亮全部 + n/N 计数 → 上一个/下一个切换 → Esc 关闭并清空', async () => {
    const wrapper = mount(ResponsePanel, {
      props: { response: makeResponse(JSON.stringify({ users: [{ id: 1 }, { id: 2 }] })) },
    })

    await openFind(wrapper)
    expect(wrapper.text()).toContain('1 / 2')
    expect(wrapper.findAll('.jt-mark')).toHaveLength(2)
    expect(wrapper.findAll('.jt-mark.active')).toHaveLength(1)

    // 下一个（第 2 个按钮） → 2 / 2
    await wrapper.findAll('.findbar-btn')[1].trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('2 / 2')
    expect(wrapper.findAll('.jt-mark.active')).toHaveLength(1)

    // 上一个（第 1 个按钮） → 回到 1 / 2
    await wrapper.findAll('.findbar-btn')[0].trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('1 / 2')

    // Esc 关闭并清空搜索词
    await wrapper.find('.findbar-input').trigger('keydown', { key: 'Escape' })
    await fallbackTick()
    expect(wrapper.find('.findbar').exists()).toBe(false)
    expect(wrapper.findAll('.jt-mark')).toHaveLength(0)
    wrapper.unmount()
  })

  it('无匹配时显示「无匹配」且高亮为零', async () => {
    const wrapper = mount(ResponsePanel, {
      props: { response: makeResponse(JSON.stringify({ users: [{ id: 1 }] })) },
    })
    await openFind(wrapper)
    await wrapper.find('.findbar-input').setValue('zzz')
    await flushPromises()
    expect(wrapper.text()).toContain('无匹配')
    expect(wrapper.findAll('.jt-mark')).toHaveLength(0)
    wrapper.unmount()
  })

  it('输入其他组件（如文本框）时 ⌘F 不拦截', async () => {
    const wrapper = mount(ResponsePanel, {
      props: { response: makeResponse(JSON.stringify({ id: 1 })) },
    })
    document.body.appendChild(createInput())
    const input = document.body.querySelector('input.fake') as HTMLInputElement
    input.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'f', metaKey: true, bubbles: true }),
    )
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.findbar').exists()).toBe(false)
    input.remove()
    wrapper.unmount()
  })
})

describe('ResponsePanel：展开全部 / 收起全部', () => {
  it('切换 JSON 树全部节点的展开状态', async () => {
    const wrapper = mount(ResponsePanel, {
      props: { response: makeResponse(JSON.stringify({ a: { b: { c: 1 } } })) },
    })
    expect(wrapper.find('.jt-line').exists()).toBe(true)

    // 展开/收起已合并为单个切换图标按钮（操作区第 2 个，前为查找）
    const toggleBtn = wrapper.findAll('.rp-icon-btn')[1]

    await toggleBtn.trigger('click')
    await wrapper.vm.$nextTick()
    expect(wrapper.text()).not.toMatch(/…\s+\d+\s+项/)

    await wrapper.findAll('.rp-icon-btn')[1].trigger('click')
    await wrapper.vm.$nextTick()
    expect(wrapper.text()).toMatch(/…\s+\d+\s+项/)
    wrapper.unmount()
  })
})

describe('ResponsePanel：卸载间隙稳定性（回归：emitsOptions 崩溃类）', () => {
  beforeEach(() => {
    stubScrollIntoView()
  })

  it('查找导航后立即卸载 / 重新挂载，不产生未处理错误', async () => {
    const collector = collectErrors()
    for (let i = 0; i < 3; i++) {
      const wrapper = mount(ResponsePanel, {
        props: { response: makeResponse(JSON.stringify({ users: [{ id: 1 }, { id: 2 }] })) },
      })
      await openFind(wrapper)
      await wrapper.findAll('.findbar-btn')[1].trigger('click')
      wrapper.unmount()
      await flushPromises()
    }
    expect(collector.errors).toEqual([])
    collector.restore()
  })
})

function createInput(): HTMLInputElement {
  const el = document.createElement('input')
  el.className = 'fake'
  return el
}

/** jsdom 下额外推一帧，确保 Teleport/Transition 缓冲期完成。 */
async function fallbackTick(): Promise<void> {
  await flushPromises()
  await new Promise((r) => setTimeout(r, 0))
}