/** Every key path in `v` with its value kind ("transport.lamps[].action.type: string");
 * arrays contribute their first element. Used to compare the mock with the engine's JSON. */
export function shape(v: unknown, path = ''): string[] {
  if (Array.isArray(v)) return v.length ? shape(v[0], `${path}[]`) : []
  if (v && typeof v === 'object') {
    return Object.entries(v).flatMap(([k, x]) => {
      const p = path ? `${path}.${k}` : k
      const kind = x === null ? 'null' : Array.isArray(x) ? 'array' : typeof x
      return [`${p}: ${kind}`, ...shape(x, p)]
    })
  }
  return []
}
