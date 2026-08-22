/**
 * schemaInfer：从 JSON 样本推断参数树（文档预览的 Tree Table 数据源）。
 *
 * - 输入：请求 Body / 响应示例的 JSON 解析结果（unknown）；
 * - 输出：扁平可递归的 SchemaRow 树（字段名 / 类型 / 示例值 / 子字段）；
 * - 数组取首个非空元素推断元素类型（itemType），object 数组继续下钻一层；
 * - 「必填」语义：由样本推断的字段一律视为必填（出现于示例），
 *   调用方可按 KeyValue.enabled 覆盖（urlencoded / multipart 字段）。
 */

/** 字段类型（JSON Schema 子集）。 */
export type SchemaType = 'string' | 'number' | 'boolean' | 'object' | 'array' | 'null'

/** 树状表格的一行（嵌套对象 / 数组元素下钻为 children）。 */
export interface SchemaRow {
  /** 字段名（数组元素行固定为空串，展示层渲染为 `items[]`）。 */
  name: string
  type: SchemaType
  /** 数组元素类型（type === 'array' 时有效）。 */
  itemType?: SchemaType
  /** 叶子字段的示例值序列化（截断展示）。 */
  example: string
  /** 层级深度（根字段为 0）。 */
  depth: number
  /** 是否必填（样本推断 = true）。 */
  required: boolean
  /** 嵌套子字段（object 的属性 / object 数组的元素字段）。 */
  children: SchemaRow[]
}

/** 示例值序列化的最大长度（超长截断加省略号）。 */
const EXAMPLE_MAX_LEN = 42

/** 值 → JSON Schema 类型。 */
function typeOf(value: unknown): SchemaType {
  if (value === null) return 'null'
  if (Array.isArray(value)) return 'array'
  if (typeof value === 'object') return 'object'
  if (typeof value === 'number') return 'number'
  if (typeof value === 'boolean') return 'boolean'
  return 'string'
}

/** 示例值 → 短文本（字符串去引号，其余 JSON 序列化，超长截断）。 */
export function exampleText(value: unknown): string {
  let text: string
  if (typeof value === 'string') {
    text = value
  } else {
    try {
      text = JSON.stringify(value) ?? String(value)
    } catch {
      text = String(value)
    }
  }
  if (text.length > EXAMPLE_MAX_LEN) {
    // 按字符截断（中文安全），预留省略号 3 字符。
    return `${Array.from(text).slice(0, EXAMPLE_MAX_LEN - 3).join('')}...`
  }
  return text
}

/** 数组 → 元素类型（取首个非 null 元素；空数组 / 全 null 返回 undefined）。 */
function arrayItemType(value: unknown[]): SchemaType | undefined {
  const first = value.find((v) => v !== null)
  return first === undefined ? undefined : typeOf(first)
}

/** 单行推断：值 → 代表该值的行（object / object 数组的子字段下钻为 children）。 */
function inferRow(name: string, value: unknown, depth: number): SchemaRow {
  const t = typeOf(value)
  if (t === 'object') {
    const children = Object.entries(value as Record<string, unknown>).map(([key, v]) =>
      inferRow(key, v, depth + 1),
    )
    return { name, type: 'object', example: '', depth, required: true, children }
  }
  if (t === 'array') {
    const arr = value as unknown[]
    const itemType = arrayItemType(arr)
    const first = arr.find((v) => v !== null)
    // object 数组：取首个元素的属性下钻一层（展示层渲染 items[]）。
    const children =
      itemType === 'object' && first !== undefined
        ? Object.entries(first as Record<string, unknown>).map(([key, v]) =>
            inferRow(key, v, depth + 1),
          )
        : []
    return { name, type: 'array', itemType, example: '', depth, required: true, children }
  }
  return { name, type: t, example: exampleText(value), depth, required: true, children: [] }
}

/** 根节点推断：object 的每个属性一行；数组 / 标量包装为单行。 */
export function inferSchema(sample: unknown): SchemaRow[] {
  const t = typeOf(sample)
  if (t === 'object') {
    return Object.entries(sample as Record<string, unknown>).map(([name, v]) =>
      inferRow(name, v, 0),
    )
  }
  // 根为数组 / 标量：整体作为一行展示（name 空串由展示层渲染）。
  return [inferRow('', sample, 0)]
}

/** 展示用类型标签：数组带元素类型（`array<object>`），其余原样。 */
export function typeLabelOf(row: Pick<SchemaRow, 'type' | 'itemType'>): string {
  if (row.type === 'array') {
    return `array<${row.itemType ?? 'any'}>`
  }
  return row.type
}
