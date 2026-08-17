import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import CodeExportDialog from './CodeExportDialog.vue'
import { collectErrors } from '../testUtils/componentTest'
import { makeDraft } from '../testUtils/draftFixture'

const codegenRender = vi.fn()

vi.mock('../composables/useFoxApi', () => ({
  useFoxApi: () => ({
    pending: { value: false },
    codegenRender,
  }),
}))

function dialogBody(): Element | null {
  return document.body.querySelector('.m-dialog')
}

describe('CodeExportDialog：导出接口代码', () => {
  afterEach(() => {
    codegenRender.mockReset()
    document.body.innerHTML = ''
  })

  it('打开即自动生成并可关闭', async () => {
    codegenRender.mockResolvedValue('curl -X GET "https://api.example.com"')
    const wrapper = mount(CodeExportDialog, {
      props: { draft: makeDraft(), url: 'https://api.example.com' },
      attachTo: document.body,
    })
    await flushPromises()
    expect(codegenRender).toHaveBeenCalledTimes(1)
    expect(dialogBody()?.querySelector('.m-title')?.textContent).toContain('导出接口代码')
    expect(dialogBody()?.querySelector('.cp-preview')?.textContent).toContain('curl')

    // Modal 通过 Teleport 渲染到 body，需从 document 层面点击关闭按钮。
    const closeBtn = dialogBody()?.querySelector<HTMLButtonElement>('.m-foot .rf-btn-primary')
    closeBtn?.click()
    await flushPromises()
    expect(wrapper.emitted('close')).toBeTruthy()
    wrapper.unmount()
  })

  it('生成未返回时关闭弹窗（立即卸载）不产生未处理错误', async () => {
    const collector = collectErrors()
    let resolve!: (v: string) => void
    codegenRender.mockImplementation(() => new Promise((r) => (resolve = r)))

    const wrapper = mount(CodeExportDialog, {
      props: { draft: makeDraft(), url: 'https://api.example.com' },
      attachTo: document.body,
    })
    await flushPromises()
    wrapper.unmount()
    resolve('late')
    await flushPromises()

    expect(collector.errors).toEqual([])
    collector.restore()
  })
})