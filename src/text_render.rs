use crate::font::{
    FONT_ASCII_DATA, FONT_ASCII_FIRST, FONT_ASC_BYTES, FONT_ASC_H, FONT_ASC_W,
    FONT_CJK_BYTES, FONT_CJK_DATA, FONT_CJK_H, FONT_CJK_W, FONT_GB_MIN, FONT_LOOKUP,
};
use crate::gbuf::{FrameBuffer, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use crate::utf8_gb2312::{count_glyphs, utf8_to_gb2312};

#[allow(clippy::too_many_arguments)]
fn draw_glyph(
    fb: &mut FrameBuffer,
    x: i32,
    y: i32,
    data: &[u8],
    w: i32,
    h: i32,
    offset: usize,
    color: u16,
) {
    let bytes_per_row = (w as usize).div_ceil(8);

    let xstart = if x < 0 { -x } else { 0 };
    let xend = if x + w > DISPLAY_WIDTH as i32 {
        (DISPLAY_WIDTH as i32 - x).max(0)
    } else {
        w
    };
    let ystart = if y < 0 { -y } else { 0 };
    let yend = if y + h > DISPLAY_HEIGHT as i32 {
        (DISPLAY_HEIGHT as i32 - y).max(0)
    } else {
        h
    };

    for row in ystart..yend {
        let py = (y + row) as usize;
        for col in xstart..xend {
            let byte_idx = offset + row as usize * bytes_per_row + col as usize / 8;
            if let Some(&byte) = data.get(byte_idx) {
                if byte & (1 << (col as usize % 8)) != 0 {
                    fb.set_pixel((x + col) as usize, py, color);
                }
            }
        }
    }
}

fn draw_ascii(fb: &mut FrameBuffer, x: i32, y: i32, code: u8, color: u16) {
    if !(0x20..=0x7E).contains(&code) {
        return;
    }
    let offset = (code as usize - FONT_ASCII_FIRST as usize) * FONT_ASC_BYTES;
    draw_glyph(
        fb,
        x,
        y,
        FONT_ASCII_DATA,
        FONT_ASC_W as i32,
        FONT_ASC_H as i32,
        offset,
        color,
    );
}

fn draw_cjk(fb: &mut FrameBuffer, x: i32, y: i32, gb_code: u16, color: u16) {
    let idx = (gb_code as usize - FONT_GB_MIN as usize) * 2;
    if idx + 1 >= FONT_LOOKUP.len() {
        return;
    }
    let font_idx = u16::from_le_bytes([FONT_LOOKUP[idx], FONT_LOOKUP[idx + 1]]);
    if font_idx == 0xFFFF {
        return;
    }
    let offset = font_idx as usize * FONT_CJK_BYTES;
    draw_glyph(
        fb,
        x,
        y,
        FONT_CJK_DATA,
        FONT_CJK_W as i32,
        FONT_CJK_H as i32,
        offset,
        color,
    );
}

pub fn draw_gb2312_str(fb: &mut FrameBuffer, codes: &[u16], x: i32, y: i32, color: u16) {
    let mut xoff = x;
    let asc_w = FONT_ASC_W as i32;
    let cjk_w = FONT_CJK_W as i32;
    for &code in codes.iter().take_while(|&&c| c != 0) {
        if code < 0x80 {
            draw_ascii(fb, xoff, y, code as u8, color);
            xoff += asc_w;
        } else {
            draw_cjk(fb, xoff, y, code, color);
            xoff += cjk_w;
        }
    }
}

pub fn draw_str(fb: &mut FrameBuffer, s: &str, x: i32, y: i32, color: u16) {
    let n = count_glyphs(s);
    let mut buf: std::vec::Vec<u16> = vec![0; n + 1];
    utf8_to_gb2312(s, &mut buf);
    draw_gb2312_str(fb, &buf, x, y, color);
}

pub fn draw_text(fb: &mut FrameBuffer, x: i32, y: i32, text: &str, color: u16) {
    draw_str(fb, text, x, y, color);
}

pub fn truncate_to_width(s: &str, max_px: i32) -> String {
    let mut result = String::new();
    let mut w = 0i32;
    for c in s.chars() {
        let cw = if c.is_ascii() {
            FONT_ASC_W as i32
        } else {
            FONT_CJK_W as i32
        };
        if w + cw > max_px {
            break;
        }
        result.push(c);
        w += cw;
    }
    result
}

pub fn wrap_to_width(text: &str, max_px: i32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        let mut line_w = 0i32;
        for c in paragraph.chars() {
            let cw = if c.is_ascii() {
                FONT_ASC_W as i32
            } else {
                FONT_CJK_W as i32
            };
            if line_w + cw > max_px {
                lines.push(current);
                current = String::new();
                line_w = 0;
            }
            current.push(c);
            line_w += cw;
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
