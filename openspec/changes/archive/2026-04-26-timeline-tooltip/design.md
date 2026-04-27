## Context

The `WeekTimeline` component renders segments as absolutely positioned divs with `title` attributes. The `DailyVolume` component already has the airflow rates. The tooltip needs airflow data to compute volume per segment.

## Goals / Non-Goals

**Goals:**
- Custom CSS tooltip positioned near the hovered segment
- Content: "08:00 - 22:00 · Normal · 14h · 1,820 m³"
- For gaps: "03:00 - 08:00 · No program · 5h"
- Styled to match the dark theme

**Non-Goals:**
- Touch/mobile tooltip (hover is desktop-only; mobile users see the table below)

## Decisions

### 1. Implementation: Preact state + positioned div

Use `useState` to track the hovered segment info and mouse position. On `mouseEnter` of a segment div, set the tooltip data. On `mouseLeave`, clear it. Render a fixed-position tooltip div.

### 2. Volume calculation

Volume = (segment duration in hours) × airflow rate for that level. Rates come from the `airflow` prop (same as `DailyVolume`).

### 3. Pass airflow to WeekTimeline

Add `airflow` prop to `WeekTimeline` so it can compute per-segment volume.
