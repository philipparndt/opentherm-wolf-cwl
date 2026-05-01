## ADDED Requirements

### Requirement: Time label shown during schedule drag
The WeekTimeline SHALL display a floating time label showing the current snapped time (HH:MM) while the user is dragging a schedule segment handle.

#### Scenario: Dragging start handle shows time
- **WHEN** the user drags a segment's left (start) handle
- **THEN** a label SHALL appear near the cursor showing the snapped start time in HH:MM format (e.g. "14:30")

#### Scenario: Dragging end handle shows time
- **WHEN** the user drags a segment's right (end) handle
- **THEN** a label SHALL appear near the cursor showing the snapped end time in HH:MM format

#### Scenario: Time label updates during drag
- **WHEN** the user moves the mouse while dragging
- **THEN** the time label SHALL update in real-time to reflect the new snapped time position

#### Scenario: Time label disappears on release
- **WHEN** the user releases the mouse button (drag ends)
- **THEN** the time label SHALL disappear immediately

### Requirement: Drag time label positioning
The drag time label SHALL be positioned near the cursor without obscuring the drag handle, using fixed positioning with pointer-events disabled.

#### Scenario: Label does not interfere with drag
- **WHEN** the time label is shown during drag
- **THEN** it SHALL have `pointer-events: none` so it does not capture mouse events
