use crate::font::{CODE_TABLE, CODE_TABLE_LEN, UNICODE_REMAP};

fn search_remap(unicode_key: u16) -> u16 {
    for &(src, tgt) in UNICODE_REMAP {
        if src == unicode_key {
            return tgt;
        }
    }
    unicode_key
}

fn search_code_table(unicode_key: u16) -> u16 {
    let key = search_remap(unicode_key);
    let mut first: isize = 0;
    let mut end: isize = CODE_TABLE_LEN as isize - 1;
    while first <= end {
        let mid = (first + end) / 2;
        let entry = CODE_TABLE[mid as usize];
        if entry.0 == key {
            return entry.1;
        } else if entry.0 > key {
            end = mid - 1;
        } else {
            first = mid + 1;
        }
    }
    0xA1F5
}

fn utf8_byte_len(first_byte: u8) -> usize {
    if first_byte < 0xC0 {
        0
    } else if first_byte < 0xE0 {
        2
    } else if first_byte < 0xF0 {
        3
    } else if first_byte < 0xF8 {
        4
    } else if first_byte < 0xFC {
        5
    } else {
        6
    }
}

pub fn count_glyphs(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut count = 0;
    while i < bytes.len() {
        let len = utf8_byte_len(bytes[i]);
        if len == 0 {
            i += 1;
        } else {
            i += len;
        }
        count += 1;
    }
    count
}

pub fn utf8_to_gb2312(s: &str, buf: &mut [u16]) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut k = 0;
    let mut byte_count;
    let mut temp = [0u8; 2];

    while i < bytes.len() {
        let v = utf8_byte_len(bytes[i]);
        match v {
            0 => {
                buf[k] = bytes[i] as u16;
                k += 1;
                byte_count = 1;
            }
            2 => {
                let unicode_key = ((bytes[i] as u16 & 0x1F) << 6) | (bytes[i + 1] as u16 & 0x3F);
                let gb_key = search_code_table(unicode_key);
                buf[k] = gb_key;
                k += 1;
                byte_count = 2;
            }
            3 => {
                temp[1] = ((bytes[i] & 0x0F) << 4) | ((bytes[i + 1] >> 2) & 0x0F);
                temp[0] = ((bytes[i + 1] & 0x03) << 6) | (bytes[i + 2] & 0x3F);
                let unicode_key = u16::from_le_bytes(temp);
                let gb_key = search_code_table(unicode_key);
                buf[k] = gb_key;
                k += 1;
                byte_count = 3;
            }
            4 => {
                buf[k] = b'?' as u16;
                k += 1;
                byte_count = 4;
            }
            5 => {
                buf[k] = b'?' as u16;
                k += 1;
                byte_count = 5;
            }
            6 => {
                buf[k] = b'?' as u16;
                k += 1;
                byte_count = 6;
            }
            _ => {
                buf[k] = b'?' as u16;
                k += 1;
                byte_count = 1;
            }
        }
        i += byte_count;
    }
    if k < buf.len() {
        buf[k] = 0;
    }
    k
}
