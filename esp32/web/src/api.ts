export interface Status {
  ventilation: { level: number; levelName: string; relative: number }
  temperature: { supplyInlet: number; exhaustInlet: number }
  status: { fault: boolean; filter: boolean; bypass: boolean; connected: boolean }
  system: { uptime: number; freeHeap: number; version: string; mqttConnected: boolean; wifiRssi: number }
}

export interface Config {
  network: { wifiSsid: string; wifiPassword: string }
  mqtt: { enabled: boolean; server: string; port: number; topic: string; authEnabled: boolean; username: string; password: string }
  web: { username: string; password: string }
  pins: { otIn: number; otOut: number; sda: number; scl: number; encClk: number; encDt: number; encSw: number }
  configured: boolean
}

async function request(url: string, options?: RequestInit) {
  const res = await fetch(url, { ...options, credentials: 'same-origin' })
  if (res.status === 401) throw new Error('unauthorized')
  return res
}

export async function login(username: string, password: string): Promise<boolean> {
  const res = await fetch('/api/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
    credentials: 'same-origin'
  })
  return res.ok
}

export async function getStatus(): Promise<Status> {
  const res = await request('/api/status')
  return res.json()
}

export async function getConfig(): Promise<Config> {
  const res = await request('/api/config')
  return res.json()
}

export async function saveConfig(config: Partial<Config>): Promise<boolean> {
  const res = await request('/api/config', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(config)
  })
  return res.ok
}

export async function setVentilationLevel(level: number): Promise<boolean> {
  const res = await request('/api/ventilation/level', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ level })
  })
  return res.ok
}

export async function uploadFirmware(file: File): Promise<boolean> {
  const formData = new FormData()
  formData.append('firmware', file)
  const res = await request('/api/ota/upload', { method: 'POST', body: formData })
  return res.ok
}

export interface ScheduleEntry {
  enabled: boolean
  startHour: number; startMinute: number
  endHour: number; endMinute: number
  ventLevel: number
  activeDays: number
}

export interface BypassScheduleData {
  enabled: boolean
  startDay: number; startMonth: number
  endDay: number; endMonth: number
}

export async function getSchedules(): Promise<ScheduleEntry[]> {
  const res = await request('/api/schedules')
  return res.json()
}

export async function saveSchedules(schedules: ScheduleEntry[]): Promise<boolean> {
  const res = await request('/api/schedules', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(schedules)
  })
  return res.ok
}

export async function getBypassSchedule(): Promise<BypassScheduleData> {
  const res = await request('/api/bypass-schedule')
  return res.json()
}

export async function saveBypassSchedule(schedule: BypassScheduleData): Promise<boolean> {
  const res = await request('/api/bypass-schedule', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(schedule)
  })
  return res.ok
}

export async function wifiSetup(ssid: string, password: string, mqtt?: { server: string; port: number; topic: string; username?: string; password?: string }): Promise<boolean> {
  const body: Record<string, unknown> = { ssid, password }
  if (mqtt) body.mqtt = mqtt
  const res = await fetch('/api/wifi/setup', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body)
  })
  return res.ok
}
