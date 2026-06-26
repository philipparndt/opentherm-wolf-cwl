import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, fireEvent, waitFor } from '@testing-library/preact'
import { TimelineChart } from './TimelineChart'
import * as api from './api'
import type { History } from './api'

vi.mock('./api', () => ({ getHistory: vi.fn() }))

function makeHistory(): History {
  return {
    slots: 5,
    bucketMs: 600_000,
    nowEpoch: 1_700_000_000,
    supply: [null, [18.0, 19.0], [18.2, 18.6], [18.4, 18.8], [18.5, 18.5]],
    exhaust: [[20.0, 21.0], [20.5, 21.0], [20.8, 21.2], [21.0, 21.0], [21.0, 21.0]],
    humiditySlots: 5,
    humidityBucketMs: 600_000,
    indoorHumidity: [[40, 42], [41, 43], [42, 44], [43, 45], [44, 44]],
    outdoorHumidity: [[60, 62], [61, 63], [62, 64], [63, 65], [66, 66]],
    events: [{ epoch: 1_699_998_800, level: 2, reason: 'cooling' }],
    bypass: [{ epoch: 1_699_999_000, open: true }],
  }
}

function withEnthalpy(h: History): History {
  return {
    ...h,
    indoorEnthalpy: [[44, 45], [45, 46], [46, 47], [47, 48], [48, 48]],
    outdoorEnthalpy: [[30, 31], [31, 32], [32, 33], [33, 34], [35, 35]],
  }
}

async function renderChart(history: History) {
  vi.mocked(api.getHistory).mockResolvedValue(history)
  const result = render(<TimelineChart lang="en" />)
  await waitFor(() => expect(result.container.querySelector('svg')).not.toBeNull())
  return result
}

describe('TimelineChart current-point markers', () => {
  beforeEach(() => vi.clearAllMocks())

  it('draws a highlighted current-point marker for each populated series', async () => {
    const { container } = await renderChart(makeHistory())
    // Visible current markers carry the contrasting ring (stroke=--bg-secondary):
    // supply, exhaust, indoor RH, outdoor RH = 4.
    const markers = container.querySelectorAll('circle[stroke="var(--bg-secondary)"]')
    expect(markers.length).toBe(4)
  })

  it('renders nothing chart-like before data and the empty message', () => {
    vi.mocked(api.getHistory).mockReturnValue(new Promise(() => {})) // never resolves
    const { container, getByText } = render(<TimelineChart lang="en" />)
    expect(container.querySelector('svg')).toBeNull()
    expect(getByText('Collecting data…')).toBeTruthy()
  })
})

describe('TimelineChart hover tooltips', () => {
  beforeEach(() => vi.clearAllMocks())

  it('shows the latest values when hovering a current point', async () => {
    const { container } = await renderChart(makeHistory())
    const hit = container.querySelector('circle.chart-hit')!
    fireEvent.mouseEnter(hit, { clientX: 200, clientY: 120 })

    const tip = container.querySelector('.chart-tooltip')!
    expect(tip).toBeTruthy()
    expect(tip.textContent).toContain('Supply: 18.5°')
    expect(tip.textContent).toContain('Exhaust: 21.0°')
    expect(tip.textContent).toContain('Indoor RH: 44.0%')
    expect(tip.textContent).toContain('Outdoor RH: 66.0%')
    expect(tip.textContent).toContain('Now')
  })

  it('dismisses the tooltip when the pointer leaves', async () => {
    const { container } = await renderChart(makeHistory())
    const hit = container.querySelector('circle.chart-hit')!
    fireEvent.mouseEnter(hit, { clientX: 200, clientY: 120 })
    expect(container.querySelector('.chart-tooltip')).toBeTruthy()
    fireEvent.mouseLeave(hit)
    expect(container.querySelector('.chart-tooltip')).toBeNull()
  })

  it('explains a level-change marker on hover, with curve values at its time', async () => {
    const { container } = await renderChart(makeHistory())
    // The level-change marker hit area is the 12px-wide transparent rect.
    const hit = Array.from(container.querySelectorAll('rect.chart-hit')).find((r) => r.getAttribute('width') === '12')!
    expect(hit).toBeTruthy()
    fireEvent.mouseEnter(hit, { clientX: 150, clientY: 100 })
    const tip = container.querySelector('.chart-tooltip')!
    expect(tip.textContent).toContain('Normal')
    expect(tip.textContent).toContain('Cooling assist')
    // The reading at the marker's time is shown too (curve values).
    expect(tip.textContent).toMatch(/Supply: \d/)
    expect(tip.textContent).toMatch(/Exhaust: \d/)
  })

  it('colours each tooltip line with its series swatch', async () => {
    const { container } = await renderChart(makeHistory())
    const hit = container.querySelector('circle.chart-hit')!
    fireEvent.mouseEnter(hit, { clientX: 200, clientY: 120 })
    const styles = Array.from(container.querySelectorAll('.chart-tooltip span')).map((s) => s.getAttribute('style') ?? '')
    const swatches = styles.filter((s) => s.includes('border-radius'))
    // One swatch per value line; supply's swatch uses the supply accent colour.
    expect(swatches.length).toBeGreaterThan(0)
    expect(swatches.some((c) => c.includes('var(--accent-orange)'))).toBe(true)
  })

  it('explains a bypass span on hover', async () => {
    const { container } = await renderChart(makeHistory())
    // The bypass hit rects sit in the strip lane (y = BYPASS_Y - 4 = 200); the
    // most recent span (last rect) is the open one in this fixture.
    const spans = Array.from(container.querySelectorAll('rect.chart-hit')).filter((r) => r.getAttribute('y') === '200')
    const hit = spans[spans.length - 1]
    expect(hit).toBeTruthy()
    fireEvent.mouseEnter(hit, { clientX: 300, clientY: 210 })
    const tip = container.querySelector('.chart-tooltip')!
    expect(tip.textContent).toContain('Open (free cooling)')
  })

  it('localizes tooltip labels (German)', async () => {
    vi.mocked(api.getHistory).mockResolvedValue(makeHistory())
    const { container } = render(<TimelineChart lang="de" />)
    await waitFor(() => expect(container.querySelector('svg')).not.toBeNull())
    const spans = Array.from(container.querySelectorAll('rect.chart-hit')).filter((r) => r.getAttribute('y') === '200')
    fireEvent.mouseEnter(spans[spans.length - 1], { clientX: 300, clientY: 210 })
    expect(container.querySelector('.chart-tooltip')!.textContent).toContain('Offen (Kühlung)')
  })
})

describe('TimelineChart enthalpy + toggles', () => {
  beforeEach(() => { vi.clearAllMocks(); localStorage.clear() })

  it('draws enthalpy curves with their own current markers and value lines', async () => {
    const { container } = await renderChart(withEnthalpy(makeHistory()))
    // supply, exhaust, indoor RH, outdoor RH, indoor h, outdoor h = 6 markers.
    const markers = container.querySelectorAll('circle[stroke="var(--bg-secondary)"]')
    expect(markers.length).toBe(6)
    const hit = container.querySelector('circle.chart-hit')!
    fireEvent.mouseEnter(hit, { clientX: 200, clientY: 120 })
    const tip = container.querySelector('.chart-tooltip')!
    expect(tip.textContent).toContain('Indoor h: 48.0 kJ/kg')
    expect(tip.textContent).toContain('Outdoor h: 35.0 kJ/kg')
  })

  it('hides a series when its legend entry is toggled off and persists the choice', async () => {
    const { container, getByText, unmount } = await renderChart(withEnthalpy(makeHistory()))
    expect(container.querySelectorAll('circle[stroke="var(--bg-secondary)"]').length).toBe(6)

    fireEvent.click(getByText('Supply').closest('.legend-toggle')!)
    // Supply curve + its current marker are gone → 5 markers.
    expect(container.querySelectorAll('circle[stroke="var(--bg-secondary)"]').length).toBe(5)
    expect(JSON.parse(localStorage.getItem('wolf-cwl.timeline-visibility')!).supply).toBe(false)

    // Remount reads the persisted selection: supply stays hidden.
    unmount()
    const again = await renderChart(withEnthalpy(makeHistory()))
    expect(again.container.querySelectorAll('circle[stroke="var(--bg-secondary)"]').length).toBe(5)
  })

  it('defaults an unknown stored series to visible', async () => {
    // Storage that only pins one series off; everything else must default on.
    localStorage.setItem('wolf-cwl.timeline-visibility', JSON.stringify({ exhaust: false }))
    const { container } = await renderChart(withEnthalpy(makeHistory()))
    // exhaust hidden → 5 of the 6 markers remain.
    expect(container.querySelectorAll('circle[stroke="var(--bg-secondary)"]').length).toBe(5)
  })
})

describe('TimelineChart scrubbing crosshair', () => {
  beforeEach(() => { vi.clearAllMocks(); localStorage.clear() })

  // The full-plot scrubbing overlay is the chart-hit rect at y = PAD_T = 26.
  // jsdom has no layout, so give it a concrete rect to map clientX against.
  function plotOverlay(container: Element) {
    const rect = Array.from(container.querySelectorAll('rect.chart-hit')).find((r) => r.getAttribute('y') === '26')! as SVGRectElement
    rect.getBoundingClientRect = () => ({ left: 0, top: 0, width: 600, height: 170, right: 600, bottom: 170, x: 0, y: 0, toJSON: () => {} }) as DOMRect
    return rect
  }

  it('shows every visible curve value at the hovered time and draws a crosshair', async () => {
    const { container } = await renderChart(withEnthalpy(makeHistory()))
    const overlay = plotOverlay(container)
    // Far right (clientX = width) → the latest bucket of each series.
    fireEvent.mouseMove(overlay, { clientX: 600, clientY: 100 })

    const tip = container.querySelector('.chart-tooltip')!
    expect(tip.textContent).toContain('Supply: 18.5°')
    expect(tip.textContent).toContain('Exhaust: 21.0°')
    expect(tip.textContent).toContain('Indoor RH: 44.0%')
    expect(tip.textContent).toContain('Outdoor h: 35.0 kJ/kg')

    // The vertical crosshair line (text-secondary) plus a dot per visible curve.
    const crosshair = container.querySelector('line[stroke="var(--text-secondary)"]')
    expect(crosshair).not.toBeNull()
  })

  it('reads an earlier bucket when scrubbing left (skips empty buckets)', async () => {
    const { container } = await renderChart(makeHistory())
    const overlay = plotOverlay(container)
    // Far left → bucket 0: supply[0] is null (skipped), exhaust[0] = 20.5.
    fireEvent.mouseMove(overlay, { clientX: 0, clientY: 100 })
    const tip = container.querySelector('.chart-tooltip')!
    expect(tip.textContent).toContain('Exhaust: 20.5°')
    expect(tip.textContent).not.toContain('Supply:')
  })

  it('omits hidden curves from the crosshair tooltip', async () => {
    localStorage.setItem('wolf-cwl.timeline-visibility', JSON.stringify({ exhaust: false }))
    const { container } = await renderChart(makeHistory())
    const overlay = plotOverlay(container)
    fireEvent.mouseMove(overlay, { clientX: 600, clientY: 100 })
    const tip = container.querySelector('.chart-tooltip')!
    expect(tip.textContent).toContain('Supply: 18.5°')
    expect(tip.textContent).not.toContain('Exhaust:')
  })

  it('clears the crosshair and tooltip when the pointer leaves the plot', async () => {
    const { container } = await renderChart(makeHistory())
    const overlay = plotOverlay(container)
    fireEvent.mouseMove(overlay, { clientX: 300, clientY: 100 })
    expect(container.querySelector('.chart-tooltip')).not.toBeNull()
    fireEvent.mouseLeave(overlay)
    expect(container.querySelector('.chart-tooltip')).toBeNull()
    expect(container.querySelector('line[stroke="var(--text-secondary)"]')).toBeNull()
  })
})
