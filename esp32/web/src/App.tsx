import { useState, useEffect, useCallback } from 'preact/hooks'
import { login, getStatus, getConfig, saveConfig, setVentilationLevel, uploadFirmware,
         getSchedules, saveSchedules, getBypassSchedule, saveBypassSchedule, sendEncoderAction,
         cancelTimedOff, downloadBackup, restoreBackup,
         type Status, type Config, type ScheduleEntry, type BypassScheduleData } from './api'
import { StatusTab } from './StatusTab'
import { OledMirror } from './OledMirror'
import { t, initialLang, storeLang, type Lang, type Translations } from './translations'


function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <div class="toggle">
      <label class="toggle-switch">
        <input type="checkbox" checked={checked} onChange={(e) => onChange((e.target as HTMLInputElement).checked)} />
        <span class="toggle-slider" />
      </label>
      <span>{label}</span>
    </div>
  )
}

function LoginPage({ tr, onLogin }: { tr: Translations; onLogin: () => void }) {
  const [user, setUser] = useState('admin')
  const [pass, setPass] = useState('')
  const [error, setError] = useState('')

  const handleSubmit = async (e: Event) => {
    e.preventDefault()
    if (await login(user, pass)) onLogin()
    else setError(tr.invalidCredentials)
  }

  return (
    <div class="login">
      <div class="card">
        <h3>{tr.appTitle} · {tr.loginTitle}</h3>
        {error && <div class="msg error">{error}</div>}
        <form onSubmit={handleSubmit}>
          <label>{tr.username}</label>
          <input type="text" value={user} onInput={(e) => setUser((e.target as HTMLInputElement).value)} />
          <label>{tr.password}</label>
          <input type="password" value={pass} onInput={(e) => setPass((e.target as HTMLInputElement).value)} />
          <button type="submit" class="primary">{tr.login}</button>
        </form>
      </div>
    </div>
  )
}

function parseTimeRange(input: string): { startHour: number; startMinute: number; endHour: number; endMinute: number } | null {
  const match = input.match(/^\s*(\d{1,2})(?::(\d{2}))?\s*-\s*(\d{1,2})(?::(\d{2}))?\s*$/)
  if (!match) return null
  const sh = parseInt(match[1]), sm = parseInt(match[2] || '0')
  const eh = parseInt(match[3]), em = parseInt(match[4] || '0')
  if (sh > 23 || sm > 59 || eh > 23 || em > 59) return null
  return { startHour: sh, startMinute: sm, endHour: eh, endMinute: em }
}

function fmt(h: number, m: number) { return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}` }

function daysStr(tr: Translations, mask: number) {
  if (mask === 0x7F) return tr.everyDay
  if (mask === 0x1F) return tr.weekdays
  if (mask === 0x60) return tr.weekend
  return tr.days.filter((_, i) => mask & (1 << i)).join(', ') || tr.noDays
}

const LEVEL_COLORS = ['var(--lvl-0)', 'var(--lvl-1)', 'var(--lvl-2)', 'var(--lvl-3)'] as const

type TlSeg = { start: number; end: number; level: number; entryIdx: number } // entryIdx=-1 for gaps

function computeDaySegments(entries: ScheduleEntry[], dayIdx: number): TlSeg[] {
  const dayBit = 1 << dayIdx
  const covered = new Uint8Array(1440)   // level+1 per minute
  const source = new Int8Array(1440).fill(-1) // entry index per minute
  entries.forEach((entry, idx) => {
    if (!entry.enabled || !(entry.activeDays & dayBit)) return
    const startMin = entry.startHour * 60 + entry.startMinute
    const endMin = entry.endHour * 60 + entry.endMinute
    const mark = (from: number, to: number) => {
      for (let m = from; m < to; m++) { if (!covered[m]) { covered[m] = entry.ventLevel + 1; source[m] = idx } }
    }
    if (startMin <= endMin) mark(startMin, endMin)
    else { mark(startMin, 1440); mark(0, endMin) }
  })
  const segs: TlSeg[] = []
  let cur = covered[0], curSrc = source[0], segStart = 0
  for (let m = 1; m <= 1440; m++) {
    const v = m < 1440 ? covered[m] : 255
    const s = m < 1440 ? source[m] : -2
    if (v !== cur || s !== curSrc) {
      segs.push({ start: segStart, end: m, level: cur ? cur - 1 : -1, entryIdx: curSrc })
      segStart = m; cur = v; curSrc = s
    }
  }
  return segs
}

const snap15 = (min: number) => Math.round(min / 15) * 15

function WeekTimeline({ tr, entries, airflow, onEntriesChange }: {
  tr: Translations;
  entries: ScheduleEntry[];
  airflow?: { reduced: number; normal: number; party: number };
  onEntriesChange?: (entries: ScheduleEntry[]) => void;
}) {
  const DAY_LABELS = tr.days
  const LEVELS = tr.levels
  const nf = (v: number) => Math.round(v).toLocaleString(tr.locale)
  const rates = [0, airflow?.reduced ?? 100, airflow?.normal ?? 130, airflow?.party ?? 195]
  const editable = !!onEntriesChange

  const [tip, setTip] = useState<{ x: number; y: number; text: string } | null>(null)
  const [dragTime, setDragTime] = useState<{ x: number; y: number; text: string } | null>(null)
  const [dragging, setDragging] = useState<{ entryIdx: number; edge: 'start' | 'end' } | null>(null)
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; seg: TlSeg; dayIdx: number } | null>(null)

  // Current time line
  const [nowMin, setNowMin] = useState(() => { const d = new Date(); return d.getHours() * 60 + d.getMinutes() })
  useEffect(() => {
    const id = setInterval(() => { const d = new Date(); setNowMin(d.getHours() * 60 + d.getMinutes()) }, 60000)
    return () => clearInterval(id)
  }, [])
  const nowPct = nowMin / 1440 * 100
  // JS getDay: 0=Sun, convert to Mon=0
  const todayIdx = (new Date().getDay() + 6) % 7

  const showTip = (e: MouseEvent, seg: TlSeg) => {
    if (dragging) return
    const dur = (seg.end - seg.start) / 60
    const startStr = fmt(Math.floor(seg.start / 60), seg.start % 60)
    const endStr = fmt(Math.floor(seg.end / 60), seg.end % 60)
    let text: string
    if (seg.level < 0) {
      text = `${startStr} – ${endStr} · ${tr.noProgram} · ${dur.toFixed(1)} h`
      if (editable) text += ` · ${tr.clickToAdd}`
    } else {
      const vol = Math.round(dur * rates[seg.level])
      text = `${startStr} – ${endStr} · ${LEVELS[seg.level]} · ${dur.toFixed(1)} h · ${nf(vol)} m³`
      if (editable) text += ` · ${tr.rightClickOptions}`
    }
    setTip({ x: e.clientX, y: e.clientY, text })
  }

  // Convert mouse X to minutes relative to the bar
  const xToMinutes = (e: MouseEvent, bar: HTMLElement): number => {
    const rect = bar.getBoundingClientRect()
    const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
    return snap15(Math.round(pct * 1440))
  }

  // Click gap → add new segment
  const handleGapClick = (e: MouseEvent, dayIdx: number, seg: TlSeg) => {
    if (!onEntriesChange || dragging) return
    const bar = (e.currentTarget as HTMLElement).parentElement!
    const clickMin = xToMinutes(e as unknown as MouseEvent, bar)
    const startMin = snap15(Math.max(seg.start, clickMin - 60))
    const endMin = snap15(Math.min(seg.end, clickMin + 60))
    if (endMin - startMin < 15) return
    const activeDays = dayIdx < 5 ? 0x1F : 0x60 // weekdays or weekend
    const newEntry: ScheduleEntry = {
      enabled: true,
      startHour: Math.floor(startMin / 60), startMinute: startMin % 60,
      endHour: Math.floor(endMin / 60), endMinute: endMin % 60,
      ventLevel: 2, activeDays
    }
    onEntriesChange([...entries, newEntry])
  }

  // Context menu on segment
  const handleSegContext = (e: MouseEvent, seg: TlSeg, dayIdx: number) => {
    if (!onEntriesChange || seg.entryIdx < 0) return
    e.preventDefault()
    setTip(null)
    setCtxMenu({ x: e.clientX, y: e.clientY, seg, dayIdx })
  }

  const setLevel = (level: number) => {
    if (!ctxMenu || !onEntriesChange || ctxMenu.seg.entryIdx < 0) return
    const updated = [...entries]
    updated[ctxMenu.seg.entryIdx] = { ...updated[ctxMenu.seg.entryIdx], ventLevel: level }
    onEntriesChange(updated)
    setCtxMenu(null)
  }

  const splitSegment = () => {
    if (!ctxMenu || !onEntriesChange || ctxMenu.seg.entryIdx < 0) return
    const seg = ctxMenu.seg
    const midMin = snap15(Math.floor((seg.start + seg.end) / 2))
    if (midMin <= seg.start || midMin >= seg.end) { setCtxMenu(null); return }

    const entry = entries[ctxMenu.seg.entryIdx]
    const updated = [...entries]
    // Shrink original to first half
    updated[ctxMenu.seg.entryIdx] = {
      ...entry,
      endHour: Math.floor(midMin / 60) % 24, endMinute: midMin % 60
    }
    // Add second half as new entry
    updated.push({
      ...entry,
      startHour: Math.floor(midMin / 60) % 24, startMinute: midMin % 60
    })
    onEntriesChange(updated)
    setCtxMenu(null)
  }

  const deleteSegment = () => {
    if (!ctxMenu || !onEntriesChange || ctxMenu.seg.entryIdx < 0) return
    onEntriesChange(entries.filter((_, i) => i !== ctxMenu.seg.entryIdx))
    setCtxMenu(null)
  }

  // Close context menu on outside click
  useEffect(() => {
    if (!ctxMenu) return
    const close = () => setCtxMenu(null)
    window.addEventListener('click', close)
    return () => window.removeEventListener('click', close)
  }, [ctxMenu])

  // Drag start
  const handleDragStart = (e: MouseEvent, entryIdx: number, edge: 'start' | 'end') => {
    if (!onEntriesChange) return
    e.preventDefault()
    e.stopPropagation()
    setTip(null)
    setDragging({ entryIdx, edge })

    const onMove = (me: MouseEvent) => {
      const bar = document.querySelector('.tl-bar') as HTMLElement
      if (!bar) return
      const min = xToMinutes(me, bar)
      const updated = [...entries]
      const entry = { ...updated[entryIdx] }
      if (edge === 'start') {
        entry.startHour = Math.floor(min / 60); entry.startMinute = min % 60
      } else {
        const endMin = min === 0 ? 1440 : min
        entry.endHour = Math.floor(endMin / 60) % 24; entry.endMinute = endMin % 60
      }
      updated[entryIdx] = entry
      onEntriesChange(updated)
      const displayMin = edge === 'start' ? min : (min === 0 ? 1440 : min)
      const hh = String(Math.floor(displayMin / 60) % 24).padStart(2, '0')
      const mm = String(displayMin % 60).padStart(2, '0')
      setDragTime({ x: me.clientX, y: me.clientY, text: `${hh}:${mm}` })
    }

    const onUp = () => {
      setDragging(null)
      setDragTime(null)
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', onUp)
    }

    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', onUp)
  }

  return (
    <div class="card">
      <h3>{tr.weeklyOverview}</h3>
      {editable && <p class="hint" style="margin-top:-6px">{tr.weeklyOverviewHint}</p>}
      <div class="tl-axis">
        <div class="tl-label" />
        {[0, 6, 12, 18, 24].map(h => (
          <span class="tl-tick" style={`left:${h / 24 * 100}%`}>{h}</span>
        ))}
      </div>
      {DAY_LABELS.map((day, dayIdx) => {
        const segs = computeDaySegments(entries, dayIdx)
        const counts = computeDayCoverage(entries, dayIdx)
        const volume = counts.reduce((sum, min, lvl) => lvl < 4 ? sum + (min / 60) * rates[lvl] : sum, 0)
        const gapHours = (counts[4] + counts[0]) / 60
        return (
          <div class="tl-row" key={dayIdx}>
            <div class="tl-label">{day}</div>
            <div class="tl-bar">
              <div class={`tl-now ${dayIdx === todayIdx ? 'tl-now-today' : ''}`} style={`left:${nowPct}%`} />
              {segs.map((seg, i) => {
                const left = seg.start / 1440 * 100
                const width = (seg.end - seg.start) / 1440 * 100
                const isGap = seg.level < 0
                return (
                  <div key={i}
                    class={`tl-seg ${isGap ? 'tl-gap' : ''} ${editable ? 'tl-editable' : ''}`}
                    style={`left:${left}%;width:${width}%;${isGap ? '' : `background:${LEVEL_COLORS[seg.level]}`}`}
                    onMouseEnter={(e) => showTip(e as unknown as MouseEvent, seg)}
                    onMouseMove={(e) => !dragging && setTip(t => t ? { ...t, x: (e as unknown as MouseEvent).clientX, y: (e as unknown as MouseEvent).clientY } : null)}
                    onMouseLeave={() => setTip(null)}
                    onClick={(e) => { setCtxMenu(null); if (isGap) handleGapClick(e as unknown as MouseEvent, dayIdx, seg) }}
                    onContextMenu={(e) => { if (!isGap) handleSegContext(e as unknown as MouseEvent, seg, dayIdx) }}
                  >
                    {editable && !isGap && seg.entryIdx >= 0 && (
                      <>
                        <div class="tl-handle tl-handle-l" onMouseDown={(e) => handleDragStart(e as unknown as MouseEvent, seg.entryIdx, 'start')} />
                        <div class="tl-handle tl-handle-r" onMouseDown={(e) => handleDragStart(e as unknown as MouseEvent, seg.entryIdx, 'end')} />
                      </>
                    )}
                  </div>
                )
              })}
            </div>
            <div class={`tl-vol ${gapHours > 4 ? 'tl-warn' : ''}`}>{nf(volume)} m³</div>
          </div>
        )
      })}
      <div class="tl-legend">
        {LEVELS.map((name, i) => (
          <span class="tl-legend-item" key={i}><span class="tl-swatch" style={`background:${LEVEL_COLORS[i]}`} />{name}</span>
        ))}
        <span class="tl-legend-item"><span class="tl-swatch tl-gap" />{tr.noProgram}</span>
      </div>
      {dragTime && <div class="tl-drag-time" style={`left:${dragTime.x + 12}px;top:${dragTime.y - 28}px`}>{dragTime.text}</div>}
      {tip && !ctxMenu && !dragTime && <div class="tl-tooltip" style={`left:${tip.x + 12}px;top:${tip.y - 10}px`}>{tip.text}</div>}
      {ctxMenu && (
        <div class="tl-ctx" style={`left:${ctxMenu.x}px;top:${ctxMenu.y}px`} onClick={(e) => e.stopPropagation()}>
          <div class="tl-ctx-header">{fmt(Math.floor(ctxMenu.seg.start / 60), ctxMenu.seg.start % 60)} – {fmt(Math.floor(ctxMenu.seg.end / 60), ctxMenu.seg.end % 60)}</div>
          {LEVELS.map((name, i) => (
            <div key={i} class={`tl-ctx-item ${ctxMenu.seg.level === i ? 'active' : ''}`} onClick={() => setLevel(i)}>
              <span class="tl-swatch" style={`background:${LEVEL_COLORS[i]}`} />{name}
            </div>
          ))}
          <div class="tl-ctx-sep" />
          <div class="tl-ctx-item" onClick={splitSegment}>{tr.split}</div>
          <div class="tl-ctx-item tl-ctx-danger" onClick={deleteSegment}>{tr.delete}</div>
        </div>
      )}
    </div>
  )
}

// Compute per-level minute counts for a given day
function computeDayCoverage(entries: ScheduleEntry[], dayIdx: number): number[] {
  const dayBit = 1 << dayIdx
  const covered = new Uint8Array(1440) // 0 = uncovered
  for (const entry of entries) {
    if (!entry.enabled || !(entry.activeDays & dayBit)) continue
    const startMin = entry.startHour * 60 + entry.startMinute
    const endMin = entry.endHour * 60 + entry.endMinute
    const mark = (from: number, to: number) => {
      for (let m = from; m < to; m++) { if (!covered[m]) covered[m] = entry.ventLevel + 1 }
    }
    if (startMin <= endMin) mark(startMin, endMin)
    else { mark(startMin, 1440); mark(0, endMin) }
  }
  // Count minutes per level: [off, reduced, normal, party, uncovered]
  const counts = [0, 0, 0, 0, 0]
  for (let m = 0; m < 1440; m++) {
    if (covered[m]) counts[covered[m] - 1]++
    else counts[4]++
  }
  return counts
}

// DailyVolume removed — volume is now integrated into WeekTimeline rows

function ScheduleDialog({ tr, entry, onSave, onCancel }: { tr: Translations; entry: ScheduleEntry; onSave: (e: ScheduleEntry) => void; onCancel: () => void }) {
  const [draft, setDraft] = useState<ScheduleEntry>({ ...entry })
  const [timeText, setTimeText] = useState(`${fmt(entry.startHour, entry.startMinute)} - ${fmt(entry.endHour, entry.endMinute)}`)
  const [error, setError] = useState('')

  const handleSave = () => {
    const parsed = parseTimeRange(timeText)
    if (!parsed) { setError(tr.invalidRange); return }
    if (draft.activeDays === 0) { setError(tr.selectDay); return }
    onSave({ ...draft, ...parsed })
  }

  return (
    <div class="modal-overlay">
      <div class="modal">
        <h3>{tr.schedule}</h3>
        {error && <div class="msg error">{error}</div>}

        <Toggle checked={draft.enabled} onChange={(v) => setDraft({ ...draft, enabled: v })} label={tr.enabled} />

        <label style="margin-top:8px;display:block">{tr.timeRange}</label>
        <input type="text" value={timeText} placeholder="08:00 - 22:00" style="margin-bottom:4px"
          onInput={(e) => { setTimeText((e.target as HTMLInputElement).value); setError('') }} />
        <p class="hint" style="margin-bottom:12px">{tr.formatsHint}</p>

        <label>{tr.level}</label>
        <div class="level-buttons" style="margin-top:4px">
          {tr.levels.map((name, i) => (
            <div class={`level-btn ${draft.ventLevel === i ? 'active' : ''}`}
                 onClick={() => setDraft({ ...draft, ventLevel: i })}>{name}</div>
          ))}
        </div>

        <label>{tr.daysLabel}</label>
        <div class="days">
          {tr.days.map((day, i) => (
            <div class={`level-btn ${draft.activeDays & (1 << i) ? 'active' : ''}`}
                 onClick={() => setDraft({ ...draft, activeDays: draft.activeDays ^ (1 << i) })}>{day}</div>
          ))}
        </div>

        <div class="row end">
          <button onClick={onCancel}>{tr.cancel}</button>
          <button class="primary" onClick={handleSave}>{tr.save}</button>
        </div>
      </div>
    </div>
  )
}

function SchedulesTab({ status, airflow, lang, onRefresh }: { status: Status | null; airflow?: { reduced: number; normal: number; party: number }; lang: Lang; onRefresh?: () => void }) {
  const [entries, setEntries] = useState<ScheduleEntry[]>([])
  const [bypass, setBypass] = useState<BypassScheduleData>({ enabled: false, startDay: 15, startMonth: 4, endDay: 30, endMonth: 9 })
  const [msg, setMsg] = useState<{ type: string; text: string } | null>(null)
  const [editIdx, setEditIdx] = useState<number | null>(null)
  const [ehEnabled, setEhEnabled] = useState(false)
  const tr = t(lang)

  useEffect(() => {
    getSchedules().then(setEntries)
    getBypassSchedule().then(setBypass)
  }, [])

  // Reflect the persisted extreme-heat state as it arrives via polling.
  useEffect(() => {
    if (status?.extremeHeat) setEhEnabled(status.extremeHeat.enabled)
  }, [status?.extremeHeat?.enabled])

  const toggleExtremeHeat = async (v: boolean) => {
    setEhEnabled(v) // optimistic; next poll confirms
    await saveConfig({ extremeHeat: { enabled: v } })
    onRefresh?.()
  }

  const handleDialogSave = (entry: ScheduleEntry) => {
    if (editIdx !== null && editIdx < entries.length) {
      const updated = [...entries]
      updated[editIdx] = entry
      setEntries(updated)
    } else {
      setEntries([...entries, entry])
    }
    setEditIdx(null)
  }

  const removeEntry = (idx: number) => {
    setEntries(entries.filter((_, i) => i !== idx))
  }

  const handleSave = async () => {
    const ok1 = await saveSchedules(entries)
    const ok2 = await saveBypassSchedule(bypass)
    if (ok1 && ok2) setMsg({ type: 'success', text: tr.schedulesSaved })
    else setMsg({ type: 'error', text: tr.failedToSave })
  }

  return (
    <>
      {msg && <div class={`msg ${msg.type}`}>{msg.text}</div>}

      <div class="card">
        <h3>{tr.extremeHeatMode}</h3>
        <div class="toggle">
          <label class="toggle-switch">
            <input type="checkbox" checked={ehEnabled} onChange={(e) => toggleExtremeHeat((e.target as HTMLInputElement).checked)} />
            <span class="toggle-slider" />
          </label>
          <span>{ehEnabled ? tr.on : tr.off}</span>
        </div>
        <p class="hint">{tr.extremeHeatHint}</p>
      </div>

      {ehEnabled && <div class="msg warning">{tr.extremeHeatOverridesSchedules}</div>}

      <div class="col" style={ehEnabled ? 'opacity:0.45;pointer-events:none' : ''} aria-disabled={ehEnabled}>
      <h2>{tr.ventilationSchedules}</h2>
      <WeekTimeline tr={tr} entries={entries} airflow={airflow} onEntriesChange={setEntries} />
      {(() => {
        type Group = { label: string; items: { entry: ScheduleEntry; idx: number }[] }
        const groups: Group[] = []
        const groupMap: Record<string, Group> = {}

        entries.forEach((entry, idx) => {
          const label = daysStr(tr, entry.activeDays)
          if (!groupMap[label]) {
            groupMap[label] = { label, items: [] }
            groups.push(groupMap[label])
          }
          groupMap[label].items.push({ entry, idx })
        })

        return groups.map((group) => (
          <div key={group.label} class="card">
            <h3>{group.label}</h3>
            <div>
              {group.items.sort((a, b) => (a.entry.startHour * 60 + a.entry.startMinute) - (b.entry.startHour * 60 + b.entry.startMinute)).map(({ entry, idx }) => (
                <div class="sched-row" key={idx}>
                  <div class="sched-info" style={entry.enabled ? '' : 'opacity:0.5'}>
                    <div class="sched-time">{fmt(entry.startHour, entry.startMinute)} – {fmt(entry.endHour, entry.endMinute)}</div>
                    <div class="sched-detail"><span class="tl-swatch" style={`width:8px;height:8px;border-radius:50%;background:${LEVEL_COLORS[entry.ventLevel]}`} />{tr.levels[entry.ventLevel]}</div>
                  </div>
                  <div class="row">
                    <button class="sm" onClick={() => setEditIdx(idx)}>{tr.edit}</button>
                    <button class="sm danger" onClick={() => removeEntry(idx)}>{tr.delete}</button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))
      })()}
      {entries.length < 16 && <button onClick={() => setEditIdx(entries.length)} style="align-self:flex-start">{tr.addSchedule}</button>}
      </div>

      {editIdx !== null && (
        <ScheduleDialog
          tr={tr}
          entry={editIdx < entries.length ? entries[editIdx] : { enabled: true, startHour: 8, startMinute: 0, endHour: 22, endMinute: 0, ventLevel: 2, activeDays: 0x7F }}
          onSave={handleDialogSave}
          onCancel={() => setEditIdx(null)}
        />
      )}

      <div class="col" style={ehEnabled ? 'opacity:0.45;pointer-events:none' : ''} aria-disabled={ehEnabled}>
      <div class="card">
        <h3>{tr.summerMode}</h3>
        <p class="hint" style="margin-top:-6px">{tr.freeCoolingHint}</p>
        <Toggle checked={bypass.enabled} onChange={(v) => setBypass({ ...bypass, enabled: v })} label={tr.enableSummer} />
        {bypass.enabled && <div class="row" style="margin-top:8px">
          <label>{tr.bypassFrom}</label>
          <input type="number" min="1" max="31" value={bypass.startDay} style="width:64px"
            onInput={(e) => setBypass({ ...bypass, startDay: parseInt((e.target as HTMLInputElement).value) })} />
          <select value={bypass.startMonth} onChange={(e) => setBypass({ ...bypass, startMonth: parseInt((e.target as HTMLSelectElement).value) })}>
            {tr.months.map((m, i) => <option value={i + 1}>{m}</option>)}
          </select>
          <label>{tr.bypassTo}</label>
          <input type="number" min="1" max="31" value={bypass.endDay} style="width:64px"
            onInput={(e) => setBypass({ ...bypass, endDay: parseInt((e.target as HTMLInputElement).value) })} />
          <select value={bypass.endMonth} onChange={(e) => setBypass({ ...bypass, endMonth: parseInt((e.target as HTMLSelectElement).value) })}>
            {tr.months.map((m, i) => <option value={i + 1}>{m}</option>)}
          </select>
        </div>}
      </div>
      </div>

      <button class="primary" onClick={handleSave} style="align-self:flex-start">{tr.saveAllSchedules}</button>
    </>
  )
}

function SettingsTab({ lang }: { lang: Lang }) {
  const [config, setConfig] = useState<Config | null>(null)
  const [msg, setMsg] = useState<{ type: string; text: string } | null>(null)
  const tr = t(lang)

  useEffect(() => { getConfig().then(setConfig) }, [])

  const handleSave = async () => {
    if (!config) return
    if (await saveConfig(config)) setMsg({ type: 'success', text: tr.savedReboot })
    else setMsg({ type: 'error', text: tr.failedToSave })
  }

  if (!config) return <div class="msg">{tr.loading}</div>

  const update = (section: string, field: string, value: string | number | boolean) => {
    setConfig({ ...config, [section]: { ...(config as unknown as Record<string, Record<string, unknown>>)[section], [field]: value } } as Config)
  }

  const hum = config.humidity ?? { insideTopics: [], outsideTopics: [], protectionEnabled: false }
  const setHum = (h: { insideTopics: string[]; outsideTopics: string[]; protectionEnabled: boolean }) =>
    setConfig({ ...config, humidity: h })
  const val = (e: Event) => (e.target as HTMLInputElement).value

  const topicList = (key: 'outsideTopics' | 'insideTopics', placeholder: string) => hum[key].map((topic, i) => (
    <div class="row" style="margin-bottom:6px">
      <input type="text" class="mono" value={topic} placeholder={placeholder}
        onInput={(e) => { const arr = [...hum[key]]; arr[i] = val(e); setHum({ ...hum, [key]: arr }) }} />
      <button class="danger" aria-label={tr.delete} onClick={() => setHum({ ...hum, [key]: hum[key].filter((_, j) => j !== i) })}>✕</button>
    </div>
  ))

  return (
    <>
      {msg && <div class={`msg ${msg.type}`}>{msg.text}</div>}
      <div class="card">
        <h3>{tr.displayLanguage}</h3>
        <p class="hint" style="margin-top:-6px">{tr.displayLanguageHint}</p>
        <select value={config.language ?? 'en'} onChange={(e) => setConfig({ ...config, language: (e.target as HTMLSelectElement).value })}>
          <option value="en">{tr.english}</option>
          <option value="de">{tr.german}</option>
        </select>
      </div>
      <div class="card">
        <h3>{tr.wifi}</h3>
        <label>{tr.ssid}</label>
        <input type="text" value={config.network.wifiSsid} onInput={(e) => update('network', 'wifiSsid', val(e))} />
        <label>{tr.password}</label>
        <input type="password" value={config.network.wifiPassword} onInput={(e) => update('network', 'wifiPassword', val(e))} />
      </div>
      <div class="card">
        <h3>{tr.mqtt}</h3>
        <Toggle checked={config.mqtt.enabled} onChange={(v) => update('mqtt', 'enabled', v)} label={tr.enabled} />
        <label>{tr.server}</label>
        <input type="text" value={config.mqtt.server} onInput={(e) => update('mqtt', 'server', val(e))} />
        <label>{tr.port}</label>
        <input type="number" value={config.mqtt.port} onInput={(e) => update('mqtt', 'port', parseInt(val(e)))} />
        <label>{tr.topic}</label>
        <input type="text" value={config.mqtt.topic} onInput={(e) => update('mqtt', 'topic', val(e))} />
        <Toggle checked={config.mqtt.authEnabled} onChange={(v) => update('mqtt', 'authEnabled', v)} label={tr.authEnabled} />
        {config.mqtt.authEnabled && <>
          <label>{tr.username}</label>
          <input type="text" value={config.mqtt.username} onInput={(e) => update('mqtt', 'username', val(e))} />
          <label>{tr.password}</label>
          <input type="password" value={config.mqtt.password} onInput={(e) => update('mqtt', 'password', val(e))} />
        </>}
      </div>
      <div class="card">
        <h3>{tr.humiditySensors}</h3>
        <p class="hint" style="margin-top:-6px">{tr.humiditySensorsHint}</p>
        <Toggle checked={hum.protectionEnabled} onChange={(v) => setHum({ ...hum, protectionEnabled: v })} label={tr.moistureProtection} />
        <label style="margin-top:10px;display:block">{tr.outdoorSensorTopics}</label>
        <p class="hint" style="margin-top:0">{tr.outdoorSensorTopicsHint}</p>
        {topicList('outsideTopics', 'garden/weather/outdoor')}
        <button class="sm" onClick={() => setHum({ ...hum, outsideTopics: [...hum.outsideTopics, ''] })}>{tr.addOutdoorSensor}</button>
        <label style="margin-top:14px;display:block;margin-bottom:6px">{tr.indoorSensorTopics}</label>
        {topicList('insideTopics', 'zigbee2mqtt/og_temp_bad')}
        <button class="sm" onClick={() => setHum({ ...hum, insideTopics: [...hum.insideTopics, ''] })}>{tr.addIndoorSensor}</button>
      </div>
      <div class="card">
        <h3>{tr.webUi}</h3>
        {/* Firmware predating the toggle omits the field — treat it as "login required". */}
        <Toggle checked={config.web.authEnabled !== false} onChange={(v) => update('web', 'authEnabled', v)} label={tr.requireLogin} />
        {config.web.authEnabled === false ? (
          <div class="msg warning">{tr.noLoginWarning}</div>
        ) : (
          <>
            <label>{tr.username}</label>
            <input type="text" value={config.web.username} onInput={(e) => update('web', 'username', val(e))} />
            <label>{tr.password}</label>
            <input type="password" value={config.web.password} onInput={(e) => update('web', 'password', val(e))} />
          </>
        )}
      </div>
      <button class="primary" style="align-self:flex-start" onClick={handleSave}>{tr.saveSettings}</button>

      <div class="card">
        <h3>{tr.backupRestore}</h3>
        <p class="hint" style="margin-top:-6px">{tr.backupHint}</p>
        <div class="row">
          <button onClick={() => downloadBackup()}>{tr.exportBackup}</button>
          <label class="btn">
            {tr.importBackup}
            <input type="file" accept=".json" style="display:none" onChange={async (e) => {
              const f = (e.target as HTMLInputElement).files?.[0]
              if (f && await restoreBackup(f)) {
                setMsg({ type: 'success', text: tr.backupRestored })
                getConfig().then(setConfig)
              } else {
                setMsg({ type: 'error', text: tr.restoreFailed })
              }
            }} />
          </label>
        </div>
      </div>
    </>
  )
}

function SystemTab({ status, lang }: { status: Status | null; lang: Lang }) {
  const [file, setFile] = useState<File | null>(null)
  const [uploading, setUploading] = useState(false)
  const [msg, setMsg] = useState<{ type: string; text: string } | null>(null)
  const tr = t(lang)

  const handleUpload = async () => {
    if (!file) return
    setUploading(true)
    setMsg(null)
    if (await uploadFirmware(file)) setMsg({ type: 'success', text: tr.updateSuccessful })
    else setMsg({ type: 'error', text: tr.updateFailed })
    setUploading(false)
  }

  const formatUptime = (seconds: number) => {
    const d = Math.floor(seconds / 86400)
    const h = Math.floor((seconds % 86400) / 3600)
    const m = Math.floor((seconds % 3600) / 60)
    return d > 0 ? `${d} d ${h} h` : `${h} h ${m} min`
  }

  return (
    <>
      {status && <div class="card">
        <h3>{tr.systemInfo}</h3>
        <div class="stat"><span class="label">{tr.version}</span><span class="value mono">{status.system.version}</span></div>
        <div class="stat"><span class="label">{tr.uptime}</span><span class="value">{formatUptime(status.system.uptime)}</span></div>
        <div class="stat"><span class="label">{tr.freeHeap}</span><span class="value">{Math.round(status.system.freeHeap / 1024)} KB</span></div>
        <div class="stat"><span class="label">{tr.network}</span><span class="value">{status.system.wifiRssi ? `${tr.wifi} · ${status.system.wifiRssi} dBm` : tr.ethernet}</span></div>
        <div class="stat"><span class="label">MQTT</span><span class="value"><span class={`dot ${status.system.mqttConnected ? '' : 'bad'}`} />{status.system.mqttConnected ? tr.connected : tr.disconnected}</span></div>
      </div>}
      <div class="card">
        <h3>{tr.firmwareUpdate}</h3>
        <input type="file" accept=".bin" onChange={(e) => setFile((e.target as HTMLInputElement).files?.[0] ?? null)} />
        <div><button onClick={handleUpload} disabled={!file || uploading}>{uploading ? tr.uploading : tr.uploadFirmware}</button></div>
        {msg && <div class={`msg ${msg.type}`}>{msg.text}</div>}
      </div>
    </>
  )
}

function DebugTab({ tr }: { tr: Translations }) {
  return (
    <>
      <OledMirror title={tr.oledDisplay} na={tr.displayNA} />
      <div class="encoder-bar">
        <button class="enc-btn" onClick={() => sendEncoderAction('left')}>&#9664; {tr.left}</button>
        <button class="enc-btn primary" onClick={() => sendEncoderAction('press')}>{tr.press}</button>
        <button class="enc-btn" onClick={() => sendEncoderAction('right')}>{tr.right} &#9654;</button>
      </div>
    </>
  )
}

const FAN = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M10.827 16.379a6.082 6.082 0 0 1-8.618-7.002l5.412 1.45a6.082 6.082 0 0 1 7.002-8.618l-1.45 5.412a6.082 6.082 0 0 1 8.618 7.002l-5.412-1.45a6.082 6.082 0 0 1-7.002 8.618l1.45-5.412Z" />
    <path d="M12 12v.01" />
  </svg>
)

export function App() {
  const [authenticated, setAuthenticated] = useState(false)
  const [tab, setTab] = useState(0)
  const [status, setStatus] = useState<Status | null>(null)
  const [pendingAction, setPendingAction] = useState(false)
  const [lang, setLangState] = useState<Lang>(initialLang)
  const setLang = (l: Lang) => { setLangState(l); storeLang(l); document.documentElement.lang = l }
  const tr = t(lang)

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await getStatus())
    } catch (e) {
      if ((e as Error).message === 'unauthorized') setAuthenticated(false)
    }
  }, [])

  useEffect(() => {
    if (!authenticated) return
    refreshStatus()
    const interval = setInterval(refreshStatus, pendingAction ? 1000 : 5000)
    return () => clearInterval(interval)
  }, [authenticated, refreshStatus, pendingAction])

  useEffect(() => {
    document.documentElement.lang = lang
    getStatus().then((s) => { setStatus(s); setAuthenticated(true) }).catch(() => {})
  }, [])

  if (!authenticated) return <LoginPage tr={tr} onLogin={() => setAuthenticated(true)} />

  const online = !!status && status.system.mqttConnected && status.status.connected
  const tabs = [tr.status, tr.schedules, tr.settings, tr.system, tr.debug]

  return (
    <>
      <header class="top-bar">
        <div class="wrap">
          <div class="app-head">
            <span class="ico">{FAN}</span>
            <div class="hd">
              <h1 class="tt">{tr.appTitle}</h1>
              <div class="st">
                <span class={`dot ${online ? 'live' : 'bad'}`} />
                {online ? tr.connected : tr.disconnected}
                <span class="mono">· Wolf CWL{status ? ` ${status.system.version}` : ''}</span>
              </div>
            </div>
            <div class="seg" role="group" aria-label={tr.language}>
              {(['de', 'en'] as Lang[]).map((l) => (
                <button type="button" aria-pressed={lang === l} onClick={() => setLang(l)}>{l.toUpperCase()}</button>
              ))}
            </div>
          </div>
          <nav class="app-tabs">
            {tabs.map((name, i) => (
              <button type="button" class={tab === i ? 'active' : ''} aria-current={tab === i ? 'page' : undefined} onClick={() => setTab(i)}>{name}</button>
            ))}
          </nav>
        </div>
      </header>
      <main class={`wrap${tab === 4 ? ' has-bar' : ''}`}>
        {status?.system.simulated && <div class="msg warning">{tr.simulation}</div>}
        {tab === 0 && <StatusTab status={status} lang={lang} onLevelChange={async (level) => { setPendingAction(true); await setVentilationLevel(level) }} onCancelOff={async () => { await cancelTimedOff(); refreshStatus() }} onConfirmed={() => setPendingAction(false)} />}
        {tab === 1 && <SchedulesTab status={status} airflow={status?.airflow} lang={lang} onRefresh={refreshStatus} />}
        {tab === 2 && <SettingsTab lang={lang} />}
        {tab === 3 && <SystemTab status={status} lang={lang} />}
        {tab === 4 && <DebugTab tr={tr} />}
      </main>
    </>
  )
}
