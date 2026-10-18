import { useState, useEffect } from 'preact/hooks'
import { resumeSchedule } from './api'
import type { Status } from './api'
import { TimelineChart } from './TimelineChart'
import { t, fx, type Lang } from './translations'

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

  const tr = t(lang)
  if (!status) return <div class="msg">{tr.loading}</div>

  const fmtRemaining = (min: number) => {
    const h = Math.floor(min / 60), m = min % 60
    return h > 0 ? `${h} h ${m} min` : `${m} min`
  }

  // Determine which level to show as "selected" (active).
  // A user-initiated change shows the pending level (with spinner) until the
  // unit confirms; otherwise always show the unit's *actual* running level
  // (ID 77), not the level we commanded (ID 71) — they can differ.
  const displayLevel = localPending ?? status.ventilation.actualLevel

  const eh = status.extremeHeat
  const hum = status.humidity
  const showDecision = eh.enabled || eh.protectionEnabled
  const reasonLabels = tr.reasonLabels as Record<string, string>
  const reasonText = tr.reasonText as Record<string, string>
  const n1 = (v: number | null, suffix = '') => (v == null ? '–' : `${fx(lang, v)}${suffix}`)
  const dh = hum.indoorEnthalpy != null && hum.outdoorEnthalpy != null ? hum.outdoorEnthalpy - hum.indoorEnthalpy : null
  const dhStr = dh == null ? '–' : `${dh >= 0 ? '+' : ''}${fx(lang, dh)}`
  const fmtMMSS = (s: number) => { const m = Math.floor(Math.max(0, s) / 60); const ss = Math.max(0, s) % 60; return `${m}:${String(ss).padStart(2, '0')}` }
  const al = status.ventilation.actualLevel
  const flow = [0, status.airflow?.reduced, status.airflow?.normal, status.airflow?.party][al]

  return (
    <>
      {status.timedOff?.active && (
        <div class="msg error banner">
          <span>{tr.ventOffResumes.replace('{t}', fmtRemaining(status.timedOff.remainingMinutes))}</span>
          <button class="sm" onClick={onCancelOff}>{tr.cancelOff}</button>
        </div>
      )}
      <div class="card">
        <h3>{tr.levelTitle}
          <span class="meta"><span class={`dot ${status.ventilation.override ? 'warn' : 'idle'}`} />{status.ventilation.override ? tr.manualOverride : tr.followsSchedule}</span>
        </h3>
        {status.extremeHeat.enabled && (() => {
          const lvl = status.extremeHeat.currentLevel
          const d = status.temperature.supply - status.temperature.exhaust
          const dStr = `${d >= 0 ? '+' : ''}${fx(lang, d)}`
          return (
            <div class="msg warning">
              <strong>{tr.extremeHeatActive}</strong><br />
              {tr.extremeHeatForcedTo}: <strong>{tr.levels[lvl]}</strong><br />
              {tr.extremeHeatMatchedRule}: {reasonLabels[eh.reason] ?? eh.reason} (Δ {dStr} °C)
            </div>
          )
        })()}
        <div class="level-buttons" role="group" aria-label={tr.levelTitle}>
          {tr.levels.map((name, i) => {
            const isSelected = displayLevel === i
            const isPending = localPending === i
            return (
              <div class={`level-btn ${isSelected ? 'active' : ''} ${isPending ? 'pending' : ''}`} role="button" tabIndex={0} aria-pressed={isSelected}
                   onClick={() => handleLevelChange(i)}
                   onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleLevelChange(i) } }}>{isPending ? <span class="spinner" /> : null}{name}</div>
            )
          })}
        </div>
        <div class="figs">
          <div class="fig"><div class="k">{tr.supply}</div><div class="v">{fx(lang, status.temperature.supply)}<small>°C</small></div></div>
          <div class="fig"><div class="k">{tr.exhaust}</div><div class="v">{fx(lang, status.temperature.exhaust)}<small>°C</small></div></div>
          <div class="fig"><div class="k">{tr.volumeFlow}</div><div class="v">{flow ?? '–'}<small>m³/h</small></div></div>
          <div class="fig"><div class="k">{tr.relative}</div><div class="v">{status.ventilation.relative}<small>%</small></div></div>
        </div>
        {status.ventilation.override && (
          <button style="margin-top:12px;width:100%" onClick={async () => { await resumeSchedule(); onConfirmed?.() }}>{tr.resumeSchedule}</button>
        )}
      </div>
      <TimelineChart lang={lang} />

      {showDecision && (
        <div class="card">
          <h3>{tr.climateDecision}</h3>
          <div class="stat"><span class="label">{tr.activeRule}</span><span class="value">{reasonLabels[eh.reason] ?? eh.reason}</span></div>
          <div class="stat"><span class="label">{tr.level}</span><span class="value">{tr.levels[eh.currentLevel] ?? eh.currentLevel}</span></div>
          <p class="hint" style="margin:8px 0">{reasonText[eh.reason] ?? ''}</p>
          {eh.holdReason !== 'none' && (
            <div class="msg">
              {eh.holdReason === 'dwell'
                ? tr.holdDwell.replace('{level}', tr.levels[eh.pendingLevel] ?? String(eh.pendingLevel)).replace('{time}', fmtMMSS(eh.dwellRemainingSecs))
                : tr.holdDeadband.replace('{dh}', dhStr)}
            </div>
          )}
          {hum.active ? (
            <>
              <div class="stat"><span class="label">{tr.indoorAir}</span><span class="value">{n1(hum.indoorTemp, '°')} · {n1(hum.indoorRh, ' %')} · {n1(hum.indoorAh, ' g/m³')} · {n1(hum.indoorEnthalpy, ' kJ/kg')}</span></div>
              <div class="stat"><span class="label">{tr.outdoorAir}</span><span class="value">{n1(hum.outdoorTemp, '°')} · {n1(hum.outdoorRh, ' %')} · {n1(hum.outdoorAh, ' g/m³')} · {n1(hum.outdoorEnthalpy, ' kJ/kg')}</span></div>
              <div class="stat"><span class="label">{tr.ambientPressure}</span><span class="value">{(hum.ambientPressureKpa * 10).toFixed(0)} hPa</span></div>
              <p class="hint" style="margin-top:8px">{tr.aggregateHint}</p>
              {eh.protectionActive && <div class="msg warning">{tr.protectionActiveMsg}</div>}
            </>
          ) : (
            <div class="msg">{status.status.connected ? tr.tempOnlyFallback : tr.waitingForUnit}</div>
          )}
        </div>
      )}

      {hum.sensors.length > 0 && (
        <div class="card">
          <h3>{tr.humiditySensors}</h3>
          {hum.sensors.map((sn) => (
            <div class="stat">
              <span class="label"><span class={`dot ${sn.fresh ? '' : 'warn'}`} />{sn.role === 'outdoor' ? tr.outdoor : tr.indoor} · <span class="mono">{sn.topic}</span></span>
              <span class="value">
                {sn.humidity != null ? `${sn.humidity.toFixed(0)} %` : ''}{sn.temperature != null ? `${sn.humidity != null ? ' · ' : ''}${fx(lang, sn.temperature)}°` : ''}{sn.pressure != null ? ` · ${sn.pressure.toFixed(0)} hPa` : ''}{sn.fresh ? '' : ` · ${tr.stale}`}
              </span>
            </div>
          ))}
        </div>
      )}

      <div class="card">
        <h3>{tr.unit}</h3>
        <div class="stat"><span class="label">OpenTherm</span><span class="value"><span class={`dot ${status.status.connected ? '' : 'bad'}`} />{status.status.connected ? tr.connected : tr.disconnected}</span></div>
        <div class="stat"><span class="label">Filter</span><span class="value"><span class={`dot ${status.status.filter ? 'warn' : ''}`} />{status.status.filter ? tr.filterChange : 'OK'}</span></div>
        <div class="stat"><span class="label">{tr.bypassMode}</span><span class="value">{status.status.bypass ? tr.open : tr.closed}</span></div>
      </div>
    </>
  )
}
