export type Lang = 'en' | 'de'

export interface Translations {
  status: string; schedules: string; settings: string; system: string
  levels: readonly string[]; days: readonly string[]
  login: string; username: string; password: string; invalidCredentials: string
  ventilationSchedules: string; weeklyOverview: string; weeklyOverviewHint: string
  noProgram: string; schedule: string; level: string; time: string; from: string; to: string
  addSchedule: string; saveAllSchedules: string; schedulesSaved: string; failedToSave: string
  split: string; delete: string; edit: string; cancel: string; save: string
  weekdays: string; weekend: string; daily: string
  summerMode: string; bypassFrom: string; bypassTo: string
  wifi: string; ssid: string; mqtt: string; server: string; port: string; topic: string
  authEnabled: string; webUi: string; saveSettings: string; language: string
  english: string; german: string
  backupRestore: string; exportBackup: string; importBackup: string; backupRestored: string
  systemInfo: string; firmwareUpdate: string; uploadFirmware: string; uploading: string; updateSuccessful: string
  ventilation: string; override: string; timedOff: string; cancelOff: string; resumesIn: string
  current: string; temperatures: string
  supplyInlet: string; exhaustInlet: string; supplyOutlet: string; exhaustOutlet: string
  summer: string; winter: string; bypassMode: string
}

const en: Translations = {
  // Tabs
  status: 'Status',
  schedules: 'Schedules',
  settings: 'Settings',
  system: 'System',

  // Levels
  levels: ['Off', 'Reduced', 'Normal', 'Party'] as readonly string[],
  days: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] as readonly string[],

  // Login
  login: 'Login',
  username: 'Username',
  password: 'Password',
  invalidCredentials: 'Invalid credentials',

  // Schedule
  ventilationSchedules: 'Ventilation Schedules',
  weeklyOverview: 'Weekly Overview',
  weeklyOverviewHint: '— click gaps to add, right-click segments for options, drag borders',
  noProgram: 'No program',
  schedule: 'Schedule',
  level: 'Level',
  time: 'Time',
  from: 'From',
  to: 'to',
  addSchedule: '+ Add Schedule',
  saveAllSchedules: 'Save All Schedules',
  schedulesSaved: 'Schedules saved',
  failedToSave: 'Failed to save',
  split: 'Split',
  delete: 'Delete',
  edit: 'Edit',
  cancel: 'Cancel',
  save: 'Save',
  weekdays: 'Weekdays',
  weekend: 'Weekend',
  daily: 'Daily',

  // Summer mode
  summerMode: 'Summer Mode (Cooling)',
  bypassFrom: 'From',
  bypassTo: 'to',

  // Settings
  wifi: 'WiFi',
  ssid: 'SSID',
  mqtt: 'MQTT',
  server: 'Server',
  port: 'Port',
  topic: 'Topic',
  authEnabled: 'Authentication',
  webUi: 'Web UI',
  saveSettings: 'Save Settings',
  language: 'Language',
  english: 'English',
  german: 'Deutsch',

  // Backup
  backupRestore: 'Backup & Restore',
  exportBackup: 'Export Backup',
  importBackup: 'Import Backup',
  backupRestored: 'Backup restored. Reboot to apply network changes.',

  // System
  systemInfo: 'System Info',
  firmwareUpdate: 'Firmware Update',
  uploadFirmware: 'Upload Firmware',
  uploading: 'Uploading...',
  updateSuccessful: 'Update successful. Rebooting...',

  // Status tab
  ventilation: 'Ventilation',
  override: 'Override',
  timedOff: 'Timed Off',
  cancelOff: 'Cancel Off',
  resumesIn: 'Resumes in',
  current: 'Current',
  temperatures: 'Temperatures',
  supplyInlet: 'Supply inlet',
  exhaustInlet: 'Exhaust inlet',
  supplyOutlet: 'Supply outlet',
  exhaustOutlet: 'Exhaust outlet',
  summer: 'Summer',
  winter: 'Winter',
  bypassMode: 'Bypass',
}

const de: Translations = {
  status: 'Status',
  schedules: 'Zeitpläne',
  settings: 'Einstellungen',
  system: 'System',

  levels: ['Aus', 'Reduziert', 'Normal', 'Party'],
  days: ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'],

  login: 'Anmelden',
  username: 'Benutzer',
  password: 'Passwort',
  invalidCredentials: 'Ungültige Anmeldedaten',

  ventilationSchedules: 'Lüftungs-Zeitpläne',
  weeklyOverview: 'Wochenübersicht',
  weeklyOverviewHint: '— Lücken klicken zum Hinzufügen, Rechtsklick für Optionen, Ränder ziehen',
  noProgram: 'Kein Programm',
  schedule: 'Zeitplan',
  level: 'Stufe',
  time: 'Zeit',
  from: 'Von',
  to: 'bis',
  addSchedule: '+ Zeitplan hinzufügen',
  saveAllSchedules: 'Alle Zeitpläne speichern',
  schedulesSaved: 'Zeitpläne gespeichert',
  failedToSave: 'Speichern fehlgeschlagen',
  split: 'Teilen',
  delete: 'Löschen',
  edit: 'Bearbeiten',
  cancel: 'Abbrechen',
  save: 'Speichern',
  weekdays: 'Werktage',
  weekend: 'Wochenende',
  daily: 'Täglich',

  summerMode: 'Sommermodus (Kühlung)',
  bypassFrom: 'Von',
  bypassTo: 'bis',

  wifi: 'WiFi',
  ssid: 'SSID',
  mqtt: 'MQTT',
  server: 'Server',
  port: 'Port',
  topic: 'Topic',
  authEnabled: 'Authentifizierung',
  webUi: 'Web UI',
  saveSettings: 'Einstellungen speichern',
  language: 'Sprache',
  english: 'English',
  german: 'Deutsch',

  backupRestore: 'Sicherung & Wiederherstellung',
  exportBackup: 'Backup exportieren',
  importBackup: 'Backup importieren',
  backupRestored: 'Backup wiederhergestellt. Neustart für Netzwerkänderungen.',

  systemInfo: 'Systeminformation',
  firmwareUpdate: 'Firmware-Update',
  uploadFirmware: 'Firmware hochladen',
  uploading: 'Hochladen...',
  updateSuccessful: 'Update erfolgreich. Neustart...',

  ventilation: 'Lüftung',
  override: 'Manuell',
  timedOff: 'Zeitweise aus',
  cancelOff: 'Aus beenden',
  resumesIn: 'Weiter in',
  current: 'Aktuell',
  temperatures: 'Temperaturen',
  supplyInlet: 'Zuluft Eingang',
  exhaustInlet: 'Abluft Eingang',
  supplyOutlet: 'Zuluft Ausgang',
  exhaustOutlet: 'Abluft Ausgang',
  summer: 'Sommer',
  winter: 'Winter',
  bypassMode: 'Bypass',
}

const translations: Record<Lang, Translations> = { en, de }

export function t(lang: Lang): Translations {
  return translations[lang] ?? translations.en
}
