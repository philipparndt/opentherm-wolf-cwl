## Context

The Wolf CWL bypass damper is controlled via bit 1 in the HI byte of the Status V/H (Data ID 70) request. When open, outdoor air bypasses the heat exchanger — effectively **free cooling** in summer. When closed, air goes through the heat exchanger for heat recovery in winter. The firmware currently has `requestedBypassOpen` (bool) toggled by MQTT or encoder. The CWL also has its own bypass logic based on TSP registers (min outdoor temp for bypass, min indoor temp), but the master-controlled bit gives explicit override.

The bypass schedule is simpler than the ventilation schedule: it's a single date range (start day/month to end day/month) defining the "summer season." During this period, the bypass is open. Outside this period, it's closed.

## Goals / Non-Goals

**Goals:**
- Single date-range bypass schedule: start date (DD.MM) to end date (DD.MM) — e.g., 15.04 to 30.09
- Clearly labeled as "Summer Mode (Cooling)" in all UI surfaces
- Help text explaining: "When active, outdoor air bypasses the heat exchanger for free cooling. Enable during warm months."
- Evaluated once per minute (or on date change)
- Manual override via MQTT/encoder, cleared when the season transitions
- Persist in NVS
- MQTT and OLED integration

**Non-Goals:**
- Temperature-based automatic bypass (the CWL has its own logic for that via TSP 6/7)
- Multiple bypass date ranges
- Hourly bypass scheduling (that's what the CWL's built-in bypass logic is for)

## Decisions

### 1. Data model

```cpp
struct BypassSchedule {
    bool enabled;        // Schedule active
    uint8_t startDay;    // 1-31
    uint8_t startMonth;  // 1-12
    uint8_t endDay;      // 1-31
    uint8_t endMonth;    // 1-12
};
```

Stored in `AppConfig` alongside the existing bypass state. NVS keys: `bp_sched_en`, `bp_sched_sd`, `bp_sched_sm`, `bp_sched_ed`, `bp_sched_em`.

### 2. Evaluation logic

Compare current date (from NTP/localtime) against the date range:
- If startMonth < endMonth (e.g., Apr-Sep): match if current month is in [start, end] range, with day boundary checks for start/end months
- If startMonth > endMonth (e.g., Oct-Mar, winter heating): match if current month is >= start OR <= end
- If NTP not synced: do nothing, use manual bypass state

### 3. Manual override

Same pattern as ventilation schedule: if user explicitly sets bypass via MQTT/encoder/web, set `bypassManualOverride = true`. Cleared when the season transitions (date crosses into/out of the scheduled range).

### 4. UI labeling

Throughout the web UI and OLED:
- **"Summer Mode"** as the primary label
- Subtitle/tooltip: "Free cooling — outdoor air bypasses heat exchanger"
- Toggle shows: "Active (Apr 15 - Sep 30)" or "Inactive (Winter)"
- Avoid raw "bypass" terminology in user-facing text; keep "bypass" in MQTT topics and code for technical consistency

### 5. MQTT topics

```
wolf-cwl/bypass/mode           → "summer" or "winter"
wolf-cwl/bypass/schedule_active → "1" or "0" (is the schedule controlling bypass)
wolf-cwl/bypass/override       → "1" or "0" (manual override active)
```

### 6. OLED display

On the home page, the bypass status line changes from "Bypass: Open/Closed" to:
- "Summer Mode" when bypass is open (scheduled or manual)
- "Winter Mode" when bypass is closed

## Risks / Trade-offs

- **[CWL internal bypass logic]** → The CWL has its own bypass decision based on TSP registers (min outdoor temp, min indoor temp). The master bypass bit in ID 70 is an override. If the master says "open" but outdoor temp is below the CWL's threshold, the CWL may still refuse. This is acceptable — the schedule expresses intent, the CWL has the final safety check.
- **[Single date range]** → Sufficient for typical use. Users in unusual climates can adjust or use manual override.
