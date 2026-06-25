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

// SVG coordinate system (scaled to 100% width via viewBox).
const W = 640, H = 220
const PAD_L = 38, PAD_T = 26, PAD_B = 24
// Right padding is wider when the humidity axis (0–100 %) is shown so its
// labels don't get clipped at the SVG edge.
const PAD_R_BASE = 12, PAD_R_HUM = 30
const PLOT_H = H - PAD_T - PAD_B

// Shared style for the three legend section headers (Measurements / Level
// changes / Change reason) so they line up consistently.
const LEGEND_TITLE = 'font-size:0.72em;text-transform:uppercase;letter-spacing:0.5px;color:var(--text-muted);opacity:0.75;margin:10px 0 3px'

const mid = (b: HistoryBucket) => (b[0] + b[1]) / 2 // (min + max) / 2

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

/** A series: a line for runs of ≥2 points, a dot for lone points. `dash` marks
 *  the (coarser, secondary-axis) humidity series. */
function Series({ segs, color, dash }: { segs: Pt[][]; color: string; dash?: boolean }) {
  return (
    <>
      {segs.map((seg) => seg.length >= 2
        ? <path d={'M' + seg.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join('L')} fill="none" stroke={color} stroke-width="2" stroke-linejoin="round" stroke-dasharray={dash ? '4,3' : undefined} />
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

  const tr = t(lang)
  const levels = tr.levels
  const reasonLabels = tr.reasonLabels as Record<string, string>

  // Temperature domain across both series.
  let lo = Infinity, hi = -Infinity
  if (hist) {
    for (const s of [hist.supply, hist.exhaust]) {
      for (const b of s) { if (b) { if (b[0] < lo) lo = b[0]; if (b[1] > hi) hi = b[1] } }
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

  // Is there any humidity data? Decided from raw buckets so the right padding
  // (and thus the plot width) can be reserved before the scales are built.
  const hasHumidity = !!(hist!.indoorHumidity?.some((b) => b != null) || hist!.outdoorHumidity?.some((b) => b != null))
  const padR = hasHumidity ? PAD_R_HUM : PAD_R_BASE
  const PLOT_W = W - PAD_L - padR

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

  // Humidity overlay on a right-hand 0–100 % axis (own, coarser resolution).
  const hSlots = hist!.humiditySlots ?? 0
  const xH = (col: number) => PAD_L + (hSlots <= 1 ? 0 : (col / (hSlots - 1)) * PLOT_W)
  const yH = (v: number) => PAD_T + (1 - v / 100) * PLOT_H
  const indoorHumSegs = hist!.indoorHumidity ? segments(hist!.indoorHumidity, xH, yH) : []
  const outdoorHumSegs = hist!.outdoorHumidity ? segments(hist!.outdoorHumidity, xH, yH) : []

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
            <line x1={PAD_L} y1={y(tv)} x2={W - padR} y2={y(tv)} stroke="var(--border-color)" stroke-width="1" />
            <text x={PAD_L - 5} y={y(tv) + 3} text-anchor="end" font-size="10" fill="var(--text-muted)">{tv.toFixed(0)}°</text>
          </g>
        ))}

        {/* right humidity axis (0–100 %) */}
        {hasHumidity && [0, 50, 100].map((hv) => (
          <text x={W - padR + 4} y={yH(hv) + 3} text-anchor="start" font-size="10" fill="var(--text-muted)">{hv}</text>
        ))}

        {/* humidity series (dashed, right axis) */}
        <Series segs={outdoorHumSegs} color="#a78bfa" dash />
        <Series segs={indoorHumSegs} color="#22c3c3" dash />

        {/* event markers — level-change: line=level (state), dot=reason; reboot: neutral line+dot */}
        {hist!.events.map((ev) => {
          const ex = eventX(ev.epoch)
          const rc = REASON_COLORS[ev.reason] ?? 'var(--text-muted)'
          const lc = LEVEL_COLORS[ev.level] ?? 'var(--text-muted)'
          const reboot = ev.reason === 'reboot'
          const title = reboot ? (reasonLabels.reboot ?? 'Reboot') : `${levels[ev.level] ?? ev.level} · ${reasonLabels[ev.reason] ?? ev.reason}`
          return (
            <g>
              <title>{title}</title>
              <line x1={ex} y1={PAD_T} x2={ex} y2={PAD_T + PLOT_H} stroke={reboot ? rc : lc} stroke-width="1" stroke-dasharray="3,3" opacity="0.85" />
              <circle cx={ex} cy={PAD_T - 4} r="3.5" fill={rc} stroke={reboot ? rc : lc} stroke-width="1.5" />
            </g>
          )
        })}

        {/* temperature series (left axis) */}
        <Series segs={exhaustSegs} color="var(--accent-blue)" />
        <Series segs={supplySegs} color="var(--accent-orange)" />

        {/* time axis labels */}
        <text x={PAD_L} y={H - 6} text-anchor="start" font-size="10" fill="var(--text-muted)">-{windowHours}h</text>
        <text x={W - padR} y={H - 6} text-anchor="end" font-size="10" fill="var(--text-muted)">now</text>
      </svg>

      {/* legend */}
      <div style={LEGEND_TITLE}>{tr.legendMeasurements}:</div>
      <div style="display:flex;gap:6px 14px;flex-wrap:wrap;align-items:center;font-size:0.8em;color:var(--text-muted)">
        <span style="white-space:nowrap"><span style="display:inline-block;width:14px;height:3px;background:var(--accent-orange);vertical-align:middle;margin-right:4px" />{t(lang).supplyInlet}</span>
        <span style="white-space:nowrap"><span style="display:inline-block;width:14px;height:3px;background:var(--accent-blue);vertical-align:middle;margin-right:4px" />{t(lang).exhaustInlet}</span>
        {hasHumidity && <span style="white-space:nowrap"><span style="display:inline-block;width:14px;height:0;border-top:2px dashed #22c3c3;vertical-align:middle;margin-right:4px" />{tr.indoorRh}</span>}
        {hasHumidity && <span style="white-space:nowrap"><span style="display:inline-block;width:14px;height:0;border-top:2px dashed #a78bfa;vertical-align:middle;margin-right:4px" />{tr.outdoorRh}</span>}
      </div>
      {hist!.events.length > 0 && (
        <>
          {/* each section gets its own title line, then its swatches; level
              swatches (dashed) and reason markers (dots) never interleave */}
          <div style={LEGEND_TITLE}>{tr.levelChanges}:</div>
          <div style="display:flex;gap:6px 14px;flex-wrap:wrap;align-items:center;font-size:0.78em;color:var(--text-muted)">
            {levels.map((name, i) => (
              <span style="white-space:nowrap"><span style={`display:inline-block;width:10px;height:0;border-top:2px dashed ${LEVEL_COLORS[i]};vertical-align:middle;margin-right:4px`} />{name}</span>
            ))}
          </div>
          <div style={LEGEND_TITLE}>{tr.legendReason}:</div>
          <div style="display:flex;gap:6px 14px;flex-wrap:wrap;align-items:center;font-size:0.78em;color:var(--text-muted)">
            {[...new Set(hist!.events.map((e) => e.reason))].map((r) => (
              <span style="white-space:nowrap"><span style={`display:inline-block;width:8px;height:8px;border-radius:50%;background:${REASON_COLORS[r] ?? 'var(--text-muted)'};vertical-align:middle;margin-right:4px`} />{reasonLabels[r] ?? r}</span>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
