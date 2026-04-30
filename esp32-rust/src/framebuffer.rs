//! In-memory 128x64 monochrome framebuffer for mirroring OLED content to web UI.

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 64;
pub const BUF_SIZE: usize = WIDTH * HEIGHT / 8; // 1024 bytes

/// A 128x64 monochrome framebuffer implementing embedded-graphics DrawTarget.
/// Pixel layout: byte = 8 vertical pixels (column-major, matching SSD1306 page mode).
pub struct FrameBuffer {
    pub buf: [u8; BUF_SIZE],
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self { buf: [0; BUF_SIZE] }
    }

    pub fn clear(&mut self) {
        self.buf.fill(0);
    }

    /// Set a pixel. Layout: 8 pages of 128 columns, each byte is 8 vertical pixels.
    fn set_pixel(&mut self, x: usize, y: usize, on: bool) {
        if x >= WIDTH || y >= HEIGHT { return; }
        let page = y / 8;
        let bit = y % 8;
        let idx = page * WIDTH + x;
        if on {
            self.buf[idx] |= 1 << bit;
        } else {
            self.buf[idx] &= !(1 << bit);
        }
    }
}

impl DrawTarget for FrameBuffer {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x >= 0 && point.y >= 0 {
                self.set_pixel(point.x as usize, point.y as usize, color == BinaryColor::On);
            }
        }
        Ok(())
    }
}

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}
