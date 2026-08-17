import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import JsonTree from './JsonTree.vue'
import { collectErrors, stubScrollIntoView } from '../testUtils/componentTest'

const DATA = {
  users: [
    { id: 1, name: 'fox' },
    { id: 2, role: 'admin' },
  ],
}

/** 折叠摘要行的形态：`… 1 项`（token：` … ` + `N 项`）。用 text() 避免中间标签干扰。 */
function expectCollapsed(wrapper: { text: () => string }, collapsed: boolean): void {
  if (collapsed) {
    expect(wrapper.text()).toMatch(/…\s+\d+\s+项/)
  } else {
    expect(wrapper.text()).not.toMatch(/…\s+\d+\s+项/)
  }
}

describe('JsonTree：折叠 / 展开全部', () => {
  it('expandDepth=0 时容器默认收起；collapseAll 后再 expandAll 恢复展开', async () => {
    const wrapper = mount(JsonTree, { props: { data: DATA, expandDepth: 0 } })
    expectCollapsed(wrapper, true)

    const tree = wrapper.findComponent(JsonTree)
    ;(tree.vm as unknown as { expandAll: () => void }).expandAll()
    await wrapper.vm.$nextTick()
    expectCollapsed(wrapper, false)

    ;(tree.vm as unknown as { collapseAll: () => void }).collapseAll()
    await wrapper.vm.$nextTick()
    expectCollapsed(wrapper, true)
    wrapper.unmount()
  })
})

describe('JsonTree：查找高亮与导航', () => {
  beforeEach(() => {
    stubScrollIntoView()
  })
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('搜索时强制展开全部并上抛匹配总数', async () => {
    const onCount = vi.fn()
    const wrapper = mount(JsonTree, {
      props: { data: DATA, expandDepth: 0, query: 'id' },
      attrs: { 'onMatch-count': onCount },
    })
    await flushPromises()
    // 两个 "id" 键匹配；expandDepth=0 也强制全展开（无折叠摘要）。
    expect(onCount).toHaveBeenCalledWith(2)
    expect(wrapper.text()).not.toMatch(/…\s+\d+\s+项/)
    expect(wrapper.findAll('.jt-mark')).toHaveLength(2)
    expect(wrapper.findAll('.jt-mark.active')).toHaveLength(1)
    wrapper.unmount()
  })

  it('activeMatch 切换时当前高亮移动并触发滚动', async () => {
    const scrollFn = vi.fn()
    const original = HTMLElement.prototype.scrollIntoView
    HTMLElement.prototype.scrollIntoView = scrollFn as never
    try {
      const wrapper = mount(JsonTree, { props: { data: DATA, query: 'id', activeMatch: 0 } })
      await flushPromises()
      expect(wrapper.find('.jt-mark.active').text()).toBe('id')

      await wrapper.setProps({ activeMatch: 1 })
      expect(wrapper.findAll('.jt-mark.active')).toHaveLength(1)
      expect(wrapper.find('.jt-mark.active').text()).toBe('id')
      expect(scrollFn).toHaveBeenCalled()
      wrapper.unmount()
    } finally {
      HTMLElement.prototype.scrollIntoView = original
    }
  })

  it('无匹配时为零计数；清除 query 后无高亮残留', async () => {
    const onCount = vi.fn()
    const wrapper = mount(JsonTree, {
      props: { data: DATA, query: 'zzz-not-exist' },
      attrs: { 'onMatch-count': onCount },
    })
    await flushPromises()
    expect(onCount).toHaveBeenLastCalledWith(0)
    expect(wrapper.findAll('.jt-mark')).toHaveLength(0)

    await wrapper.setProps({ query: '', activeMatch: 0 })
    expect(wrapper.findAll('.jt-mark')).toHaveLength(0)
    wrapper.unmount()
  })
})

describe('JsonTree：卸载间隙稳定性（回归：emitsOptions 崩溃类）', () => {
  beforeEach(() => {
    stubScrollIntoView()
  })

  it('查找导航后立即卸载组件，不产生任何未处理错误', async () => {
    const collector = collectErrors()
    const wrapper = mount(JsonTree, { props: { data: DATA, query: 'id' } })
    await wrapper.setProps({ activeMatch: 1 })
    await flushPromises()
    wrapper.unmount()
    await flushPromises()
    expect(collector.errors).toEqual([])
    collector.restore()
  })

  it('快速挂载/卸载多个实例（模拟条件渲染竞态）不崩溃', async () => {
    const collector = collectErrors()
    for (let i = 0; i < 5; i++) {
      const w = mount(JsonTree, { props: { data: DATA, query: i % 2 ? 'id' : 'name' } })
      await w.setProps({ activeMatch: 1 })
      await flushPromises()
      w.unmount()
      await flushPromises()
    }
    expect(collector.errors).toEqual([])
    collector.restore()
  })
})