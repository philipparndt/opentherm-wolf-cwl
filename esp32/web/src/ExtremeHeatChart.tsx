import { useState, useEffect } from 'preact/hooks'
import { getHistory, type History, type HistoryBucket } from './api'
import { t, type Lang } from './translations'

// Off / Reduced / Normal / Party — same order & colours as the rest of the UI.
const LEVEL_COLORS = ['var(--accent-red)', 'var(--accent-orange)', 'var(--accent-blue)', 'var(--accent-green)'] as const

// SVG coordinate system (scaled to 100% width via viewBox).
const W = 640, H = 220
const PAD_L = 38, PAD_R = 12, PAD_T = 26, PAD_B = 24
const PLOT_W = W - PAD_L - PAD_R
const PLOT_H = H - PAD_T - PAD_B

const mid = (b: HistoryBucket) => (b.min + b.max) / 2

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

/** A temperature series: a line for runs of ≥2 points, a dot for lone points. */
function Series({ segs, color }: { segs: Pt[][]; color: string }) {
  return (
    <>
      {segs.map((seg) => seg.length >= 2
        ? <path d={'M' + seg.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join('L')} fill="none" stroke={color} stroke-width="2" stroke-linejoin="round" />
        : <circle cx={seg[0].x} cy={seg[0].y} r="2.5" fill={color} />
      )}
    </>
  )
}

export function ExtremeHeatChart({ lang }: { lang: Lang }) {
  const [hist, setHist] = useState<History | null>(null)

  useEffect(() => {
    let alive = true
    const load = () => { getHistory().then((h) => { if (alive) setHist(h) }).catch(() => {}) }
    load()
    // History buckets roll over slowly (~11 min); poll well inside that.
    const id = setInterval(load, 30000)
    return () => { alive = false; clearInterval(id) }
  }, [])

  const levels = t(lang).levels

  // Temperature domain across both series.
  let lo = Infinity, hi = -Infinity
  if (hist) {
    for (const s of [hist.supply, hist.exhaust]) {
      for (const b of s) { if (b) { if (b.min < lo) lo = b.min; if (b.max > hi) hi = b.max } }
    }
  }
  const hasData = hist != null && lo !== Infinity
  if (!hasData) {
    return (
      <div class="card">
        <h3>{t(lang).tempHistory}</h3>
        <p style="color:var(--text-muted);font-size:0.85em">{t(lang).noHistoryYet}</p>
      </div>
    )
  }

  // Pad the temperature range a little so lines don't touch the edges.
  if (hi - lo < 2) { const c = (hi + lo) / 2; lo = c - 1; hi = c + 1 }
  const padT = (hi - lo) * 0.1
  lo -= padT; hi += padT

  const slots = hist!.slots
  const x = (col: number) => PAD_L + (slots <= 1 ? 0 : (col / (slots - 1)) * PLOT_W)
  const y = (v: number) => PAD_T + (1 - (v - lo) / (hi - lo)) * PLOT_H

  // Map an event epoch (seconds) to an x position aligned with the bucket axis.
  const windowMs = (slots - 1) * hist!.bucketMs
  const nowMs = hist!.nowEpoch * 1000
  const eventX = (epochSec: number) => {
    const ageMs = nowMs - epochSec * 1000
    const frac = 1 - ageMs / windowMs
    return PAD_L + Math.max(0, Math.min(1, frac)) * PLOT_W
  }

  const supplySegs = segments(hist!.supply, x, y)
  const exhaustSegs = segments(hist!.exhaust, x, y)

  // Horizontal gridlines / axis labels at min, mid, max.
  const ticks = [hi, (hi + lo) / 2, lo]
  const windowHours = Math.round((slots * hist!.bucketMs) / 3_600_000)

  return (
    <div class="card">
      <h3>{t(lang).tempHistory}</h3>
      <svg viewBox={`0 0 ${W} ${H}`} style="width:100%;height:auto;display:block">
        {/* gridlines + temperature labels */}
        {ticks.map((tv) => (
          <g>
            <line x1={PAD_L} y1={y(tv)} x2={W - PAD_R} y2={y(tv)} stroke="var(--border-color)" stroke-width="1" />
            <text x={PAD_L - 5} y={y(tv) + 3} text-anchor="end" font-size="10" fill="var(--text-muted)">{tv.toFixed(0)}°</text>
          </g>
        ))}

        {/* event markers — vertical line + level dot */}
        {hist!.events.map((ev) => {
          const ex = eventX(ev.epoch)
          const color = LEVEL_COLORS[ev.level] ?? 'var(--text-muted)'
          return (
            <g>
              <line x1={ex} y1={PAD_T} x2={ex} y2={PAD_T + PLOT_H} stroke={color} stroke-width="1" stroke-dasharray="3,3" opacity="0.8" />
              <circle cx={ex} cy={PAD_T - 4} r="3.5" fill={color} />
            </g>
          )
        })}

        {/* temperature series */}
        <Series segs={exhaustSegs} color="var(--accent-blue)" />
        <Series segs={supplySegs} color="var(--accent-orange)" />

        {/* time axis labels */}
        <text x={PAD_L} y={H - 6} text-anchor="start" font-size="10" fill="var(--text-muted)">-{windowHours}h</text>
        <text x={W - PAD_R} y={H - 6} text-anchor="end" font-size="10" fill="var(--text-muted)">now</text>
      </svg>

      {/* legend */}
      <div style="display:flex;gap:16px;flex-wrap:wrap;font-size:0.8em;color:var(--text-muted);margin-top:4px">
        <span><span style="display:inline-block;width:14px;height:3px;background:var(--accent-orange);vertical-align:middle;margin-right:4px" />{t(lang).supplyInlet}</span>
        <span><span style="display:inline-block;width:14px;height:3px;background:var(--accent-blue);vertical-align:middle;margin-right:4px" />{t(lang).exhaustInlet}</span>
        {hist!.events.length > 0 && <span style="margin-left:auto">{t(lang).levelChanges}:</span>}
        {hist!.events.length > 0 && levels.map((name, i) => (
          <span><span style={`display:inline-block;width:8px;height:8px;border-radius:50%;background:${LEVEL_COLORS[i]};vertical-align:middle;margin-right:4px`} />{name}</span>
        ))}
      </div>
    </div>
  )
}
