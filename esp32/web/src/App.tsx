import { useState, useEffect, useCallback } from 'preact/hooks'
import { login, getStatus, getConfig, saveConfig, setVentilationLevel, uploadFirmware,
         getSchedules, saveSchedules, getBypassSchedule, saveBypassSchedule, sendEncoderAction,
         cancelTimedOff, downloadBackup, restoreBackup,
         type Status, type Config, type ScheduleEntry, type BypassScheduleData } from './api'
import { StatusTab } from './StatusTab'
import { OledMirror } from './OledMirror'
import { t, type Lang } from './translations'

const LEVELS = ['Off', 'Reduced', 'Normal', 'Party'] as const
const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] as const
const MONTHS = ['', 'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'] as const

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

function LoginPage({ onLogin }: { onLogin: () => void }) {
  const [user, setUser] = useState('admin')
  const [pass, setPass] = useState('')
  const [error, setError] = useState('')

  const handleSubmit = async (e: Event) => {
    e.preventDefault()
    if (await login(user, pass)) onLogin()
    else setError('Invalid credentials')
  }

  return (
    <div class="login">
      <div class="card">
        <h3>Wolf CWL Login</h3>
        {error && <div class="msg error">{error}</div>}
        <form onSubmit={handleSubmit}>
          <label>Username</label>
          <input type="text" value={user} onInput={(e) => setUser((e.target as HTMLInputElement).value)} />
          <label>Password</label>
          <input type="password" value={pass} onInput={(e) => setPass((e.target as HTMLInputElement).value)} />
          <button type="submit">Login</button>
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

function daysStr(mask: number) {
  if (mask === 0x7F) return 'Every day'
  if (mask === 0x1F) return 'Weekdays'
  if (mask === 0x60) return 'Weekend'
  return DAYS.filter((_, i) => mask & (1 << i)).join(', ') || 'No days'
}

const LEVEL_COLORS = ['var(--accent-red)', 'var(--accent-orange)', 'var(--accent-blue)', 'var(--accent-green)'] as const

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

function WeekTimeline({ entries, airflow, onEntriesChange }: {
  entries: ScheduleEntry[];
  airflow?: { reduced: number; normal: number; party: number };
  onEntriesChange?: (entries: ScheduleEntry[]) => void;
}) {
  const DAY_LABELS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
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
      text = `${startStr} - ${endStr} · No program · ${dur.toFixed(1)}h`
      if (editable) text += ' · Click to add'
    } else {
      const vol = Math.round(dur * rates[seg.level])
      text = `${startStr} - ${endStr} · ${LEVELS[seg.level]} · ${dur.toFixed(1)}h · ${vol.toLocaleString()} m³`
      if (editable) text += ' · Right-click for options'
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
    <div class="card" style="padding:12px">
      <h3>Weekly Overview {editable && <span style="font-size:0.7em;color:var(--text-muted);font-weight:400">— click gaps to add, right-click segments for options, drag borders</span>}</h3>
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
            <div class={`tl-vol ${gapHours > 4 ? 'tl-warn' : ''}`}>{Math.round(volume).toLocaleString()} m³</div>
          </div>
        )
      })}
      <div class="tl-legend">
        {LEVELS.map((name, i) => (
          <span class="tl-legend-item" key={i}><span class="tl-swatch" style={`background:${LEVEL_COLORS[i]}`} />{name}</span>
        ))}
        <span class="tl-legend-item"><span class="tl-swatch tl-gap" />No program</span>
      </div>
      {dragTime && <div class="tl-drag-time" style={`left:${dragTime.x + 12}px;top:${dragTime.y - 28}px`}>{dragTime.text}</div>}
      {tip && !ctxMenu && !dragTime && <div class="tl-tooltip" style={`left:${tip.x + 12}px;top:${tip.y - 10}px`}>{tip.text}</div>}
      {ctxMenu && (
        <div class="tl-ctx" style={`left:${ctxMenu.x}px;top:${ctxMenu.y}px`} onClick={(e) => e.stopPropagation()}>
          <div class="tl-ctx-header">{fmt(Math.floor(ctxMenu.seg.start / 60), ctxMenu.seg.start % 60)} - {fmt(Math.floor(ctxMenu.seg.end / 60), ctxMenu.seg.end % 60)}</div>
          {LEVELS.map((name, i) => (
            <div key={i} class={`tl-ctx-item ${ctxMenu.seg.level === i ? 'active' : ''}`} onClick={() => setLevel(i)}>
              <span class="tl-swatch" style={`background:${LEVEL_COLORS[i]}`} />{name}
            </div>
          ))}
          <div class="tl-ctx-sep" />
          <div class="tl-ctx-item" onClick={splitSegment}>Split</div>
          <div class="tl-ctx-item tl-ctx-danger" onClick={deleteSegment}>Delete</div>
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

function ScheduleDialog({ entry, onSave, onCancel }: { entry: ScheduleEntry; onSave: (e: ScheduleEntry) => void; onCancel: () => void }) {
  const [draft, setDraft] = useState<ScheduleEntry>({ ...entry })
  const [timeText, setTimeText] = useState(`${fmt(entry.startHour, entry.startMinute)} - ${fmt(entry.endHour, entry.endMinute)}`)
  const [error, setError] = useState('')

  const handleSave = () => {
    const parsed = parseTimeRange(timeText)
    if (!parsed) { setError('Invalid time range. Use e.g. 8:00 - 22:00'); return }
    if (draft.activeDays === 0) { setError('Select at least one day'); return }
    onSave({ ...draft, ...parsed })
  }

  return (
    <div class="modal-overlay">
      <div class="modal">
        <h3 style="margin-bottom:16px">Schedule</h3>
        {error && <div class="msg error">{error}</div>}

        <Toggle checked={draft.enabled} onChange={(v) => setDraft({ ...draft, enabled: v })} label="Enabled" />

        <label style="margin-top:12px;display:block">Time range</label>
        <input type="text" value={timeText} placeholder="08:00 - 22:00" style="margin-bottom:4px"
          onInput={(e) => { setTimeText((e.target as HTMLInputElement).value); setError('') }} />
        <div style="font-size:12px;color:var(--text-muted);margin-bottom:12px">Formats: 8-22, 08:00-22:00, 8:00 - 13:30</div>

        <label>Level</label>
        <div class="level-buttons" style="margin-bottom:12px">
          {LEVELS.map((name, i) => (
            <div class={`level-btn ${draft.ventLevel === i ? 'active' : ''}`}
                 onClick={() => setDraft({ ...draft, ventLevel: i })}>{name}</div>
          ))}
        </div>

        <label>Days</label>
        <div style="display:flex;gap:4px;flex-wrap:wrap;margin-bottom:16px">
          {DAYS.map((day, i) => (
            <div class={`level-btn ${draft.activeDays & (1 << i) ? 'active' : ''}`}
                 style="flex:none;padding:6px 10px;font-size:0.85em"
                 onClick={() => setDraft({ ...draft, activeDays: draft.activeDays ^ (1 << i) })}>{day}</div>
          ))}
        </div>

        <div style="display:flex;gap:8px;justify-content:flex-end">
          <button class="danger" onClick={onCancel}>Cancel</button>
          <button onClick={handleSave}>Save</button>
        </div>
      </div>
    </div>
  )
}

function SchedulesTab({ airflow, lang }: { airflow?: { reduced: number; normal: number; party: number }; lang: Lang }) {
  const [entries, setEntries] = useState<ScheduleEntry[]>([])
  const [bypass, setBypass] = useState<BypassScheduleData>({ enabled: false, startDay: 15, startMonth: 4, endDay: 30, endMonth: 9 })
  const [msg, setMsg] = useState<{ type: string; text: string } | null>(null)
  const [editIdx, setEditIdx] = useState<number | null>(null)

  useEffect(() => {
    getSchedules().then(setEntries)
    getBypassSchedule().then(setBypass)
  }, [])

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
    if (ok1 && ok2) setMsg({ type: 'success', text: 'Schedules saved' })
    else setMsg({ type: 'error', text: 'Failed to save' })
  }

  return (
    <>
      {msg && <div class={`msg ${msg.type}`}>{msg.text}</div>}

      <h2>{t(lang).ventilationSchedules}</h2>
      <WeekTimeline entries={entries} airflow={airflow} onEntriesChange={setEntries} />
      {(() => {
        type Group = { label: string; items: { entry: ScheduleEntry; idx: number }[] }
        const groups: Group[] = []
        const groupMap: Record<string, Group> = {}

        entries.forEach((entry, idx) => {
          const label = daysStr(entry.activeDays)
          if (!groupMap[label]) {
            groupMap[label] = { label, items: [] }
            groups.push(groupMap[label])
          }
          groupMap[label].items.push({ entry, idx })
        })

        return groups.map((group) => (
          <div key={group.label}>
            <h3 style="font-size:0.85em;color:var(--text-muted);margin:12px 0 4px;text-transform:uppercase;letter-spacing:0.5px">{group.label}</h3>
            <div class="card">
              {group.items.sort((a, b) => (a.entry.startHour * 60 + a.entry.startMinute) - (b.entry.startHour * 60 + b.entry.startMinute)).map(({ entry, idx }) => (
                <div class="sched-row" key={idx}>
                  <div class="sched-info" style={entry.enabled ? '' : 'opacity:0.5'}>
                    <div class="sched-time">{fmt(entry.startHour, entry.startMinute)} - {fmt(entry.endHour, entry.endMinute)}</div>
                    <div class="sched-detail">{LEVELS[entry.ventLevel]}</div>
                  </div>
                  <div style="display:flex;gap:4px">
                    <button style="padding:6px 10px;font-size:0.8em" onClick={() => setEditIdx(idx)}>Edit</button>
                    <button class="danger" style="padding:6px 10px;font-size:0.8em" onClick={() => removeEntry(idx)}>Delete</button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))
      })()}
      {entries.length < 16 && <button onClick={() => setEditIdx(entries.length)} style="margin-bottom:16px">{t(lang).addSchedule}</button>}

      {editIdx !== null && (
        <ScheduleDialog
          entry={editIdx < entries.length ? entries[editIdx] : { enabled: true, startHour: 8, startMinute: 0, endHour: 22, endMinute: 0, ventLevel: 2, activeDays: 0x7F }}
          onSave={handleDialogSave}
          onCancel={() => setEditIdx(null)}
        />
      )}

      <h2>{t(lang).summerMode}</h2>
      <div class="card">
        <p style="font-size:0.85em;color:var(--text-muted);margin-bottom:8px">Free cooling — outdoor air bypasses heat exchanger. Enable during warm months.</p>
        <Toggle checked={bypass.enabled} onChange={(v) => setBypass({ ...bypass, enabled: v })} label="Enable summer mode schedule" />
        {bypass.enabled && <div style="display:flex;gap:8px;align-items:center;margin-top:8px;flex-wrap:wrap">
          <label>From</label>
          <input type="number" min="1" max="31" value={bypass.startDay} style="width:60px"
            onInput={(e) => setBypass({ ...bypass, startDay: parseInt((e.target as HTMLInputElement).value) })} />
          <select value={bypass.startMonth} onChange={(e) => setBypass({ ...bypass, startMonth: parseInt((e.target as HTMLSelectElement).value) })}>
            {MONTHS.slice(1).map((m, i) => <option value={i + 1}>{m}</option>)}
          </select>
          <label>to</label>
          <input type="number" min="1" max="31" value={bypass.endDay} style="width:60px"
            onInput={(e) => setBypass({ ...bypass, endDay: parseInt((e.target as HTMLInputElement).value) })} />
          <select value={bypass.endMonth} onChange={(e) => setBypass({ ...bypass, endMonth: parseInt((e.target as HTMLSelectElement).value) })}>
            {MONTHS.slice(1).map((m, i) => <option value={i + 1}>{m}</option>)}
          </select>
        </div>}
      </div>

      <button onClick={handleSave}>{t(lang).saveAllSchedules}</button>
    </>
  )
}

function SettingsTab({ lang, onLangChange }: { lang: Lang; onLangChange: (l: Lang) => void }) {
  const [config, setConfig] = useState<Config | null>(null)
  const [msg, setMsg] = useState<{ type: string; text: string } | null>(null)

  useEffect(() => { getConfig().then(setConfig) }, [])

  const handleSave = async () => {
    if (!config) return
    if (await saveConfig(config)) setMsg({ type: 'success', text: 'Saved. Reboot to apply network changes.' })
    else setMsg({ type: 'error', text: 'Failed to save' })
  }

  if (!config) return <div>Loading...</div>

  const update = (section: string, field: string, value: string | number | boolean) => {
    setConfig({ ...config, [section]: { ...(config as unknown as Record<string, Record<string, unknown>>)[section], [field]: value } } as Config)
  }

  return (
    <>
      {msg && <div class={`msg ${msg.type}`}>{msg.text}</div>}
      <div class="card">
        <h3>{t(lang).language}</h3>
        <select value={config.language ?? 'en'} onChange={(e) => {
          const newLang = (e.target as HTMLSelectElement).value as Lang
          setConfig({ ...config, language: newLang })
          onLangChange(newLang)
        }}>
          <option value="en">{t(lang).english}</option>
          <option value="de">{t(lang).german}</option>
        </select>
      </div>
      <div class="card">
        <h3>{t(lang).wifi}</h3>
        <label>SSID</label>
        <input type="text" value={config.network.wifiSsid} onInput={(e) => update('network', 'wifiSsid', (e.target as HTMLInputElement).value)} />
        <label>Password</label>
        <input type="password" value={config.network.wifiPassword} onInput={(e) => update('network', 'wifiPassword', (e.target as HTMLInputElement).value)} />
      </div>
      <div class="card">
        <h3>MQTT</h3>
        <Toggle checked={config.mqtt.enabled} onChange={(v) => update('mqtt', 'enabled', v)} label="Enabled" />
        <label>Server</label>
        <input type="text" value={config.mqtt.server} onInput={(e) => update('mqtt', 'server', (e.target as HTMLInputElement).value)} />
        <label>Port</label>
        <input type="number" value={config.mqtt.port} onInput={(e) => update('mqtt', 'port', parseInt((e.target as HTMLInputElement).value))} />
        <label>Topic</label>
        <input type="text" value={config.mqtt.topic} onInput={(e) => update('mqtt', 'topic', (e.target as HTMLInputElement).value)} />
        <Toggle checked={config.mqtt.authEnabled} onChange={(v) => update('mqtt', 'authEnabled', v)} label="Authentication" />
        {config.mqtt.authEnabled && <>
          <label>Username</label>
          <input type="text" value={config.mqtt.username} onInput={(e) => update('mqtt', 'username', (e.target as HTMLInputElement).value)} />
          <label>Password</label>
          <input type="password" value={config.mqtt.password} onInput={(e) => update('mqtt', 'password', (e.target as HTMLInputElement).value)} />
        </>}
      </div>
      <div class="card">
        <h3>Web UI</h3>
        <label>Username</label>
        <input type="text" value={config.web.username} onInput={(e) => update('web', 'username', (e.target as HTMLInputElement).value)} />
        <label>Password</label>
        <input type="password" value={config.web.password} onInput={(e) => update('web', 'password', (e.target as HTMLInputElement).value)} />
      </div>
      <button onClick={handleSave}>{t(lang).saveSettings}</button>

      <div class="card" style="margin-top:16px">
        <h3>Backup & Restore</h3>
        <p style="font-size:0.85em;color:var(--text-muted);margin-bottom:8px">Export or import all settings and schedules as a JSON file.</p>
        <div style="display:flex;gap:8px;flex-wrap:wrap">
          <button onClick={() => downloadBackup()}>Export Backup</button>
          <label class="level-btn" style="cursor:pointer;padding:8px 16px;margin:0">
            Import Backup
            <input type="file" accept=".json" style="display:none" onChange={async (e) => {
              const f = (e.target as HTMLInputElement).files?.[0]
              if (f && await restoreBackup(f)) {
                setMsg({ type: 'success', text: 'Backup restored. Reboot to apply network changes.' })
                getConfig().then(setConfig)
              } else {
                setMsg({ type: 'error', text: 'Failed to restore backup' })
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
  const [msg, setMsg] = useState('')

  const handleUpload = async () => {
    if (!file) return
    setUploading(true)
    setMsg('')
    if (await uploadFirmware(file)) setMsg('Update successful. Rebooting...')
    else setMsg('Update failed')
    setUploading(false)
  }

  const formatUptime = (seconds: number) => {
    const h = Math.floor(seconds / 3600)
    const m = Math.floor((seconds % 3600) / 60)
    return `${h}h ${m}m`
  }

  return (
    <>
      {status && <div class="card">
        <h3>{t(lang).systemInfo}</h3>
        <div class="stat"><span class="label">Version</span><span class="value">{status.system.version}</span></div>
        <div class="stat"><span class="label">Uptime</span><span class="value">{formatUptime(status.system.uptime)}</span></div>
        <div class="stat"><span class="label">Free heap</span><span class="value">{Math.round(status.system.freeHeap / 1024)} KB</span></div>
        <div class="stat"><span class="label">WiFi RSSI</span><span class="value">{status.system.wifiRssi} dBm</span></div>
        <div class="stat"><span class="label">MQTT</span><span class={`value ${status.system.mqttConnected ? 'ok' : 'fault'}`}>{status.system.mqttConnected ? 'Connected' : 'Disconnected'}</span></div>
      </div>}
      <div class="card">
        <h3>{t(lang).firmwareUpdate}</h3>
        <input type="file" accept=".bin" onChange={(e) => setFile((e.target as HTMLInputElement).files?.[0] ?? null)} />
        <br /><br />
        <button onClick={handleUpload} disabled={!file || uploading}>{uploading ? 'Uploading...' : 'Upload Firmware'}</button>
        {msg && <div class="msg success">{msg}</div>}
      </div>
    </>
  )
}

export function App() {
  const [authenticated, setAuthenticated] = useState(false)
  const [tab, setTab] = useState(0)
  const [status, setStatus] = useState<Status | null>(null)
  const [pendingAction, setPendingAction] = useState(false)
  const [lang, setLang] = useState<Lang>('en')

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
    getStatus().then((s) => { setStatus(s); setAuthenticated(true) }).catch(() => {})
    getConfig().then((c) => { if (c.language === 'de' || c.language === 'en') setLang(c.language) }).catch(() => {})
  }, [])

  if (!authenticated) return <LoginPage onLogin={() => setAuthenticated(true)} />

  return (
    <div class="container">
      <h1>Wolf CWL</h1>
      {status?.system.simulated && <div class="sim-banner">Simulation Mode</div>}
      <div class="tabs">
        <div class={`tab ${tab === 0 ? 'active' : ''}`} onClick={() => setTab(0)}>{t(lang).status}</div>
        <div class={`tab ${tab === 1 ? 'active' : ''}`} onClick={() => setTab(1)}>{t(lang).schedules}</div>
        <div class={`tab ${tab === 2 ? 'active' : ''}`} onClick={() => setTab(2)}>{t(lang).settings}</div>
        <div class={`tab ${tab === 3 ? 'active' : ''}`} onClick={() => setTab(3)}>{t(lang).system}</div>
      </div>
      {tab === 0 && <StatusTab status={status} lang={lang} onLevelChange={async (level) => { setPendingAction(true); await setVentilationLevel(level) }} onCancelOff={async () => { await cancelTimedOff(); refreshStatus() }} onConfirmed={() => setPendingAction(false)} />}
      {tab === 1 && <SchedulesTab airflow={status?.airflow} lang={lang} />}
      {tab === 2 && <SettingsTab lang={lang} onLangChange={setLang} />}
      {tab === 3 && <SystemTab status={status} lang={lang} />}

      <OledMirror />

      <div class="encoder-bar">
        <button class="enc-btn" onClick={() => sendEncoderAction('left')}>&#9664; Left</button>
        <button class="enc-btn press" onClick={() => sendEncoderAction('press')}>Press</button>
        <button class="enc-btn" onClick={() => sendEncoderAction('right')}>Right &#9654;</button>
      </div>
    </div>
  )
}
