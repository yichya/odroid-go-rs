pub const DISPLAY_WIDTH: usize = 320;
pub const DISPLAY_HEIGHT: usize = 240;
pub const FB_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;

pub struct FrameBuffer {
    pub pixels: Box<[u16; FB_SIZE]>,
}

impl FrameBuffer {
    pub fn new() -> Self {
        let pixels = vec![0u16; FB_SIZE]
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "vec len mismatch")
            .unwrap();

        Self { pixels }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: u16) {
        if x < DISPLAY_WIDTH && y < DISPLAY_HEIGHT {
            self.pixels[y * DISPLAY_WIDTH + x] = color;
        }
    }

    pub fn fill(&mut self, color: u16) {
        self.pixels.fill(color);
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u16) {
        let x_end = (x + w).min(DISPLAY_WIDTH);
        let y_end = (y + h).min(DISPLAY_HEIGHT);
        for row in y..y_end {
            let start = row * DISPLAY_WIDTH + x;
            let end = row * DISPLAY_WIDTH + x_end;
            self.pixels[start..end].fill(color);
        }
    }

    pub fn as_u8_slice(&self) -> &[u8] {
        // SAFETY: u16 pixels are stored little-endian on both Xtensa and host
        // platforms. The slice is valid for the lifetime of FrameBuffer.
        unsafe { core::slice::from_raw_parts(self.pixels.as_ptr() as *const u8, FB_SIZE * 2) }
    }
}
