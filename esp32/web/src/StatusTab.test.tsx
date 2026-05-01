import { describe, it, expect, vi } from 'vitest'
import { render, fireEvent } from '@testing-library/preact'
import { StatusTab } from './StatusTab'
import type { Status } from './api'

function makeStatus(level: number, requestedLevel?: number): Status {
  return {
    ventilation: { level, levelName: ['Off', 'Reduced', 'Normal', 'Party'][level], relative: [0, 51, 67, 100][level], requestedLevel: requestedLevel ?? level, scheduleActive: true, override: false },
    temperature: { supplyInlet: 18.5, exhaustInlet: 21.0 },
    status: { fault: false, filter: false, bypass: false, connected: true },
    system: { uptime: 100, freeHeap: 200000, version: 'test', mqttConnected: true, wifiRssi: -50, simulated: false },
    timedOff: { active: false, remainingMinutes: 0 },
    airflow: { reduced: 100, normal: 130, party: 195 },
  }
}

describe('StatusTab level buttons', () => {
  it('shows the confirmed level as active with no spinner', () => {
    const { container } = render(
      <StatusTab lang="en" status={makeStatus(1)} onLevelChange={() => {}} onCancelOff={() => {}} />
    )
    const buttons = container.querySelectorAll('.level-btn')

    expect(buttons[1].classList.contains('active')).toBe(true)
    expect(buttons[1].querySelector('.spinner')).toBeNull()

    // Others are not active
    expect(buttons[0].classList.contains('active')).toBe(false)
    expect(buttons[2].classList.contains('active')).toBe(false)
    expect(buttons[3].classList.contains('active')).toBe(false)
  })

  it('immediately selects clicked button with spinner and deselects old button', () => {
    const onLevelChange = vi.fn()
    const { container } = render(
      <StatusTab lang="en" status={makeStatus(1)} onLevelChange={onLevelChange} onCancelOff={() => {}} />
    )

    // Click Party (index 3)
    fireEvent.click(container.querySelectorAll('.level-btn')[3])

    expect(onLevelChange).toHaveBeenCalledWith(3)

    const buttons = container.querySelectorAll('.level-btn')

    // Reduced (1) should no longer be active
    expect(buttons[1].classList.contains('active')).toBe(false)

    // Party (3) should be selected (active) with spinner
    expect(buttons[3].classList.contains('active')).toBe(true)
    expect(buttons[3].classList.contains('pending')).toBe(true)
    expect(buttons[3].querySelector('.spinner')).not.toBeNull()
  })

  it('keeps spinner while CWL has not yet confirmed', () => {
    const { container, rerender } = render(
      <StatusTab lang="en" status={makeStatus(1)} onLevelChange={() => {}} onCancelOff={() => {}} />
    )

    // Click Party
    fireEvent.click(container.querySelectorAll('.level-btn')[3])

    // Status polls but CWL still reports level 1
    rerender(
      <StatusTab lang="en" status={makeStatus(1, 3)} onLevelChange={() => {}} onCancelOff={() => {}} />
    )

    const buttons = container.querySelectorAll('.level-btn')

    // Reduced should not be active (user clicked away)
    expect(buttons[1].classList.contains('active')).toBe(false)

    // Party still shows as selected with spinner
    expect(buttons[3].classList.contains('active')).toBe(true)
    expect(buttons[3].classList.contains('pending')).toBe(true)
    expect(buttons[3].querySelector('.spinner')).not.toBeNull()
  })

  it('removes spinner when CWL confirms the new level', () => {
    const onConfirmed = vi.fn()
    const { container, rerender } = render(
      <StatusTab lang="en" status={makeStatus(1)} onLevelChange={() => {}} onCancelOff={() => {}} onConfirmed={onConfirmed} />
    )

    // Click Party
    fireEvent.click(container.querySelectorAll('.level-btn')[3])

    // CWL confirms level 3
    rerender(
      <StatusTab lang="en" status={makeStatus(3)} onLevelChange={() => {}} onCancelOff={() => {}} onConfirmed={onConfirmed} />
    )

    const buttons = container.querySelectorAll('.level-btn')

    // Party is active with NO spinner
    expect(buttons[3].classList.contains('active')).toBe(true)
    expect(buttons[3].classList.contains('pending')).toBe(false)
    expect(buttons[3].querySelector('.spinner')).toBeNull()

    // Reduced is not active
    expect(buttons[1].classList.contains('active')).toBe(false)

    expect(onConfirmed).toHaveBeenCalled()
  })

  it('does not show spinner when clicking the already-confirmed level', () => {
    const { container } = render(
      <StatusTab lang="en" status={makeStatus(2)} onLevelChange={() => {}} onCancelOff={() => {}} />
    )

    // Click Normal (already active)
    fireEvent.click(container.querySelectorAll('.level-btn')[2])

    const buttons = container.querySelectorAll('.level-btn')
    expect(buttons[2].classList.contains('active')).toBe(true)
    expect(buttons[2].classList.contains('pending')).toBe(false)
    expect(buttons[2].querySelector('.spinner')).toBeNull()
  })

  it('handles rapid clicks - last click wins', () => {
    const onLevelChange = vi.fn()
    const { container } = render(
      <StatusTab lang="en" status={makeStatus(1)} onLevelChange={onLevelChange} onCancelOff={() => {}} />
    )

    const buttons = container.querySelectorAll('.level-btn')

    // Click Party then Normal quickly
    fireEvent.click(buttons[3])
    fireEvent.click(buttons[2])

    // Normal should be the pending one
    expect(buttons[2].classList.contains('active')).toBe(true)
    expect(buttons[2].classList.contains('pending')).toBe(true)
    expect(buttons[2].querySelector('.spinner')).not.toBeNull()

    // Party should not be selected
    expect(buttons[3].classList.contains('active')).toBe(false)
  })
})
