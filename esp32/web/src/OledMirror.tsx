import { useEffect, useRef, useState } from 'preact/hooks'
import { getDisplay } from './api'

const SCALE = 3
const WIDTH = 128
const HEIGHT = 64
const BORDER = 2 // border in OLED pixels
const CANVAS_W = (WIDTH + BORDER * 2) * SCALE
const CANVAS_H = (HEIGHT + BORDER * 2) * SCALE
const OFF_X = BORDER * SCALE
const OFF_Y = BORDER * SCALE

// Background color (dark, matches OLED bezel)
const BG_R = 5, BG_G = 6, BG_B = 10

export function OledMirror() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    let active = true

    const poll = async () => {
      while (active) {
        try {
          const frame = await getDisplay()
          if (!active) break

          const canvas = canvasRef.current
          if (!canvas) break

          const ctx = canvas.getContext('2d')
          if (!ctx) break

          // Decode base64 framebuffer
          const raw = atob(frame.data)
          const bytes = new Uint8Array(raw.length)
          for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i)

          // Create image with border
          const img = ctx.createImageData(CANVAS_W, CANVAS_H)

          // Fill entire image with bezel color
          for (let i = 0; i < img.data.length; i += 4) {
            img.data[i] = BG_R
            img.data[i + 1] = BG_G
            img.data[i + 2] = BG_B
            img.data[i + 3] = 255
          }

          // Render OLED pixels offset by border
          for (let page = 0; page < 8; page++) {
            for (let col = 0; col < 128; col++) {
              const byte = bytes[page * 128 + col]
              for (let bit = 0; bit < 8; bit++) {
                const on = (byte >> bit) & 1
                for (let sy = 0; sy < SCALE; sy++) {
                  for (let sx = 0; sx < SCALE; sx++) {
                    const px = OFF_X + col * SCALE + sx
                    const py = OFF_Y + (page * 8 + bit) * SCALE + sy
                    const idx = (py * CANVAS_W + px) * 4
                    if (on) {
                      img.data[idx] = 120
                      img.data[idx + 1] = 210
                      img.data[idx + 2] = 255
                    } else {
                      img.data[idx] = 10
                      img.data[idx + 1] = 12
                      img.data[idx + 2] = 18
                    }
                  }
                }
              }
            }
          }
          ctx.putImageData(img, 0, 0)
          setError(false)
        } catch {
          setError(true)
        }

        await new Promise(r => setTimeout(r, 200))
      }
    }

    poll()
    return () => { active = false }
  }, [])

  return (
    <div class="card" style="padding:12px;text-align:center">
      <h3>OLED Display</h3>
      <canvas
        ref={canvasRef}
        width={CANVAS_W}
        height={CANVAS_H}
        style="border-radius:6px;image-rendering:pixelated;max-width:100%;height:auto"
      />
      {error && <div style="color:var(--text-muted);font-size:0.8em;margin-top:4px">Display not available</div>}
    </div>
  )
}
