import { useState, useEffect } from 'preact/hooks'
import { resumeSchedule } from './api'
import type { Status } from './api'
import { TimelineChart } from './TimelineChart'
import { t, type Lang } from './translations'

export function StatusTab({ status, lang, onLevelChange, onCancelOff, onConfirmed }: {
  status: Status | null
  lang: Lang
  onLevelChange: (level: number) => void
  onCancelOff: () => void
  onConfirmed?: () => void
}) {
  const [localPending, setLocalPending] = useState<number | null>(null)

  // Clear pending once the unit's *actual* level (ID 77) reaches what was set.
  useEffect(() => {
    if (localPending !== null && status?.ventilation.actualLevel === localPending) {
      setLocalPending(null)
      onConfirmed?.()
    }
  }, [status?.ventilation.actualLevel, localPending])

  const handleLevelChange = (level: number) => {
    setLocalPending(level)
    onLevelChange(level)
  }

  if (!status) return <div>Loading...</div>

  const fmtRemaining = (min: number) => {
    const h = Math.floor(min / 60), m = min % 60
    return h > 0 ? `${h}h ${m}m` : `${m}m`
  }

  // Determine which level to show as "selected" (active).
  // A user-initiated change shows the pending level (with spinner) until the
  // unit confirms; otherwise always show the unit's *actual* running level
  // (ID 77), not the level we commanded (ID 71) — they can differ.
  const displayLevel = localPending ?? status.ventilation.actualLevel

  const eh = status.extremeHeat
  const hum = status.humidity
  const showDecision = eh.enabled || eh.protectionEnabled
  const tr = t(lang)
  const reasonLabels = tr.reasonLabels as Record<string, string>
  const reasonText = tr.reasonText as Record<string, string>
  const n1 = (v: number | null, suffix = '') => (v == null ? '–' : `${v.toFixed(1)}${suffix}`)
  const dh = hum.indoorEnthalpy != null && hum.outdoorEnthalpy != null ? hum.outdoorEnthalpy - hum.indoorEnthalpy : null
  const dhStr = dh == null ? '–' : `${dh >= 0 ? '+' : ''}${dh.toFixed(1)}`
  const fmtMMSS = (s: number) => { const m = Math.floor(Math.max(0, s) / 60); const ss = Math.max(0, s) % 60; return `${m}:${String(ss).padStart(2, '0')}` }

  return (
    <>
      {status.timedOff?.active && (
        <div class="msg error" style="display:flex;justify-content:space-between;align-items:center">
          <span>Ventilation Off — resumes in {fmtRemaining(status.timedOff.remainingMinutes)}</span>
          <button class="danger" style="padding:6px 12px;font-size:0.8em;margin:0" onClick={onCancelOff}>Cancel</button>
        </div>
      )}
      <div class="card">
        <h3>{t(lang).ventilation}</h3>
        {status.extremeHeat.enabled && (() => {
          const lvl = status.extremeHeat.currentLevel
          const d = status.temperature.supply - status.temperature.exhaust
          const dStr = `${d >= 0 ? '+' : ''}${d.toFixed(1)}`
          return (
            <div class="msg warning">
              <strong>{t(lang).extremeHeatActive}</strong><br />
              {t(lang).extremeHeatForcedTo}: <strong>{t(lang).levels[lvl]}</strong><br />
              {t(lang).extremeHeatMatchedRule}: {reasonLabels[eh.reason] ?? eh.reason} (Δ {dStr} °C)
            </div>
          )
        })()}
        <div class="level-buttons">
          {t(lang).levels.map((name, i) => {
            const isSelected = displayLevel === i
            const isPending = localPending === i
            return (
              <div class={`level-btn ${isSelected ? 'active' : ''} ${isPending ? 'pending' : ''}`}
                   onClick={() => handleLevelChange(i)}>{isPending ? <span class="spinner" /> : null}{name}</div>
            )
          })}
        </div>
        <div class="stat"><span class="label">Relative</span><span class="value">{status.ventilation.relative}%</span></div>
        {status.ventilation.override && (
          <button class="resume-btn" onClick={async () => { await resumeSchedule(); onConfirmed?.() }}>Resume Schedule</button>
        )}
      </div>
      <div class="card">
        <h3>{t(lang).temperatures}</h3>
        <div class="stat"><span class="label">{t(lang).supply}</span><span class="value">{status.temperature.supply.toFixed(1)} °C</span></div>
        <div class="stat"><span class="label">{t(lang).exhaust}</span><span class="value">{status.temperature.exhaust.toFixed(1)} °C</span></div>
      </div>
      <TimelineChart lang={lang} />

      {showDecision && (
        <div class="card">
          <h3>{tr.climateDecision}</h3>
          <div class="stat"><span class="label">{tr.activeRule}</span><span class="value">{reasonLabels[eh.reason] ?? eh.reason}</span></div>
          <div class="stat"><span class="label">{tr.level}</span><span class="value">{tr.levels[eh.currentLevel] ?? eh.currentLevel}</span></div>
          <p style="font-size:0.85em;color:var(--text-muted);margin:6px 0">{reasonText[eh.reason] ?? ''}</p>
          {eh.holdReason !== 'none' && (
            <div class="msg" style="font-size:0.85em;margin:6px 0">
              {eh.holdReason === 'dwell'
                ? tr.holdDwell.replace('{level}', tr.levels[eh.pendingLevel] ?? String(eh.pendingLevel)).replace('{time}', fmtMMSS(eh.dwellRemainingSecs))
                : tr.holdDeadband.replace('{dh}', dhStr)}
            </div>
          )}
          {hum.active ? (
            <>
              <div class="stat"><span class="label">{tr.indoorAir}</span><span class="value">{n1(hum.indoorTemp, '°')} · {n1(hum.indoorRh, '%')} · {n1(hum.indoorAh, ' g/m³')} · {n1(hum.indoorEnthalpy, ' kJ/kg')}</span></div>
              <div class="stat"><span class="label">{tr.outdoorAir}</span><span class="value">{n1(hum.outdoorTemp, '°')} · {n1(hum.outdoorRh, '%')} · {n1(hum.outdoorAh, ' g/m³')} · {n1(hum.outdoorEnthalpy, ' kJ/kg')}</span></div>
              <p style="font-size:0.8em;color:var(--text-muted);margin:2px 0 0">{tr.aggregateHint}</p>
              <div class="stat"><span class="label">{tr.ambientPressure}</span><span class="value">{(hum.ambientPressureKpa * 10).toFixed(0)} hPa</span></div>
              {eh.protectionActive && <div class="msg warning" style="margin-top:8px">{tr.protectionActiveMsg}</div>}
            </>
          ) : (
            <div class="msg" style="font-size:0.85em">{status.status.connected ? tr.tempOnlyFallback : tr.waitingForUnit}</div>
          )}
        </div>
      )}

      {hum.sensors.length > 0 && (
        <div class="card">
          <h3>{tr.humiditySensors}</h3>
          {hum.sensors.map((sn) => (
            <div class="stat">
              <span class="label">{sn.role === 'outdoor' ? '🌤' : '🏠'} {sn.topic}</span>
              <span class={`value ${sn.fresh ? '' : 'fault'}`}>
                {sn.humidity != null ? `${sn.humidity.toFixed(0)}%` : ''}{sn.temperature != null ? `${sn.humidity != null ? ' · ' : ''}${sn.temperature.toFixed(1)}°` : ''}{sn.pressure != null ? ` · ${sn.pressure.toFixed(0)} hPa` : ''}{sn.fresh ? '' : ' · stale'}
              </span>
            </div>
          ))}
        </div>
      )}

      <div class="card">
        <h3>Status</h3>
        <div class="stat"><span class="label">Connected</span><span class={`value ${status.status.connected ? 'ok' : 'fault'}`}>{status.status.connected ? 'Yes' : 'No'}</span></div>
        <div class="stat"><span class="label">Filter</span><span class={`value ${status.status.filter ? 'fault' : 'ok'}`}>{status.status.filter ? 'Replace' : 'OK'}</span></div>
        <div class="stat"><span class="label">Bypass</span><span class="value">{status.status.bypass ? 'Open' : 'Closed'}</span></div>
      </div>
    </>
  )
}
