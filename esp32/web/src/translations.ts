export type Lang = 'en' | 'de'

export interface Translations {
  status: string; schedules: string; settings: string; system: string; debug: string
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
  requireLogin: string; noLoginWarning: string
  english: string; german: string
  backupRestore: string; exportBackup: string; importBackup: string; backupRestored: string
  systemInfo: string; firmwareUpdate: string; uploadFirmware: string; uploading: string; updateSuccessful: string
  ventilation: string; override: string; timedOff: string; cancelOff: string; resumesIn: string
  current: string; temperatures: string
  supply: string; exhaust: string
  summer: string; winter: string; bypassMode: string
  extremeHeatMode: string; extremeHeatHint: string; extremeHeatOverridesSchedules: string
  extremeHeatActive: string; extremeHeatForcedTo: string; extremeHeatMatchedRule: string
  ehRules: readonly string[]
  tempHistory: string; levelChanges: string; noHistoryYet: string
  legendMeasurements: string; legendReason: string
  legendBypass: string; bypassOpen: string; bypassClosed: string
  now: string
  // Humidity-aware ventilation
  climateDecision: string; activeRule: string
  humiditySensors: string; humiditySensorsHint: string
  moistureProtection: string; outdoorSensorTopics: string; outdoorSensorTopicsHint: string; addOutdoorSensor: string; indoorSensorTopics: string; addIndoorSensor: string
  indoorRh: string; outdoorRh: string; ambientPressure: string
  indoorEnthalpy: string; outdoorEnthalpy: string
  indoorAir: string; outdoorAir: string; aggregateHint: string
  holdDeadband: string; holdDwell: string
  protectionActiveMsg: string; tempOnlyFallback: string; waitingForUnit: string
  reasonLabels: { temp: string; cooling: string; dehumidify: string; muggy: string; manual: string; schedule: string; reboot: string }
  reasonText: { temp: string; cooling: string; dehumidify: string; muggy: string; manual: string; schedule: string; reboot: string }
}

const en: Translations = {
  // Tabs
  status: 'Status',
  schedules: 'Schedules',
  settings: 'Settings',
  system: 'System',
  debug: 'Debug',

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
  requireLogin: 'Require login',
  noLoginWarning: 'Login is disabled — anyone who can reach this device on the network can read and change all settings.',
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
  supply: 'Supply',
  exhaust: 'Exhaust',
  summer: 'Summer',
  winter: 'Winter',
  bypassMode: 'Bypass',
  extremeHeatMode: 'Extreme Heat Mode',
  extremeHeatHint: 'Automatically lowers ventilation when incoming air is hotter than indoor air, and boosts it when incoming air is cooler. Decisions hold for 15 min.',
  extremeHeatOverridesSchedules: 'Extreme heat mode is active and controls the ventilation level and bypass automatically. The schedules below are overridden and have no effect until it is turned off.',
  extremeHeatActive: 'Extreme heat mode active',
  extremeHeatForcedTo: 'Ventilation forced to',
  extremeHeatMatchedRule: 'Matched rule',
  ehRules: [
    'Incoming air much warmer than indoor (Δ > +0.5 °C)',
    'Incoming air slightly warmer than indoor (0 to +0.5 °C)',
    'Incoming air cooler than indoor (−1.0 to 0 °C)',
    'Incoming air much cooler than indoor (Δ < −1.0 °C)',
  ],
  tempHistory: 'Temperature History (24h)',
  levelChanges: 'Level changes',
  noHistoryYet: 'Collecting data…',
  legendMeasurements: 'Measurements',
  legendReason: 'Change reason',
  legendBypass: 'Bypass',
  bypassOpen: 'Open (free cooling)',
  bypassClosed: 'Closed (heat recovery)',
  now: 'Now',
  climateDecision: 'Climate Decision',
  activeRule: 'Active rule',
  humiditySensors: 'Humidity Sensors',
  humiditySensorsHint: 'MQTT topics publishing JSON with humidity / temperature / pressure. Used by extreme-heat mode (energy-based cooling) and moisture protection.',
  moistureProtection: 'Moisture protection (year-round)',
  outdoorSensorTopics: 'Outdoor sensor topics',
  outdoorSensorTopicsHint: 'Add several — the decision uses the lowest temperature and highest humidity across them.',
  addOutdoorSensor: '+ Add outdoor sensor',
  indoorSensorTopics: 'Indoor sensor topics',
  addIndoorSensor: '+ Add indoor sensor',
  indoorRh: 'Indoor RH',
  outdoorRh: 'Outdoor RH',
  ambientPressure: 'Ambient pressure',
  indoorEnthalpy: 'Indoor h',
  outdoorEnthalpy: 'Outdoor h',
  indoorAir: 'Indoor (T / RH / AH / h)',
  outdoorAir: 'Outdoor (T / RH / AH / h)',
  aggregateHint: 'Per side: lowest temperature and highest RH; AH/enthalpy are from the wettest sensor re-expressed at that temperature — so these figures can come from different sensors and need not reconcile as one reading.',
  holdDeadband: 'Holding — outdoor and indoor energy are within the neutral band (Δh {dh} kJ/kg); not switching on sensor noise.',
  holdDwell: 'Would switch to {level} in {time} (dwell).',
  protectionActiveMsg: 'Moisture protection is actively ventilating.',
  tempOnlyFallback: 'Using temperature only — humidity data is missing or stale.',
  waitingForUnit: 'Waiting for the ventilation unit — climate decision starts once it is connected.',
  reasonLabels: { temp: 'Temperature', cooling: 'Cooling assist', dehumidify: 'Moisture protection', muggy: 'Muggy suppression', manual: 'Manual', schedule: 'Schedule', reboot: 'Reboot' },
  reasonText: {
    temp: 'Deciding on the supply vs exhaust temperature difference.',
    cooling: 'Outdoor air carries less energy — ventilating to cool.',
    dehumidify: 'Indoor air is too humid and outside air is drier — ventilating to dehumidify.',
    muggy: 'Outdoor air is warmer or more humid (higher energy) — holding ventilation down.',
    manual: 'Ventilation level was set manually.',
    schedule: 'Following the configured ventilation schedule.',
    reboot: 'Device just (re)started — settling on a decision.',
  },
}

const de: Translations = {
  status: 'Status',
  schedules: 'Zeitpläne',
  settings: 'Einstellungen',
  system: 'System',
  debug: 'Debug',

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
  requireLogin: 'Anmeldung erforderlich',
  noLoginWarning: 'Die Anmeldung ist deaktiviert — jeder, der dieses Gerät im Netzwerk erreicht, kann alle Einstellungen lesen und ändern.',
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
  supply: 'Zuluft',
  exhaust: 'Abluft',
  summer: 'Sommer',
  winter: 'Winter',
  bypassMode: 'Bypass',
  extremeHeatMode: 'Extremhitze-Modus',
  extremeHeatHint: 'Senkt die Lüftung automatisch, wenn die Zuluft wärmer als die Raumluft ist, und erhöht sie, wenn die Zuluft kühler ist. Entscheidungen gelten 15 Min.',
  extremeHeatOverridesSchedules: 'Der Extremhitze-Modus ist aktiv und steuert die Lüftungsstufe und den Bypass automatisch. Die Zeitpläne unten werden überschrieben und haben keine Wirkung, bis er ausgeschaltet wird.',
  extremeHeatActive: 'Extremhitze-Modus aktiv',
  extremeHeatForcedTo: 'Lüftung erzwungen auf',
  extremeHeatMatchedRule: 'Zutreffende Regel',
  ehRules: [
    'Zuluft viel wärmer als innen (Δ > +0,5 °C)',
    'Zuluft etwas wärmer als innen (0 bis +0,5 °C)',
    'Zuluft kühler als innen (−1,0 bis 0 °C)',
    'Zuluft viel kühler als innen (Δ < −1,0 °C)',
  ],
  tempHistory: 'Temperaturverlauf (24h)',
  levelChanges: 'Stufenwechsel',
  noHistoryYet: 'Sammle Daten…',
  legendMeasurements: 'Messwerte',
  legendReason: 'Auslöser',
  legendBypass: 'Bypass',
  bypassOpen: 'Offen (Kühlung)',
  bypassClosed: 'Geschlossen (Wärmerückgewinnung)',
  now: 'Jetzt',
  climateDecision: 'Klima-Entscheidung',
  activeRule: 'Aktive Regel',
  humiditySensors: 'Feuchtesensoren',
  humiditySensorsHint: 'MQTT-Topics mit JSON-Feldern humidity / temperature / pressure. Genutzt von Extremhitze-Modus (energiebasierte Kühlung) und Feuchteschutz.',
  moistureProtection: 'Feuchteschutz (ganzjährig)',
  outdoorSensorTopics: 'Außensensor-Topics',
  outdoorSensorTopicsHint: 'Mehrere möglich — die Entscheidung nutzt die niedrigste Temperatur und höchste Feuchte daraus.',
  addOutdoorSensor: '+ Außensensor hinzufügen',
  indoorSensorTopics: 'Innensensor-Topics',
  addIndoorSensor: '+ Innensensor hinzufügen',
  indoorRh: 'Innen rel. F.',
  outdoorRh: 'Außen rel. F.',
  indoorEnthalpy: 'Innen h',
  outdoorEnthalpy: 'Außen h',
  ambientPressure: 'Luftdruck',
  indoorAir: 'Innen (T / rF / AF / h)',
  outdoorAir: 'Außen (T / rF / AF / h)',
  aggregateHint: 'Je Seite: niedrigste Temperatur und höchste rF; AF/Enthalpie stammen vom feuchtesten Sensor, umgerechnet auf diese Temperatur — die Werte können also von verschiedenen Sensoren kommen und müssen sich nicht zu einem Zustand zusammenfügen.',
  holdDeadband: 'Wird gehalten — Außen- und Innenenergie liegen im neutralen Band (Δh {dh} kJ/kg); kein Umschalten auf Sensorrauschen.',
  holdDwell: 'Würde in {time} auf {level} wechseln (Dwell).',
  protectionActiveMsg: 'Feuchteschutz lüftet aktiv.',
  tempOnlyFallback: 'Nur Temperatur — Feuchtedaten fehlen oder sind veraltet.',
  waitingForUnit: 'Warte auf die Lüftungsanlage — die Klima-Entscheidung startet, sobald sie verbunden ist.',
  reasonLabels: { temp: 'Temperatur', cooling: 'Kühlung', dehumidify: 'Feuchteschutz', muggy: 'Schwül-Stopp', manual: 'Manuell', schedule: 'Zeitplan', reboot: 'Neustart' },
  reasonText: {
    temp: 'Entscheidung nach der Temperaturdifferenz Zuluft/Abluft.',
    cooling: 'Außenluft hat weniger Energie — Lüften zum Kühlen.',
    dehumidify: 'Innenluft ist zu feucht und Außenluft ist trockener — Lüften zum Entfeuchten.',
    muggy: 'Außenluft ist wärmer oder feuchter (mehr Energie) — Lüftung wird gedrosselt.',
    manual: 'Lüftungsstufe wurde manuell gesetzt.',
    schedule: 'Folgt dem eingestellten Lüftungs-Zeitplan.',
    reboot: 'Gerät wurde gerade (neu) gestartet — Entscheidung wird ermittelt.',
  },
}

const translations: Record<Lang, Translations> = { en, de }

export function t(lang: Lang): Translations {
  return translations[lang] ?? translations.en
}
