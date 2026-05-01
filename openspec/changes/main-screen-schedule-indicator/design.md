## Context

The home screen on the 128x64 OLED display currently shows:
- Header: "Ventilation" with underline (y=0..11)
- Large centered text: ventilation level name (e.g. "Medium") at y=18
- Small centered info line: "65% M  Winter" or "65% S  Winter" at y=42

The single-letter "M" (manual) or "S" (schedule) indicator is easy to miss. The user wants the home screen to look visually distinct depending on which mode is active so it's immediately obvious at a glance.

Display is BinaryColor (on/off pixels only), 128x64 px, driven by `embedded_graphics` with U8g2 BDF fonts.

## Goals / Non-Goals

**Goals:**
- Make it immediately obvious whether the current ventilation level is schedule-driven or manually set
- Use the header area to communicate mode — change the header text and/or style per mode
- Keep all existing information visible (level name, percentage, bypass mode)

**Non-Goals:**
- Changing the edit mode screens (encoder interaction)
- Changing the timed-off countdown screen
- Adding new pages or navigation
- Changing MQTT/protocol behavior

## Decisions

### 1. Differentiate via header text and inverted header bar

**Choice**: Change the header to show the mode context:
- Schedule active: header shows "Schedule" with an inverted (white-on-black) filled rectangle behind it
- Manual override: header shows "Manual" in normal style (black-on-white, current rendering)
- Neither (default/no schedule): header shows "Ventilation" as today

**Rationale**: The header is the first thing the eye sees. An inverted bar is the strongest visual signal available on a 1-bit display — it's used in many embedded UIs to indicate "active" state. This requires no new fonts or graphics, just a filled rectangle behind the text.

**Alternatives considered**:
- Border/frame around entire screen: uses too many pixels on a 128x64 display, reduces content area
- Blinking text: not supported well with the current draw loop timing
- Different font size per mode: would disrupt layout consistency

### 2. Remove the inline "M"/"S" indicator from the info line

**Choice**: Since the header now clearly communicates the mode, remove the " M" / " S" suffix from the small info line. The info line becomes simply "65%  Winter".

**Rationale**: Reduces clutter. The mode is now prominently shown in the header, so duplicating it in the info line adds noise.

### 3. Implementation approach — modify `draw_home()` only

The change is isolated to the `else` branch of `draw_home()` in `display.rs`. Replace the single `draw_header("Ventilation")` call with mode-aware logic:
- If `st.schedule_active && !st.schedule_override` → `draw_header_inverted(d, "Schedule")`
- If `st.schedule_override` → `draw_header(d, "Manual")`
- Otherwise → `draw_header(d, "Ventilation")`

Add a new helper `draw_header_inverted()` that draws a filled white rectangle (0,0 to 127,11) then renders the title text in black (BinaryColor::Off on the filled area).

## Risks / Trade-offs

- **[Readability on some panels]** Inverted text can be harder to read on low-contrast SH1106 displays → Mitigation: the header is short text ("Schedule") in a proven small font; test on hardware.
- **[User expectation]** Users familiar with the old "S"/"M" indicator might briefly be confused → Mitigation: the new labels ("Schedule", "Manual") are self-explanatory and more informative than single letters.
