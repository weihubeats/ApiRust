import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import CodePanel from './CodePanel.vue'
import CustomSelect from './ui/CustomSelect.vue'
import { collectErrors } from '../testUtils/componentTest'
import { makeDraft } from '../testUtils/draftFixture'

const codegenRender = vi.fn()

vi.mock('../composables/useFoxApi', () => ({
  useFoxApi: () => ({
    pending: { value: false },
    codegenRender,
  }),
}))

describe('CodePanel：自动生成（autoGenerate）', () => {
  afterEach(() => {
    codegenRender.mockReset()
  })

  it('默认不自动生成；autoGenerate 在挂载时自动生成一次', async () => {
    codegenRender.mockResolvedValue('curl -X GET "https://api.example.com"')
    const w1 = mount(CodePanel, { props: { draft: makeDraft(), url: 'https://api.example.com' } })
    await flushPromises()
    expect(codegenRender).not.toHaveBeenCalled()

    const w2 = mount(CodePanel, {
      props: { draft: makeDraft(), url: 'https://api.example.com', autoGenerate: true },
    })
    await flushPromises()
    expect(codegenRender).toHaveBeenCalledTimes(1)
    expect(w2.find('.cp-preview').text()).toContain('curl')
    w1.unmount()
    w2.unmount()
  })

  it('autoGenerate 切换语言自动重新生成', async () => {
    codegenRender.mockResolvedValue('code')
    const wrapper = mount(CodePanel, {
      props: { draft: makeDraft(), url: 'https://api.example.com', autoGenerate: true },
    })
    await flushPromises()
    expect(codegenRender).toHaveBeenCalledTimes(1)

    const select = wrapper.findComponent(CustomSelect)
    select.vm.$emit('update:modelValue', 'java')
    await flushPromises()
    expect(codegenRender).toHaveBeenCalledTimes(2)
    // 语言参数随调用携带
    expect(codegenRender.mock.calls[1][0].lang).toBe('java')
    wrapper.unmount()
  })

  it('卸载时丢弃在途生成结果，不产生未处理错误', async () => {
    const collector = collectErrors()
    let resolve!: (v: string) => void
    codegenRender.mockImplementation(() => new Promise((r) => (resolve = r)))

    const wrapper = mount(CodePanel, {
      props: { draft: makeDraft(), url: 'https://api.example.com', autoGenerate: true },
    })
    await flushPromises()
    wrapper.unmount()
    resolve('late-result')
    await flushPromises()

    expect(collector.errors).toEqual([])
    collector.restore()
  })
})