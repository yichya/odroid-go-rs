# odroid-go-rs

Shared hardware abstraction library for [Odroid-GO](https://wiki.odroid.com/odroid_go/odroid_go) firmware projects, extracted from [ogo-shell](https://github.com/Paspartout/ogo-shell).

## Modules

| Module | Purpose |
|--------|---------|
| `backlight` | LCD backlight PWM via LEDC (GPIO14, 5kHz, 13-bit resolution) |
| `display` | ILI9341 SPI display driver (HSPI, 40MHz, DMA) |
| `font` | 12px bitmap font (ASCII 6×13, CJK 12×13) |
| `gbuf` | 320×240 frame buffer (u16 RGB565 pixels) |
| `keypad` | Button / D-pad / battery ADC driver with debounce |
| `sdcard` | SD card mount via SPI (shares HSPI with LCD, different CS) |
| `text_render` | Text rendering (ASCII + CJK, word wrap, truncation) |
| `utf8_gb2312` | UTF-8 to GB2312 code point conversion |

## Pin Assignments

| Function | GPIO |
|----------|------|
| LCD MOSI | 18 |
| LCD MISO | 19 |
| LCD CLK | 23 |
| LCD CS | 5 |
| LCD DC | 21 |
| SD Card CS | 22 |
| Backlight PWM | 14 |
| Button A | 32 |
| Button B | 33 |
| Select | 27 |
| Start | 39 |
| Menu | 13 |
| Volume | 0 |
| D-Pad X (ADC) | 34 |
| D-Pad Y (ADC) | 35 |
| Battery (ADC) | 36 |

## Build

Requires the [Rust ESP toolchain](https://github.com/esp-rs/rust-build) and ESP-IDF v5.2.7.

```bash
source sdk/export-esp.sh
cargo build --release
```

The build script generates CJK and ASCII font data at compile time from a TrueType font file.

## Tools

| Tool | Purpose |
|------|---------|
| `mkfw.py` | Firmware .fw container packer (drop-in replacement for the C `mkfw` binary) |
| `build_font.py` | Font bitmap generator (called by `build.rs`, writes generated data to `OUT_DIR`) |

### mkfw.py

```
python3 tools/mkfw.py <description> <tile> <type> <subtype> <length> <label> <binary> \
                       [<type> <subtype> <length> <label> <binary> ...]
```

Packs one or more firmware partitions into an `firmware.fw` container for the Odroid-GO bootloader.

### Font

The font is built entirely from a TrueType font file at compile time. No pre-generated bitmaps are included in the repository.

**Recommended font**: [SimSun](https://learn.microsoft.com/en-us/typography/font-list/simsun) (NSimSun, face index 1). Due to copyright considerations, this font is **not** included in the project. You must obtain the font file yourself and handle any copyright or licensing matters.

Place `simsun.ttc` in the project root (or configure `font_file_path` in `extras.json` to point to your font file).

### Adding Extra Characters

Edit `tools/extras.json`:

```json
{
  "font_file_path": "../simsun.ttc",
  "gb18030": ["龘", "新字"],
  "mapped": {"•": "·"}
}
```

- `gb18030` — GB18030 characters to render and append to the CJK glyph set
- `mapped` — Unicode remapping (source → destination, destination must already be in CODE_TABLE)

A rebuild (`cargo build`) will regenerate the font data automatically.

## License

GPL-3.0
