import { useState, useEffect } from 'preact/hooks'
import { resumeSchedule } from './api'
import type { Status } from './api'

const LEVELS = ['Off', 'Reduced', 'Normal', 'Party'] as const

export function StatusTab({ status, onLevelChange, onCancelOff, onConfirmed }: {
  status: Status | null
  onLevelChange: (level: number) => void
  onCancelOff: () => void
  onConfirmed?: () => void
}) {
  const [localPending, setLocalPending] = useState<number | null>(null)

  // Clear pending when CWL confirms the new level
  useEffect(() => {
    if (localPending !== null && status?.ventilation.level === localPending) {
      setLocalPending(null)
      onConfirmed?.()
    }
  }, [status?.ventilation.level, localPending])

  const handleLevelChange = (level: number) => {
    setLocalPending(level)
    onLevelChange(level)
  }

  if (!status) return <div>Loading...</div>

  const fmtRemaining = (min: number) => {
    const h = Math.floor(min / 60), m = min % 60
    return h > 0 ? `${h}h ${m}m` : `${m}m`
  }

  // Determine which level to show as "selected" (active)
  // If a change is pending, show the pending level as selected (with spinner)
  // Otherwise show the confirmed level
  const displayLevel = localPending ?? status.ventilation.level

  return (
    <>
      {status.timedOff?.active && (
        <div class="msg error" style="display:flex;justify-content:space-between;align-items:center">
          <span>Ventilation Off — resumes in {fmtRemaining(status.timedOff.remainingMinutes)}</span>
          <button class="danger" style="padding:6px 12px;font-size:0.8em;margin:0" onClick={onCancelOff}>Cancel</button>
        </div>
      )}
      <div class="card">
        <h3>Ventilation</h3>
        <div class="level-buttons">
          {LEVELS.map((name, i) => {
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
        <h3>Temperatures</h3>
        <div class="stat"><span class="label">Supply inlet</span><span class="value">{status.temperature.supplyInlet.toFixed(1)} °C</span></div>
        <div class="stat"><span class="label">Exhaust inlet</span><span class="value">{status.temperature.exhaustInlet.toFixed(1)} °C</span></div>
      </div>
      <div class="card">
        <h3>Status</h3>
        <div class="stat"><span class="label">Connected</span><span class={`value ${status.status.connected ? 'ok' : 'fault'}`}>{status.status.connected ? 'Yes' : 'No'}</span></div>
        <div class="stat"><span class="label">Fault</span><span class={`value ${status.status.fault ? 'fault' : 'ok'}`}>{status.status.fault ? 'YES' : 'No'}</span></div>
        <div class="stat"><span class="label">Filter</span><span class={`value ${status.status.filter ? 'fault' : 'ok'}`}>{status.status.filter ? 'Replace' : 'OK'}</span></div>
        <div class="stat"><span class="label">Bypass</span><span class="value">{status.status.bypass ? 'Open' : 'Closed'}</span></div>
      </div>
    </>
  )
}
