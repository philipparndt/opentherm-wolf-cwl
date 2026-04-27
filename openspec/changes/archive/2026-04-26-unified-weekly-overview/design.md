## Context

Currently: `.container { max-width: 600px }`. The timeline has `tl-label` (36px) + `tl-bar` (flex:1). At 600px with padding, the bar is ~540px — workable but tight. The `DailyVolume` is a separate table card below.

## Decisions

### 1. Layout: volume column right of the timeline bar

```
         0    6    12   18   24
Mon  ████████░░░░████████████░░░░   2,340 m³
Tue  ████████░░░░████████████░░░░   2,340 m³
...
```

Each `tl-row` gets a volume label on the right, right-aligned, muted color. Warning days get orange text.

### 2. Wider container

Change `.container` max-width from 600px to 900px. This gives the timeline ~200px more horizontal space, making segments easier to distinguish. Other tabs (Status, Settings, System) also benefit from a bit more room.

### 3. Remove DailyVolume component

The separate table becomes redundant. The detailed breakdown is available via the hover tooltip on each segment.
