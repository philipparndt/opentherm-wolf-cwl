import { useState, useEffect } from 'preact/hooks'
import { getHistory, type History, type HistoryBucket } from './api'
import { t, type Lang } from './translations'

// Off / Reduced / Normal / Party — same order & colours as the rest of the UI.
const LEVEL_COLORS = ['var(--accent-red)', 'var(--accent-orange)', 'var(--accent-blue)', 'var(--accent-green)'] as const

// Marker dot colour by the reason the level changed.
const REASON_COLORS: Record<string, string> = {
  temp: 'var(--accent-blue)',
  cooling: 'var(--accent-green)',
  dehumidify: '#22c3c3',
  muggy: 'var(--accent-orange)',
  manual: 'var(--text-muted)',
  schedule: '#a78bfa',
  reboot: '#e2e8f0',
}

// Series colours. Temperature on the left axis, humidity + enthalpy on their own
// right-hand axes — each pair distinct so the busier chart stays readable.
const COL_SUPPLY = 'var(--accent-orange)'
const COL_EXHAUST = 'var(--accent-blue)'
const COL_INDOOR_HUM = '#22c3c3'
const COL_OUTDOOR_HUM = '#a78bfa'
const COL_INDOOR_ENTH = '#f43f5e'
const COL_OUTDOOR_ENTH = '#84cc16'

// SVG coordinate system (scaled to 100% width via viewBox). The extra bottom
// padding leaves room for the dedicated bypass state strip below the plot.
const W = 640, H = 232
const PAD_L = 38, PAD_T = 26, PAD_B = 36
// Right gutters reserved for the humidity (%) and enthalpy (kJ/kg) axes when
// each is shown, so their labels don't get clipped at the SVG edge.
const PAD_R_BASE = 12, HUM_GUTTER = 20, ENTH_GUTTER = 34
const PLOT_H = H - PAD_T - PAD_B
// Bypass state strip: a thin solid bar just below the plot. A separate lane
// reads far clearer than tinting the plot background (open vs closed tints were
// indistinguishable).
const BYPASS_H = 7
const BYPASS_Y = PAD_T + PLOT_H + 8

// Shared style for the legend section headers (Measurements / Level changes /
// Change reason / Bypass) so they line up consistently.
const LEGEND_TITLE = 'font-size:0.72em;text-transform:uppercase;letter-spacing:0.5px;color:var(--text-muted);opacity:0.75;margin:10px 0 3px'

const mid = (b: HistoryBucket) => (b[0] + b[1]) / 2 // (min + max) / 2

// Index of the most recent populated bucket in a series, or -1 if all empty.
function lastIdx(series: (HistoryBucket | null)[]): number {
  for (let i = series.length - 1; i >= 0; i--) if (series[i] != null) return i
  return -1
}

const hasAny = (s?: (HistoryBucket | null)[]) => !!s?.some((b) => b != null)

type Pt = { x: number; y: number }

/** Split a series into runs of consecutive (non-missing) points. A single
 *  populated bucket — e.g. the first ~11 min after a restart — becomes a
 *  one-point run, which we draw as a dot rather than an invisible line. */
function segments(series: (HistoryBucket | null)[], x: (i: number) => number, y: (v: number) => number): Pt[][] {
  const segs: Pt[][] = []
  let cur: Pt[] = []
  series.forEach((b, i) => {
    if (b == null) { if (cur.length) { segs.push(cur); cur = [] } return }
    cur.push({ x: x(i), y: y(mid(b)) })
  })
  if (cur.length) segs.push(cur)
  return segs
}

/** A series: a line for runs of ≥2 points, a dot for lone points. `dashArray`
 *  marks the secondary-axis series (humidity dashed, enthalpy dotted). */
function Series({ segs, color, dashArray }: { segs: Pt[][]; color: string; dashArray?: string }) {
  return (
    <>
      {segs.map((seg) => seg.length >= 2
        ? <path d={'M' + seg.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join('L')} fill="none" stroke={color} stroke-width="2" stroke-linejoin="round" stroke-dasharray={dashArray} />
        : <circle cx={seg[0].x} cy={seg[0].y} r="2.5" fill={color} />
      )}
    </>
  )
}

// A styled hover tooltip: a small heading line plus body lines, anchored above
// a screen point. Each line may carry a colour swatch matching its chart series.
type TipLine = { text: string; color?: string }
type Tip = { x: number; y: number; head?: string; lines: TipLine[] }

// The scrubbing crosshair: a vertical line at SVG x, with a dot per visible
// curve where it crosses (in that curve's own y-scale).
type Cursor = { x: number; pts: { y: number; color: string }[] }

// Which curves / overlays are visible. Persisted per-browser so the chosen view
// survives reloads. Read defensively: a key missing from storage defaults on.
type VisKey = 'supply' | 'exhaust' | 'indoorHum' | 'outdoorHum' | 'indoorEnth' | 'outdoorEnth' | 'markers' | 'bypass'
const VIS_KEYS: VisKey[] = ['supply', 'exhaust', 'indoorHum', 'outdoorHum', 'indoorEnth', 'outdoorEnth', 'markers', 'bypass']
const VIS_STORAGE_KEY = 'wolf-cwl.timeline-visibility'

function loadVis(): Record<VisKey, boolean> {
  const out = VIS_KEYS.reduce((o, k) => (o[k] = true, o), {} as Record<VisKey, boolean>)
  try {
    const raw = typeof localStorage !== 'undefined' ? localStorage.getItem(VIS_STORAGE_KEY) : null
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<Record<VisKey, boolean>>
      for (const k of VIS_KEYS) if (parsed[k] === false) out[k] = false // unknown ⇒ visible
    }
  } catch { /* ignore malformed storage */ }
  return out
}

/** A legend entry that doubles as a show/hide toggle for its series/overlay. */
function LegendToggle({ on, onToggle, children }: { on: boolean; onToggle: () => void; children: any }) {
  return (
    <span class="legend-toggle" role="button" aria-pressed={on} tabIndex={0}
          style={`white-space:nowrap;opacity:${on ? 1 : 0.4}`}
          onClick={onToggle}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onToggle() } }}>
      {children}
    </span>
  )
}

export function TimelineChart({ lang }: { lang: Lang }) {
  const [hist, setHist] = useState<History | null>(null)
  const [tip, setTip] = useState<Tip | null>(null)
  const [cursor, setCursor] = useState<Cursor | null>(null)
  const [vis, setVis] = useState<Record<VisKey, boolean>>(loadVis)

  const toggle = (k: VisKey) => setVis((v) => {
    const next = { ...v, [k]: !v[k] }
    try { localStorage.setItem(VIS_STORAGE_KEY, JSON.stringify(next)) } catch { /* ignore */ }
    return next
  })

  // Anchor the tooltip at the pointer (mouse/touch) ...
  const tipAt = (e: { clientX: number; clientY: number }, lines: TipLine[], head?: string) =>
    setTip({ x: e.clientX, y: e.clientY, head, lines })
  // ... or, for keyboard focus, centered above the focused element.
  const tipAtEl = (e: { currentTarget: EventTarget | null }, lines: TipLine[], head?: string) => {
    const r = (e.currentTarget as Element)?.getBoundingClientRect?.()
    if (r) setTip({ x: r.left + r.width / 2, y: r.top, head, lines })
  }
  const hideTip = () => setTip(null)

  useEffect(() => {
    let alive = true
    const load = () => { getHistory().then((h) => { if (alive) setHist(h) }).catch(() => {}) }
    load()
    // History buckets roll over slowly (~11 min); poll well inside that.
    const id = setInterval(load, 30000)
    return () => { alive = false; clearInterval(id) }
  }, [])

  const tr = t(lang)
  const levels = tr.levels
  const reasonLabels = tr.reasonLabels as Record<string, string>

  // What data is present at all (independent of the visibility toggles).
  const hasTempData = !!hist && (hasAny(hist.supply) || hasAny(hist.exhaust))
  const hasHumidity = !!hist && (hasAny(hist.indoorHumidity) || hasAny(hist.outdoorHumidity))
  const hasEnthalpy = !!hist && (hasAny(hist.indoorEnthalpy) || hasAny(hist.outdoorEnthalpy))

  if (!hist || (!hasTempData && !hasHumidity && !hasEnthalpy)) {
    return (
      <div class="card">
        <h3>{t(lang).tempHistory}</h3>
        <p style="color:var(--text-muted);font-size:0.85em">{t(lang).noHistoryYet}</p>
      </div>
    )
  }

  // Temperature domain across the *visible* temperature series (left axis).
  let lo = Infinity, hi = -Infinity
  for (const [show, s] of [[vis.supply, hist.supply], [vis.exhaust, hist.exhaust]] as const) {
    if (!show) continue
    for (const b of s) { if (b) { if (b[0] < lo) lo = b[0]; if (b[1] > hi) hi = b[1] } }
  }
  const tempVisible = lo !== Infinity
  if (tempVisible) {
    if (hi - lo < 2) { const c = (hi + lo) / 2; lo = c - 1; hi = c + 1 }
    const padT = (hi - lo) * 0.1
    lo -= padT; hi += padT
  }

  // Enthalpy domain across the *visible* enthalpy series (own auto-scaled axis).
  let elo = Infinity, ehi = -Infinity
  for (const [show, s] of [[vis.indoorEnth, hist.indoorEnthalpy], [vis.outdoorEnth, hist.outdoorEnthalpy]] as const) {
    if (!show || !s) continue
    for (const b of s) { if (b) { if (b[0] < elo) elo = b[0]; if (b[1] > ehi) ehi = b[1] } }
  }
  const enthVisible = elo !== Infinity
  if (enthVisible) {
    if (ehi - elo < 2) { const c = (ehi + elo) / 2; elo = c - 1; ehi = c + 1 }
    const padE = (ehi - elo) * 0.1
    elo -= padE; ehi += padE
  }

  const humVisible = hasHumidity && (vis.indoorHum || vis.outdoorHum)

  // Reserve right gutters for whichever secondary axes are shown.
  const padR = PAD_R_BASE + (humVisible ? HUM_GUTTER : 0) + (enthVisible ? ENTH_GUTTER : 0)
  const PLOT_W = W - PAD_L - padR
  const xR = PAD_L + PLOT_W // plot right edge
  const humLabelX = xR + 4
  const enthLabelX = xR + (humVisible ? HUM_GUTTER + 2 : 4)

  const slots = hist.slots
  const x = (col: number) => PAD_L + (slots <= 1 ? 0 : (col / (slots - 1)) * PLOT_W)
  const y = (v: number) => PAD_T + (1 - (v - lo) / (hi - lo)) * PLOT_H

  // Map an event epoch (seconds) to an x position aligned with the bucket axis.
  const windowMs = (slots - 1) * hist.bucketMs
  const nowMs = hist.nowEpoch * 1000
  const eventX = (epochSec: number) => {
    const ageMs = nowMs - epochSec * 1000
    const frac = 1 - ageMs / windowMs
    return PAD_L + Math.max(0, Math.min(1, frac)) * PLOT_W
  }

  // Background bands tinted by bypass state. The transition log is oldest→newest;
  // the state at the window's left edge is the most recent transition at/before
  // it, or — if every transition is newer — the inverse of the earliest one (a
  // transition flips the state). No transitions → no shading (renders as before).
  const leftEpoch = hist.nowEpoch - windowMs / 1000
  const bypass = hist.bypass ?? []
  const bypassSpans: { x0: number; x1: number; open: boolean }[] = []
  if (bypass.length > 0) {
    const beforeEdge = bypass.filter((b) => b.epoch <= leftEpoch)
    let curOpen = beforeEdge.length > 0 ? beforeEdge[beforeEdge.length - 1].open : !bypass[0].open
    let curX = PAD_L
    for (const b of bypass.filter((b) => b.epoch > leftEpoch)) {
      const bx = eventX(b.epoch)
      if (bx > curX) bypassSpans.push({ x0: curX, x1: bx, open: curOpen })
      curX = bx
      curOpen = b.open
    }
    bypassSpans.push({ x0: curX, x1: PAD_L + PLOT_W, open: curOpen })
  }

  const supplySegs = segments(hist.supply, x, y)
  const exhaustSegs = segments(hist.exhaust, x, y)

  // Humidity overlay on a right-hand 0–100 % axis (own, coarser resolution).
  const hSlots = hist.humiditySlots ?? 0
  const xH = (col: number) => PAD_L + (hSlots <= 1 ? 0 : (col / (hSlots - 1)) * PLOT_W)
  const yH = (v: number) => PAD_T + (1 - v / 100) * PLOT_H
  const indoorHumSegs = hist.indoorHumidity ? segments(hist.indoorHumidity, xH, yH) : []
  const outdoorHumSegs = hist.outdoorHumidity ? segments(hist.outdoorHumidity, xH, yH) : []

  // Enthalpy overlay shares the humidity 5-min grid but its own kJ/kg axis.
  const yE = (v: number) => PAD_T + (1 - (v - elo) / (ehi - elo)) * PLOT_H
  const indoorEnthSegs = hist.indoorEnthalpy ? segments(hist.indoorEnthalpy, xH, yE) : []
  const outdoorEnthSegs = hist.outdoorEnthalpy ? segments(hist.outdoorEnthalpy, xH, yE) : []

  // Horizontal gridlines / axis labels at min, mid, max (temperature axis).
  const ticks = tempVisible ? [hi, (hi + lo) / 2, lo] : []
  const enthTicks = enthVisible ? [ehi, (ehi + elo) / 2, elo] : []
  const windowHours = Math.round((slots * hist.bucketMs) / 3_600_000)

  // Current-point markers: the most recent populated bucket of each visible
  // series, drawn emphasized so the "now" end of every line is anchored. Each
  // carries a value tooltip listing the latest readings.
  type Cur = { x: number; y: number; color: string }
  const curMarkers: Cur[] = []
  const valueLines: TipLine[] = []
  const pushCur = (show: boolean, series: (HistoryBucket | null)[] | undefined, xf: (i: number) => number, yf: (v: number) => number, color: string, label: string, unit: string) => {
    if (!show || !series) return
    const i = lastIdx(series)
    if (i < 0) return
    const v = mid(series[i]!)
    curMarkers.push({ x: xf(i), y: yf(v), color })
    valueLines.push({ text: `${label}: ${v.toFixed(1)}${unit}`, color })
  }
  pushCur(vis.supply, hist.supply, x, y, COL_SUPPLY, tr.supply, '°')
  pushCur(vis.exhaust, hist.exhaust, x, y, COL_EXHAUST, tr.exhaust, '°')
  pushCur(vis.indoorHum, hist.indoorHumidity, xH, yH, COL_INDOOR_HUM, tr.indoorRh, '%')
  pushCur(vis.outdoorHum, hist.outdoorHumidity, xH, yH, COL_OUTDOOR_HUM, tr.outdoorRh, '%')
  pushCur(vis.indoorEnth, hist.indoorEnthalpy, xH, yE, COL_INDOOR_ENTH, tr.indoorEnthalpy, ' kJ/kg')
  pushCur(vis.outdoorEnth, hist.outdoorEnthalpy, xH, yE, COL_OUTDOOR_ENTH, tr.outdoorEnthalpy, ' kJ/kg')

  // Heading for the value tooltip: the reading's wall-clock time.
  const pad2 = (n: number) => n.toString().padStart(2, '0')
  const hhmm = (epochSec: number) => { const d = new Date(epochSec * 1000); return `${pad2(d.getHours())}:${pad2(d.getMinutes())}` }
  const valueHead = `${tr.now} · ${hhmm(hist.nowEpoch)}`
  // The current-point hover shows the live snapshot; clear any scrubbing crosshair
  // so the two interactions don't fight over the tooltip.
  const showValue = (e: { clientX: number; clientY: number }) => { setCursor(null); tipAt(e, valueLines, valueHead) }

  // Scrubbing crosshair: map the pointer's x across the plot to a time bucket and
  // read every visible curve there. Humidity & enthalpy live on the coarser 5-min
  // grid (hSlots), temperatures on the per-bucket grid (slots); both span the same
  // window, so a single fraction indexes both.
  const sampleAt = (frac: number, n: number, series: (HistoryBucket | null)[] | undefined): number | null => {
    if (!series || series.length === 0) return null
    const i = Math.max(0, Math.min(series.length - 1, n <= 1 ? 0 : Math.round(frac * (n - 1))))
    const b = series[i]
    return b ? mid(b) : null
  }
  // Read every visible curve at a column fraction (0 = left edge, 1 = now),
  // returning tooltip lines (with swatch colours) and crosshair dot positions.
  const sampleColumn = (frac: number): { lines: TipLine[]; pts: { y: number; color: string }[] } => {
    const lines: TipLine[] = []
    const pts: { y: number; color: string }[] = []
    const add = (show: boolean, series: (HistoryBucket | null)[] | undefined, n: number, yf: (v: number) => number, color: string, label: string, unit: string) => {
      if (!show) return
      const v = sampleAt(frac, n, series)
      if (v == null) return
      lines.push({ text: `${label}: ${v.toFixed(1)}${unit}`, color })
      pts.push({ y: yf(v), color })
    }
    add(vis.supply, hist.supply, slots, y, COL_SUPPLY, tr.supply, '°')
    add(vis.exhaust, hist.exhaust, slots, y, COL_EXHAUST, tr.exhaust, '°')
    add(vis.indoorHum, hist.indoorHumidity, hSlots, yH, COL_INDOOR_HUM, tr.indoorRh, '%')
    add(vis.outdoorHum, hist.outdoorHumidity, hSlots, yH, COL_OUTDOOR_HUM, tr.outdoorRh, '%')
    add(vis.indoorEnth, hist.indoorEnthalpy, hSlots, yE, COL_INDOOR_ENTH, tr.indoorEnthalpy, ' kJ/kg')
    add(vis.outdoorEnth, hist.outdoorEnthalpy, hSlots, yE, COL_OUTDOOR_ENTH, tr.outdoorEnthalpy, ' kJ/kg')
    return { lines, pts }
  }
  const fracToEpoch = (frac: number) => windowMs > 0 ? hist.nowEpoch - (1 - frac) * windowMs / 1000 : hist.nowEpoch

  const onScrub = (e: { clientX: number; clientY: number; currentTarget: EventTarget | null }) => {
    const r = (e.currentTarget as Element)?.getBoundingClientRect?.()
    if (!r || r.width <= 0) return
    const frac = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width))
    const { lines, pts } = sampleColumn(frac)
    setCursor({ x: PAD_L + frac * PLOT_W, pts })
    if (lines.length > 0) setTip({ x: e.clientX, y: e.clientY, head: hhmm(fracToEpoch(frac)), lines })
    else setTip(null)
  }
  const endScrub = () => { setCursor(null); hideTip() }

  const swatchLine = (color: string, dashed?: boolean, dotted?: boolean) =>
    dashed
      ? `display:inline-block;width:14px;height:0;border-top:2px ${dotted ? 'dotted' : 'dashed'} ${color};vertical-align:middle;margin-right:4px`
      : `display:inline-block;width:14px;height:3px;background:${color};vertical-align:middle;margin-right:4px`

  return (
    <div class="card">
      <h3>{t(lang).tempHistory}</h3>
      <svg viewBox={`0 0 ${W} ${H}`} style="width:100%;height:auto;display:block">
        {/* gridlines + temperature labels */}
        {ticks.map((tv) => (
          <g>
            <line x1={PAD_L} y1={y(tv)} x2={xR} y2={y(tv)} stroke="var(--border-color)" stroke-width="1" />
            <text x={PAD_L - 5} y={y(tv) + 3} text-anchor="end" font-size="10" fill="var(--text-muted)">{tv.toFixed(0)}°</text>
          </g>
        ))}

        {/* right humidity axis (0–100 %) */}
        {humVisible && [0, 50, 100].map((hv) => (
          <text x={humLabelX} y={yH(hv) + 3} text-anchor="start" font-size="10" fill="var(--text-muted)">{hv}</text>
        ))}

        {/* outer enthalpy axis (auto-scaled kJ/kg) */}
        {enthVisible && (
          <>
            <text x={enthLabelX} y={PAD_T - 12} text-anchor="start" font-size="9" fill="var(--text-muted)">kJ/kg</text>
            {enthTicks.map((ev) => (
              <text x={enthLabelX} y={yE(ev) + 3} text-anchor="start" font-size="10" fill="var(--text-muted)">{ev.toFixed(0)}</text>
            ))}
          </>
        )}

        {/* enthalpy series (dotted) */}
        {vis.outdoorEnth && <Series segs={outdoorEnthSegs} color={COL_OUTDOOR_ENTH} dashArray="1,3" />}
        {vis.indoorEnth && <Series segs={indoorEnthSegs} color={COL_INDOOR_ENTH} dashArray="1,3" />}

        {/* humidity series (dashed, right axis) */}
        {vis.outdoorHum && <Series segs={outdoorHumSegs} color={COL_OUTDOOR_HUM} dashArray="4,3" />}
        {vis.indoorHum && <Series segs={indoorHumSegs} color={COL_INDOOR_HUM} dashArray="4,3" />}

        {/* event markers — level-change: line=level (state), dot=reason; reboot:
            neutral line+dot. Visual only; the hover hit area is in the interactive
            layer below so it stays above the scrubbing overlay. */}
        {vis.markers && hist.events.map((ev) => {
          const ex = eventX(ev.epoch)
          const rc = REASON_COLORS[ev.reason] ?? 'var(--text-muted)'
          const lc = LEVEL_COLORS[ev.level] ?? 'var(--text-muted)'
          const reboot = ev.reason === 'reboot'
          return (
            <g pointer-events="none">
              <line x1={ex} y1={PAD_T} x2={ex} y2={PAD_T + PLOT_H} stroke={reboot ? rc : lc} stroke-width="1" stroke-dasharray="3,3" opacity="0.85" />
              <circle cx={ex} cy={PAD_T - 4} r="3.5" fill={rc} stroke={reboot ? rc : lc} stroke-width="1.5" />
            </g>
          )
        })}

        {/* temperature series (left axis) */}
        {vis.exhaust && <Series segs={exhaustSegs} color={COL_EXHAUST} />}
        {vis.supply && <Series segs={supplySegs} color={COL_SUPPLY} />}

        {/* current-point markers — the latest reading of each visible series */}
        {curMarkers.map((m) => (
          <circle cx={m.x} cy={m.y} r="3.5" fill={m.color} stroke="var(--bg-secondary)" stroke-width="1.5" pointer-events="none" />
        ))}

        {/* scrubbing crosshair — a vertical line at the hovered time with a dot
            where it crosses each visible curve (visual only, no pointer events) */}
        {cursor && (
          <g pointer-events="none">
            <line x1={cursor.x} y1={PAD_T} x2={cursor.x} y2={PAD_T + PLOT_H} stroke="var(--text-secondary)" stroke-width="1" stroke-dasharray="3,3" opacity="0.8" />
            {cursor.pts.map((p) => (
              <circle cx={cursor.x} cy={p.y} r="3" fill={p.color} stroke="var(--bg-secondary)" stroke-width="1.5" />
            ))}
          </g>
        )}

        {/* bypass state strip — its own lane below the plot, solid colours so
            open (free cooling) vs closed (heat recovery) is unmistakable. Each
            span gets a hover hit area showing whether the bypass was open/closed. */}
        {vis.bypass && bypassSpans.length > 0 && (
          <g>
            {bypassSpans.map((s) => {
              const w = Math.max(0, s.x1 - s.x0)
              const lines: TipLine[] = [{ text: s.open ? tr.bypassOpen : tr.bypassClosed, color: s.open ? 'var(--chart-bypass-open)' : 'var(--chart-bypass-closed)' }]
              return (
                <>
                  <rect x={s.x0} y={BYPASS_Y} width={w} height={BYPASS_H}
                        fill={s.open ? 'var(--chart-bypass-open)' : 'var(--chart-bypass-closed)'} />
                  <rect class="chart-hit" x={s.x0} y={BYPASS_Y - 4} width={w} height={BYPASS_H + 8} fill="transparent" tabIndex={0}
                        onMouseEnter={(e) => tipAt(e, lines, tr.legendBypass)} onMouseLeave={hideTip}
                        onFocus={(e) => tipAtEl(e, lines, tr.legendBypass)} onBlur={hideTip} />
                </>
              )
            })}
            <rect x={PAD_L} y={BYPASS_Y} width={PLOT_W} height={BYPASS_H} rx="1.5"
                  fill="none" stroke="var(--border-color)" stroke-width="1" pointer-events="none" />
          </g>
        )}

        {/* ---- interactive layer (transparent hit targets, painted last so they
             sit above every visual element) ---- */}

        {/* scrubbing overlay over the whole plot: drives the crosshair + the
            per-curve value tooltip wherever the pointer is. Sits beneath the
            marker / current-point hit areas so those keep their own tooltips. */}
        <rect class="chart-hit" x={PAD_L} y={PAD_T} width={PLOT_W} height={PLOT_H} fill="transparent"
              onMouseMove={onScrub} onMouseEnter={onScrub} onMouseLeave={endScrub} />

        {/* level-change marker hit areas — wide transparent rects over the thin
            lines. Hover shows the level + reason AND every visible curve's value
            at the marker's time, with the crosshair anchored on the marker. */}
        {vis.markers && hist.events.map((ev) => {
          const ex = eventX(ev.epoch)
          const rc = REASON_COLORS[ev.reason] ?? 'var(--text-muted)'
          const lc = LEVEL_COLORS[ev.level] ?? 'var(--text-muted)'
          const reboot = ev.reason === 'reboot'
          const desc = reboot
            ? (reasonLabels.reboot ?? 'Reboot')
            : `${levels[ev.level] ?? ev.level} · ${reasonLabels[ev.reason] ?? ev.reason}`
          // Reboot has no level, so fall back to its neutral reason colour.
          const frac = PLOT_W > 0 ? Math.max(0, Math.min(1, (ex - PAD_L) / PLOT_W)) : 1
          const col = sampleColumn(frac)
          const lines: TipLine[] = [{ text: desc, color: reboot ? rc : lc }, ...col.lines]
          const head = hhmm(ev.epoch)
          return (
            <rect class="chart-hit" x={ex - 6} y={PAD_T - 8} width="12" height={PLOT_H + 8} fill="transparent" tabIndex={0}
                  onMouseEnter={(e) => { setCursor({ x: ex, pts: col.pts }); tipAt(e, lines, head) }} onMouseLeave={endScrub}
                  onFocus={(e) => { setCursor({ x: ex, pts: col.pts }); tipAtEl(e, lines, head) }} onBlur={endScrub} />
          )
        })}

        {/* current-point hit circles — the live snapshot of every latest reading */}
        {curMarkers.map((m) => (
          <circle class="chart-hit" cx={m.x} cy={m.y} r="9" fill="transparent"
                  onMouseEnter={showValue} onMouseMove={showValue} onMouseLeave={hideTip} />
        ))}

        {/* time axis labels */}
        <text x={PAD_L} y={H - 6} text-anchor="start" font-size="10" fill="var(--text-muted)">-{windowHours}h</text>
        <text x={xR} y={H - 6} text-anchor="end" font-size="10" fill="var(--text-muted)">{tr.now.toLowerCase()}</text>
      </svg>

      {/* hover tooltip — anchored above the point, flipped below near the top edge */}
      {tip && tip.lines.length > 0 && (() => {
        const vw = typeof window !== 'undefined' ? window.innerWidth : 9999
        const left = Math.max(60, Math.min(vw - 60, tip.x))
        const flip = tip.y < 80
        const top = flip ? tip.y + 14 : tip.y - 10
        const transform = `translate(-50%, ${flip ? '0' : '-100%'})`
        return (
          <div class="chart-tooltip" style={`left:${left}px;top:${top}px;transform:${transform}`}>
            {tip.head && <div class="chart-tooltip-time">{tip.head}</div>}
            {tip.lines.map((l) => (
              <div style="display:flex;align-items:center;gap:6px">
                <span style={`width:8px;height:8px;border-radius:50%;flex-shrink:0;background:${l.color ?? 'transparent'}`} />
                <span>{l.text}</span>
              </div>
            ))}
          </div>
        )
      })()}

      {/* legend — each entry doubles as a show/hide toggle */}
      <div style={LEGEND_TITLE}>{tr.legendMeasurements}:</div>
      <div style="display:flex;gap:6px 14px;flex-wrap:wrap;align-items:center;font-size:0.8em;color:var(--text-muted)">
        <LegendToggle on={vis.supply} onToggle={() => toggle('supply')}><span style={swatchLine(COL_SUPPLY)} />{tr.supply}</LegendToggle>
        <LegendToggle on={vis.exhaust} onToggle={() => toggle('exhaust')}><span style={swatchLine(COL_EXHAUST)} />{tr.exhaust}</LegendToggle>
        {hasHumidity && <LegendToggle on={vis.indoorHum} onToggle={() => toggle('indoorHum')}><span style={swatchLine(COL_INDOOR_HUM, true)} />{tr.indoorRh}</LegendToggle>}
        {hasHumidity && <LegendToggle on={vis.outdoorHum} onToggle={() => toggle('outdoorHum')}><span style={swatchLine(COL_OUTDOOR_HUM, true)} />{tr.outdoorRh}</LegendToggle>}
        {hasEnthalpy && <LegendToggle on={vis.indoorEnth} onToggle={() => toggle('indoorEnth')}><span style={swatchLine(COL_INDOOR_ENTH, true, true)} />{tr.indoorEnthalpy}</LegendToggle>}
        {hasEnthalpy && <LegendToggle on={vis.outdoorEnth} onToggle={() => toggle('outdoorEnth')}><span style={swatchLine(COL_OUTDOOR_ENTH, true, true)} />{tr.outdoorEnthalpy}</LegendToggle>}
      </div>
      {bypassSpans.length > 0 && (
        <>
          <div style={LEGEND_TITLE}>{tr.legendBypass}:</div>
          <div style="display:flex;gap:6px 14px;flex-wrap:wrap;align-items:center;font-size:0.8em;color:var(--text-muted)">
            <LegendToggle on={vis.bypass} onToggle={() => toggle('bypass')}>
              <span style="display:inline-block;width:14px;height:10px;background:var(--chart-bypass-open);border:1px solid var(--border-color);vertical-align:middle;margin-right:4px" />{tr.bypassOpen}
            </LegendToggle>
            <LegendToggle on={vis.bypass} onToggle={() => toggle('bypass')}>
              <span style="display:inline-block;width:14px;height:10px;background:var(--chart-bypass-closed);border:1px solid var(--border-color);vertical-align:middle;margin-right:4px" />{tr.bypassClosed}
            </LegendToggle>
          </div>
        </>
      )}
      {hist.events.length > 0 && (
        <>
          {/* level swatches (dashed) and reason markers (dots) never interleave;
              the whole overlay toggles via its section entries */}
          <div style={LEGEND_TITLE}>{tr.levelChanges}:</div>
          <div style="display:flex;gap:6px 14px;flex-wrap:wrap;align-items:center;font-size:0.78em;color:var(--text-muted)">
            {levels.map((name, i) => (
              <LegendToggle on={vis.markers} onToggle={() => toggle('markers')}>
                <span style={`display:inline-block;width:10px;height:0;border-top:2px dashed ${LEVEL_COLORS[i]};vertical-align:middle;margin-right:4px`} />{name}
              </LegendToggle>
            ))}
          </div>
          <div style={LEGEND_TITLE}>{tr.legendReason}:</div>
          <div style="display:flex;gap:6px 14px;flex-wrap:wrap;align-items:center;font-size:0.78em;color:var(--text-muted)">
            {[...new Set(hist.events.map((e) => e.reason))].map((r) => (
              <LegendToggle on={vis.markers} onToggle={() => toggle('markers')}>
                <span style={`display:inline-block;width:8px;height:8px;border-radius:50%;background:${REASON_COLORS[r] ?? 'var(--text-muted)'};vertical-align:middle;margin-right:4px`} />{reasonLabels[r] ?? r}
              </LegendToggle>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
