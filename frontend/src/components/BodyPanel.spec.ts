/**
 * BodyPanel 单测：raw JSON 编辑 + 格式化的数据流（回归：格式化使用了修改前的旧内容）。
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { reactive } from 'vue'
import BodyPanel from './BodyPanel.vue'
import { makeDraft } from '../testUtils/draftFixture'
import type { Endpoint } from '../types/foxApi'

function jsonDraft(raw: string): Endpoint {
  const d = makeDraft()
  d.request.body = { mode: 'json', raw }
  return reactive(d)
}

/** 读取草稿 json body 的 raw（联合类型窄化辅助）。 */
function rawOf(d: Endpoint): string {
  return (d.request.body as { raw: string }).raw
}

function mountPanel(draft: Endpoint) {
  return mount(BodyPanel, { props: { draft } })
}

/** 格式化按钮已改为纯图标（title 提示），按 title 定位。 */
function formatBtnOf(wrapper: ReturnType<typeof mountPanel>) {
  return wrapper.findAll('button').find((b) => b.attributes('title') === '格式化 JSON')
}

describe('BodyPanel：raw JSON 编辑与格式化', () => {
  it('编辑后点击「格式化」应格式化最新内容', async () => {
    const draft = jsonDraft('{"a":1}')
    const wrapper = mountPanel(draft)

    // raw + JSON 子类型 → 渲染 JsonEditor（工具条含「格式化」按钮）
    const formatBtn = formatBtnOf(wrapper)
    expect(formatBtn).toBeDefined()
    expect(wrapper.findComponent({ name: 'JsonEditor' }).exists()).toBe(true)

    // 模拟用户编辑 textarea（input → v-model 写回草稿）
    const ta = wrapper.find('textarea')
    await ta.setValue('{"name":"alice","age":18}')
    expect(rawOf(draft)).toBe('{"name":"alice","age":18}')

    await formatBtn!.trigger('click')
    expect(rawOf(draft)).toBe(
      JSON.stringify({ name: 'alice', age: 18 }, null, 2),
    )
  })

  it('紧凑模式格式化同样使用最新内容', async () => {
    const draft = jsonDraft('{ "a": 1 }')
    const wrapper = mountPanel(draft)
    const ta = wrapper.find('textarea')
    await ta.setValue('{ "b": 2, "c": 3 }')
    await formatBtnOf(wrapper)!.trigger('click')
    expect(rawOf(draft)).toBe('{\n  "b": 2,\n  "c": 3\n}')
  })

  it('回归：格式化保留重复键（多个 "body" 键不丢失）', async () => {
    const draft = jsonDraft(
      '{"title":"测试标题","body":"测试内容","body":"测试内容","body":"测试内容","body":"测试内容","userId":1}',
    )
    const wrapper = mountPanel(draft)
    await formatBtnOf(wrapper)!.trigger('click')
    expect(rawOf(draft)).toBe(`{
  "title": "测试标题",
  "body": "测试内容",
  "body": "测试内容",
  "body": "测试内容",
  "body": "测试内容",
  "userId": 1
}`)
  })

  it('回归：DOM 已更新但 input 事件丢失时，格式化仍以 textarea 实际内容为准', async () => {
    const draft = jsonDraft('{"a":1}')
    const wrapper = mountPanel(draft)
    const ta = wrapper.find('textarea')

    // 直接改 DOM 值、不派发 input 事件（模拟真实浏览器中事件丢失/延迟的失同步），
    // 此时 props.modelValue 仍是旧值 {"a":1}
    ta.element.value = '{"name":"alice","age":18}'
    await formatBtnOf(wrapper)!.trigger('click')

    expect(rawOf(draft)).toBe(
      JSON.stringify({ name: 'alice', age: 18 }, null, 2),
    )
  })
})
