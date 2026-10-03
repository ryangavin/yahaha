// Screenshots one UI library component's stories, in dark and light, and scores each against its
// board crop. A local tool for the factory's Inspect station, not part of CI.
//
//   npm run shots -- <Name> [--url http://localhost:6006]
//
// Writes app/.shots/<Name>/<story>-<theme>.png and, where app/src/ui/<Name>/crops/<story>-<theme>.png
// exists, <story>-<theme>.diff.png and a score in report.json. It also runs axe (colour contrast
// included, which jsdom can't check) on every story in real Chrome. Exits 1 when a cropped story
// doesn't match its crop (a different size, or a score over PASS_SCORE) or any story has an axe
// violation. Without --url it builds Storybook and serves the build itself. It drives the
// system Chrome (playwright-core, no browser download); set CHROME to use another binary.
// Chrome, the server and the build are stopped on every exit path.

import { spawn, type ChildProcess } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer, type Server } from 'node:http'
import { extname, join, normalize, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium, type Browser } from 'playwright-core'
import { PNG } from 'pngjs'
import { PASS_SCORE, diffImages, matches } from './shots-diff.ts'

const APP = resolve(fileURLToPath(import.meta.url), '../..')
const AXE = join(APP, 'node_modules/axe-core/axe.min.js')
const THEMES = ['dark', 'light'] as const

type Entry = { id: string; type: string; importPath: string; exportName?: string; name: string }
type Result = {
  story: string
  theme: string
  shot: string
  crop?: string
  diff?: string
  size: [number, number]
  cropSize?: [number, number]
  score?: number
  /** Same size as the crop and score at most PASS_SCORE; absent without a crop. */
  matchesCrop?: boolean
  /** axe violations, as `rule: help (n nodes)`. */
  a11y: string[]
  pass: boolean
}

// Everything started here, stopped by cleanup() on every exit path.
let build: ChildProcess | undefined
let server: Server | undefined
let browser: Browser | undefined

async function cleanup() {
  if (build && build.exitCode === null) build.kill('SIGTERM')
  build = undefined
  const b = browser
  browser = undefined
  await b?.close().catch(() => {})
  const s = server
  server = undefined
  s?.closeAllConnections()
  await new Promise<void>((done) => (s ? s.close(() => done()) : done()))
}

for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP'] as const) {
  process.on(signal, () => {
    void cleanup().finally(() => process.exit(130))
  })
}

function usage(message: string): never {
  console.error(`${message}\nUsage: npm run shots -- <Name> [--url <storybook url>]`)
  process.exit(2)
}

function parseArgs(argv: string[]): { name: string; url?: string } {
  let name: string | undefined
  let url: string | undefined
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--url') url = argv[++i]
    else if (!argv[i].startsWith('-')) name = argv[i]
    else usage(`Unknown option ${argv[i]}`)
  }
  if (!name) usage('Name a component.')
  if (!/^[A-Z]\w*$/.test(name) || !existsSync(join(APP, 'src/ui', name))) usage(`No component src/ui/${name}.`)
  return { name, url: url?.replace(/\/$/, '') }
}

/** Builds Storybook into a cache folder; resolves with the folder. */
function buildStorybook(): Promise<string> {
  const out = join(APP, 'node_modules/.cache/shots-storybook')
  console.log('Building Storybook…')
  return new Promise((done, fail) => {
    build = spawn('npx', ['storybook', 'build', '--quiet', '--disable-telemetry', '-o', out], {
      cwd: APP,
      stdio: ['ignore', 'ignore', 'inherit'],
    })
    build.on('error', fail)
    build.on('exit', (code) => {
      build = undefined
      if (code === 0) done(out)
      else fail(new Error(`storybook build exited with ${code}`))
    })
  })
}

const TYPES: Record<string, string> = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.mjs': 'text/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
}

/** Serves a static folder on a free localhost port; resolves with its URL. */
function serve(root: string): Promise<string> {
  return new Promise((done) => {
    server = createServer((req, res) => {
      const path = normalize(decodeURIComponent(new URL(req.url ?? '/', 'http://x').pathname))
      const file = join(root, path.endsWith('/') ? `${path}index.html` : path)
      if (!file.startsWith(root) || !existsSync(file)) {
        res.writeHead(404).end()
        return
      }
      res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' })
      res.end(readFileSync(file))
    })
    server.listen(0, '127.0.0.1', () => {
      const address = server?.address()
      done(`http://127.0.0.1:${typeof address === 'object' && address ? address.port : 0}`)
    })
  })
}

async function shoot(name: string, base: string): Promise<Result[]> {
  const index = (await (await fetch(`${base}/index.json`)).json()) as { entries: Record<string, Entry> }
  const stories = Object.values(index.entries).filter(
    (e) => e.type === 'story' && e.importPath.includes(`/src/ui/${name}/`),
  )
  if (stories.length === 0) throw new Error(`No stories for ${name} in ${base}`)

  const outDir = join(APP, '.shots', name)
  rmSync(outDir, { recursive: true, force: true })
  mkdirSync(outDir, { recursive: true })
  const crops = join(APP, 'src/ui', name, 'crops')

  browser = await chromium.launch(
    process.env.CHROME ? { executablePath: process.env.CHROME } : { channel: 'chrome' },
  )
  const page = await browser.newPage({ viewport: { width: 1000, height: 600 }, deviceScaleFactor: 1 })
  const results: Result[] = []
  for (const entry of stories) {
    const story = entry.exportName ?? entry.name.replace(/\s+/g, '')
    for (const theme of THEMES) {
      await page.goto(`${base}/iframe.html?id=${entry.id}&viewMode=story&globals=theme:${theme};backgrounds.value:${theme}`)
      // The theme frame (preview.ts) has no box of its own; the story's root is its first child.
      const root = page.locator('#storybook-root > [data-theme] > *').first()
      await root.waitFor({ state: 'visible' })
      // Wait for the story's play function and checks to finish, then drop any focus the play
      // left behind: a shot shows the story's resting look (a focus story uses pseudo-states).
      const phase = await page.waitForFunction(() => {
        const render = (window as { __STORYBOOK_PREVIEW__?: { currentRender?: { phase?: string } } })
          .__STORYBOOK_PREVIEW__?.currentRender
        return ['completed', 'finished', 'errored', 'aborted'].includes(render?.phase ?? '') && render?.phase
      })
      if ((await phase.jsonValue()) === 'errored') throw new Error(`${story} (${theme}): the story errored`)
      await page.evaluate(async () => {
        ;(document.activeElement as HTMLElement | null)?.blur()
        await document.fonts.ready
        await new Promise(requestAnimationFrame)
      })
      const actual = await page.evaluate(
        () => document.querySelector<HTMLElement>('#storybook-root > [data-theme]')?.dataset.theme,
      )
      if (actual !== theme) throw new Error(`${story}: theme is ${actual}, expected ${theme}`)

      // Chrome snaps a box's background to whole pixels but not its text, so a story centred at a
      // fractional x renders its text differently from the board. Pin the story to the canvas
      // padding's whole-pixel corner, and clip to the snapped box.
      await page.addStyleTag({
        content: '.sb-main-centered { display: block !important } .sb-main-centered #storybook-root { margin: 0 !important }',
      })
      // A visible outline (a focus ring) is part of the shot.
      const box = await root.evaluate((el) => {
        const r = el.getBoundingClientRect()
        const style = getComputedStyle(el)
        const ring =
          style.outlineStyle === 'none' ? 0 : Math.max(0, parseFloat(style.outlineWidth) + parseFloat(style.outlineOffset))
        const x = Math.round(r.left - ring)
        const y = Math.round(r.top - ring)
        return { x, y, width: Math.round(r.right + ring) - x, height: Math.round(r.bottom + ring) - y }
      })
      const file = `${story}-${theme}.png`
      const shotBuffer = await page.screenshot({ path: join(outDir, file), clip: box, animations: 'disabled' })
      const shot = PNG.sync.read(shotBuffer)

      // axe on the rendered story, colour contrast included.
      await page.addScriptTag({ path: AXE })
      const a11y = await page.evaluate(async () => {
        type Axe = { run: (context: string) => Promise<{ violations: { id: string; help: string; nodes: unknown[] }[] }> }
        const { violations } = await (window as unknown as { axe: Axe }).axe.run('#storybook-root')
        return violations.map((v) => `${v.id}: ${v.help} (${v.nodes.length} nodes)`)
      })

      const result: Result = { story, theme, shot: file, size: [shot.width, shot.height], a11y, pass: a11y.length === 0 }
      const cropPath = join(crops, file)
      if (existsSync(cropPath)) {
        const crop = PNG.sync.read(readFileSync(cropPath))
        const diff = diffImages(shot, crop)
        const diffFile = `${story}-${theme}.diff.png`
        writeFileSync(join(outDir, diffFile), PNG.sync.write(diff.image))
        Object.assign(result, {
          crop: `src/ui/${name}/crops/${file}`,
          diff: diffFile,
          cropSize: [crop.width, crop.height],
          score: Number(diff.score.toFixed(4)),
          matchesCrop: matches(diff),
        })
        result.pass &&= matches(diff)
      }
      results.push(result)
      const scored =
        result.score === undefined
          ? 'no crop'
          : `score ${result.score}${result.matchesCrop ? '' : result.cropSize?.join('×') === result.size.join('×') ? ' over' : ' wrong size'}`
      const verdict = result.pass ? 'pass' : 'FAIL'
      console.log(`${file.padEnd(28)} ${result.size.join('×').padEnd(9)} ${scored.padEnd(24)} ${verdict}`)
      for (const v of a11y) console.log(`    a11y ${v}`)
    }
  }
  writeFileSync(
    join(outDir, 'report.json'),
    `${JSON.stringify({ component: name, passScore: PASS_SCORE, stories: results }, null, 2)}\n`,
  )
  console.log(`Wrote ${join('.shots', name)}/ (report.json)`)
  return results
}

async function main() {
  const { name, url } = parseArgs(process.argv.slice(2))
  let results: Result[]
  try {
    const base = url ?? (await serve(await buildStorybook()))
    results = await shoot(name, base)
  } finally {
    await cleanup()
  }
  process.exit(results.every((r) => r.pass) ? 0 : 1)
}

main().catch(async (error) => {
  console.error(error instanceof Error ? error.message : error)
  await cleanup()
  process.exit(1)
})
