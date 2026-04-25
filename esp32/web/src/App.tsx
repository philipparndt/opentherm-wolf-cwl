import { useState, useEffect, useCallback } from 'preact/hooks'
import { login, getStatus, getConfig, saveConfig, setVentilationLevel, uploadFirmware,
         getSchedules, saveSchedules, getBypassSchedule, saveBypassSchedule,
         type Status, type Config, type ScheduleEntry, type BypassScheduleData } from './api'

const LEVELS = ['Off', 'Reduced', 'Normal', 'Party'] as const
const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] as const
const MONTHS = ['', 'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'] as const

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

function StatusTab({ status, onLevelChange }: { status: Status | null; onLevelChange: (level: number) => void }) {
  if (!status) return <div>Loading...</div>

  return (
    <>
      <div class="card">
        <h3>Ventilation</h3>
        <div class="level-buttons">
          {LEVELS.map((name, i) => (
            <div class={`level-btn ${status.ventilation.level === i ? 'active' : ''}`}
                 onClick={() => onLevelChange(i)}>{name}</div>
          ))}
        </div>
        <div class="stat"><span class="label">Relative</span><span class="value">{status.ventilation.relative}%</span></div>
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

function SchedulesTab() {
  const [entries, setEntries] = useState<ScheduleEntry[]>([])
  const [bypass, setBypass] = useState<BypassScheduleData>({ enabled: false, startDay: 15, startMonth: 4, endDay: 30, endMonth: 9 })
  const [msg, setMsg] = useState<{ type: string; text: string } | null>(null)

  useEffect(() => {
    getSchedules().then(setEntries)
    getBypassSchedule().then(setBypass)
  }, [])

  const updateEntry = (idx: number, field: string, value: number | boolean) => {
    const updated = [...entries]
    updated[idx] = { ...updated[idx], [field]: value }
    setEntries(updated)
  }

  const toggleDay = (idx: number, dayBit: number) => {
    const updated = [...entries]
    updated[idx] = { ...updated[idx], activeDays: updated[idx].activeDays ^ (1 << dayBit) }
    setEntries(updated)
  }

  const addEntry = () => {
    if (entries.length >= 8) return
    setEntries([...entries, { enabled: true, startHour: 8, startMinute: 0, endHour: 22, endMinute: 0, ventLevel: 2, activeDays: 0x7F }])
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

  const pad = (n: number) => n.toString().padStart(2, '0')

  return (
    <>
      {msg && <div class={`msg ${msg.type}`}>{msg.text}</div>}

      <h2>Ventilation Schedules</h2>
      {entries.map((entry, idx) => (
        <div class="card" key={idx}>
          <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">
            <div class="toggle" style="margin:0">
              <input type="checkbox" checked={entry.enabled} onChange={(e) => updateEntry(idx, 'enabled', (e.target as HTMLInputElement).checked)} />
              <label>Enabled</label>
            </div>
            <button class="danger" style="padding:4px 10px;font-size:0.8em" onClick={() => removeEntry(idx)}>Delete</button>
          </div>
          <div style="display:flex;gap:8px;align-items:center;margin-bottom:8px">
            <input type="time" value={`${pad(entry.startHour)}:${pad(entry.startMinute)}`}
              onInput={(e) => { const [h, m] = (e.target as HTMLInputElement).value.split(':').map(Number); updateEntry(idx, 'startHour', h); updateEntry(idx, 'startMinute', m) }} />
            <span>to</span>
            <input type="time" value={`${pad(entry.endHour)}:${pad(entry.endMinute)}`}
              onInput={(e) => { const [h, m] = (e.target as HTMLInputElement).value.split(':').map(Number); updateEntry(idx, 'endHour', h); updateEntry(idx, 'endMinute', m) }} />
          </div>
          <div class="level-buttons" style="margin-bottom:8px">
            {LEVELS.map((name, i) => (
              <div class={`level-btn ${entry.ventLevel === i ? 'active' : ''}`}
                   onClick={() => updateEntry(idx, 'ventLevel', i)}>{name}</div>
            ))}
          </div>
          <div style="display:flex;gap:4px;flex-wrap:wrap">
            {DAYS.map((day, i) => (
              <div class={`level-btn ${entry.activeDays & (1 << i) ? 'active' : ''}`}
                   style="flex:none;padding:6px 8px;font-size:0.8em"
                   onClick={() => toggleDay(idx, i)}>{day}</div>
            ))}
          </div>
        </div>
      ))}
      {entries.length < 8 && <button onClick={addEntry} style="margin-bottom:16px">+ Add Schedule</button>}

      <h2>Summer Mode (Cooling)</h2>
      <div class="card">
        <p style="font-size:0.85em;color:#888;margin-bottom:8px">Free cooling — outdoor air bypasses heat exchanger. Enable during warm months.</p>
        <div class="toggle"><input type="checkbox" checked={bypass.enabled} onChange={(e) => setBypass({ ...bypass, enabled: (e.target as HTMLInputElement).checked })} /><label>Enable summer mode schedule</label></div>
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

      <button onClick={handleSave}>Save All Schedules</button>
    </>
  )
}

function SettingsTab() {
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
        <h3>WiFi</h3>
        <label>SSID</label>
        <input type="text" value={config.network.wifiSsid} onInput={(e) => update('network', 'wifiSsid', (e.target as HTMLInputElement).value)} />
        <label>Password</label>
        <input type="password" value={config.network.wifiPassword} onInput={(e) => update('network', 'wifiPassword', (e.target as HTMLInputElement).value)} />
      </div>
      <div class="card">
        <h3>MQTT</h3>
        <div class="toggle"><input type="checkbox" checked={config.mqtt.enabled} onChange={(e) => update('mqtt', 'enabled', (e.target as HTMLInputElement).checked)} /><label>Enabled</label></div>
        <label>Server</label>
        <input type="text" value={config.mqtt.server} onInput={(e) => update('mqtt', 'server', (e.target as HTMLInputElement).value)} />
        <label>Port</label>
        <input type="number" value={config.mqtt.port} onInput={(e) => update('mqtt', 'port', parseInt((e.target as HTMLInputElement).value))} />
        <label>Topic</label>
        <input type="text" value={config.mqtt.topic} onInput={(e) => update('mqtt', 'topic', (e.target as HTMLInputElement).value)} />
        <div class="toggle"><input type="checkbox" checked={config.mqtt.authEnabled} onChange={(e) => update('mqtt', 'authEnabled', (e.target as HTMLInputElement).checked)} /><label>Authentication</label></div>
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
      <button onClick={handleSave}>Save Settings</button>
    </>
  )
}

function SystemTab({ status }: { status: Status | null }) {
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
        <h3>System Info</h3>
        <div class="stat"><span class="label">Version</span><span class="value">{status.system.version}</span></div>
        <div class="stat"><span class="label">Uptime</span><span class="value">{formatUptime(status.system.uptime)}</span></div>
        <div class="stat"><span class="label">Free heap</span><span class="value">{Math.round(status.system.freeHeap / 1024)} KB</span></div>
        <div class="stat"><span class="label">WiFi RSSI</span><span class="value">{status.system.wifiRssi} dBm</span></div>
        <div class="stat"><span class="label">MQTT</span><span class={`value ${status.system.mqttConnected ? 'ok' : 'fault'}`}>{status.system.mqttConnected ? 'Connected' : 'Disconnected'}</span></div>
      </div>}
      <div class="card">
        <h3>Firmware Update</h3>
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
    const interval = setInterval(refreshStatus, 5000)
    return () => clearInterval(interval)
  }, [authenticated, refreshStatus])

  useEffect(() => {
    getStatus().then((s) => { setStatus(s); setAuthenticated(true) }).catch(() => {})
  }, [])

  if (!authenticated) return <LoginPage onLogin={() => setAuthenticated(true)} />

  return (
    <div class="container">
      <h1>Wolf CWL</h1>
      <div class="tabs">
        <div class={`tab ${tab === 0 ? 'active' : ''}`} onClick={() => setTab(0)}>Status</div>
        <div class={`tab ${tab === 1 ? 'active' : ''}`} onClick={() => setTab(1)}>Schedules</div>
        <div class={`tab ${tab === 2 ? 'active' : ''}`} onClick={() => setTab(2)}>Settings</div>
        <div class={`tab ${tab === 3 ? 'active' : ''}`} onClick={() => setTab(3)}>System</div>
      </div>
      {tab === 0 && <StatusTab status={status} onLevelChange={async (level) => { await setVentilationLevel(level); refreshStatus() }} />}
      {tab === 1 && <SchedulesTab />}
      {tab === 2 && <SettingsTab />}
      {tab === 3 && <SystemTab status={status} />}
    </div>
  )
}
