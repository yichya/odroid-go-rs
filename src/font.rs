pub const FONT_CJK_W: u16 = 12;
pub const FONT_CJK_H: u16 = 13;
pub const FONT_ASC_W: u16 = 6;
pub const FONT_ASC_H: u16 = 13;
pub const FONT_CJK_BYTES: usize = 26;
pub const FONT_ASC_BYTES: usize = 13;
pub const FONT_GB_MIN: u16 = 0x8140;
pub const FONT_ASCII_FIRST: u16 = 0x20;
pub const FONT_ASCII_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/font_ascii.bin"));
pub const FONT_CJK_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/font_cjk.bin"));
pub const FONT_LOOKUP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/font_lookup.bin"));
pub const CODE_TABLE_LEN: usize = CODE_TABLE.len();

include!(concat!(env!("OUT_DIR"), "/unicode_gb2312.rs"));
