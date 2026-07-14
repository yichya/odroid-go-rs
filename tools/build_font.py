#!/usr/bin/env python3
"""Build font entirely from a TrueType font at build time.

Renders ASCII, GB2312 base, and GB18030 extras from the font file every time.
base_code_table.txt defines the Unicode→GB2312 mapping (standard characters).
extras.json defines additional characters and remaps.

Requires Pillow and a TrueType font (via font_file_path in extras.json)."""

import json, os, re, struct, sys

PROJ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TOOLS_DIR = os.path.join(PROJ, "tools")

GB_MIN = 0x8140

ASC_W = 6
ASC_H = 13
ASC_BYTES = 13
CJK_W = 12
CJK_H = 13
CJK_BYTES = 26

EXTRAS_PATH = os.path.join(TOOLS_DIR, "extras.json")
BASE_CT = os.path.join(TOOLS_DIR, "base_code_table.txt")

FONT_FILE_PATH = None


def read_code_table_txt(path):
    entries = {}
    with open(path) as f:
        for m in re.finditer(r"0x([0-9a-fA-F]{4}),0x([0-9a-fA-F]{4})", f.read()):
            entries[int(m.group(1), 16)] = int(m.group(2), 16)
    return entries


def encode_gb18030(ch):
    try:
        b = ch.encode("gb18030")
        if len(b) == 2:
            return (b[0] << 8) | b[1]
    except (UnicodeEncodeError, ValueError):
        pass
    return None


def render_glyph(ch, w, h, bytes_per):
    from PIL import Image, ImageFont, ImageDraw
    CANVAS = 32
    PAD_X, PAD_Y = 4, 0
    PX = 12

    font = ImageFont.truetype(FONT_FILE_PATH, PX, index=1)
    img = Image.new("1", (CANVAS, CANVAS), 1)
    draw = ImageDraw.Draw(img)
    draw.text((PAD_X, PAD_Y), ch, font=font, fill=0)

    l, r = CANVAS, 0
    for y in range(CANVAS):
        for x in range(CANVAS):
            if img.getpixel((x, y)) == 0:
                l = min(l, x); r = max(r, x)
    gw = r - l + 1 if r >= l else 0

    if gw > w:
        gw = w
    h_off = (w - gw) // 2 if gw < w else 0

    bw = (w + 7) // 8
    out = bytearray()
    for row in range(h):
        for cb in range(bw):
            v = 0
            for bit in range(8):
                x = cb * 8 + bit
                canvas_x = l + x - h_off
                if canvas_x >= 0 and canvas_x < CANVAS and row < CANVAS and img.getpixel((canvas_x, row)) == 0:
                    v |= 1 << bit
            out.append(v)
    return bytes(out)


def gen_unicode_gb2312_rs(full_entries, remap_entries, out_path):
    lines = ["// Auto-generated. Do not edit.", ""]
    lines.append("pub(crate) const UNICODE_REMAP: &[(u16, u16)] = &[")
    for (src, tgt) in remap_entries:
        lines.append(f"    (0x{src:04x}, 0x{tgt:04x}),")
    lines.append("];")
    lines.append("")

    sorted_ct = sorted(full_entries.items())
    lines.append("#[allow(clippy::large_const_arrays)]")
    lines.append(f"pub(crate) const CODE_TABLE: [(u16, u16); {len(sorted_ct)}] = [")
    for uni, gb in sorted_ct:
        lines.append(f"    (0x{uni:04x}, 0x{gb:04x}),")
    lines.append("];")
    lines.append("")

    with open(out_path, "w") as f:
        f.write("\n".join(lines))
    print(f"Generated: {os.path.basename(out_path)} ({len(sorted_ct)} entries, {len(remap_entries)} remaps)")


def build(out_dir):
    global FONT_FILE_PATH
    assert out_dir

    try:
        from PIL import Image, ImageFont, ImageDraw
    except ImportError:
        sys.exit("ERROR: Pillow not installed.")

    with open(EXTRAS_PATH) as f:
        extras = json.load(f)
    gb18030_list = extras.get("gb18030", [])
    mapped_dict = extras.get("mapped", {})
    font_file_path = extras.get("font_file_path")
    if not font_file_path:
        sys.exit("ERROR: font_file_path is required in extras.json")
    FONT_FILE_PATH = font_file_path if os.path.isabs(font_file_path) else os.path.join(PROJ, font_file_path)
    if not os.path.exists(FONT_FILE_PATH):
        sys.exit(f"ERROR: font_file_path not found: {FONT_FILE_PATH}")

    full_entries = read_code_table_txt(BASE_CT)
    print(f"Base code table: {len(full_entries)} entries")

    cjk = bytearray()
    lookup = bytearray()
    new_glyphs = 0
    added_ct = 0

    # ── Render GB2312 base characters ──
    for uni, gb in full_entries.items():
        idx = (gb - GB_MIN) * 2
        while len(lookup) <= idx + 1:
            lookup.extend(struct.pack("<H", 0xFFFF))
        fi = len(cjk) // CJK_BYTES
        glyph = render_glyph(chr(uni), CJK_W, CJK_H, CJK_BYTES)
        cjk.extend(glyph)
        struct.pack_into("<H", lookup, idx, fi)
        new_glyphs += 1

    print(f"Rendered base: {new_glyphs} CJK glyphs, lookup {len(lookup)} bytes")

    # ── GB18030 extras ──
    added_ct = 0
    remap_entries = []
    for item in gb18030_list:
        ch = str(item)
        if len(ch) != 1:
            print(f"  WARN '{ch}': not a single char, skipping")
            continue
        gb = encode_gb18030(ch)
        if gb is None:
            print(f"  WARN {ch}: not 2-byte encodable in GB18030, skipping")
            continue

        uni = ord(ch)
        idx = (gb - GB_MIN) * 2
        while len(lookup) <= idx + 1:
            lookup.extend(struct.pack("<H", 0xFFFF))

        existing_fi = struct.unpack_from("<H", lookup, idx)[0]
        if existing_fi != 0xFFFF:
            full_entries[uni] = gb
            added_ct += 1
            print(f"  OK  {ch} U+{uni:04X} → 0x{gb:04X} fi={existing_fi}")
        else:
            glyph = render_glyph(ch, CJK_W, CJK_H, CJK_BYTES)
            new_fi = len(cjk) // CJK_BYTES
            cjk.extend(glyph)
            struct.pack_into("<H", lookup, idx, new_fi)
            full_entries[uni] = gb
            new_glyphs += 1
            added_ct += 1
            print(f"  +   {ch} U+{uni:04X} → 0x{gb:04X} fi={new_fi}")

    # ── Mapped extras → UNICODE_REMAP ──
    for src_ch, tgt_ch in mapped_dict.items():
        src = str(src_ch)
        tgt = str(tgt_ch)
        if len(tgt) != 1:
            print(f"  WARN map '{src}'→'{tgt}': target not single char, skipping")
            continue
        uni = ord(src[0])
        tgt_uni = ord(tgt)
        if tgt_uni not in full_entries and not (0x20 <= tgt_uni <= 0x7E):
            print(f"  WARN map {src[0]}→{tgt}: target not in CODE_TABLE/ASCII, skipping")
            continue
        if tgt_uni not in full_entries:
            print(f"  WARN map {src[0]}→{tgt}: target ASCII but not in CODE_TABLE, skipping")
            continue
        remap_entries.append((uni, tgt_uni))
        print(f"  ~   {src[0]} U+{uni:04X} → {tgt} U+{tgt_uni:04X}")

    print(f"\nTotal CJK: {len(cjk)//CJK_BYTES} glyphs  CODE_TABLE: {len(full_entries)} entries  REMAP: {len(remap_entries)}")

    # ── Verify ──
    errors = []
    for uni, gb in sorted(full_entries.items()):
        idx = (gb - GB_MIN) * 2
        if idx + 1 >= len(lookup):
            errors.append(f"U+{uni:04X} GB 0x{gb:04X}: LOOKUP idx {idx} OOB")
            continue
        fi = struct.unpack_from("<H", lookup, idx)[0]
        if fi == 0xFFFF:
            errors.append(f"U+{uni:04X} GB 0x{gb:04X}: LOOKUP=0xFFFF")
        elif fi >= len(cjk) // CJK_BYTES:
            errors.append(f"U+{uni:04X} GB 0x{gb:04X}: fi={fi} OOB")
    if errors:
        print(f"\n*** {len(errors)} ERRORS ***")
        for e in errors[:20]:
            print(f"  {e}")
        sys.exit(1)
    print(f"Verified: {len(full_entries)} entries OK")

    # ── Write outputs ──
    gen_unicode_gb2312_rs(full_entries, remap_entries,
                          os.path.join(out_dir, "unicode_gb2312.rs"))

    with open(os.path.join(out_dir, "font_cjk.bin"), "wb") as f:
        f.write(cjk)
    with open(os.path.join(out_dir, "font_lookup.bin"), "wb") as f:
        f.write(lookup)

    ascii_data = bytearray()
    for code in range(0x20, 0x7F):
        glyph = render_glyph(chr(code), ASC_W, ASC_H, ASC_BYTES)
        ascii_data.extend(glyph)
    with open(os.path.join(out_dir, "font_ascii.bin"), "wb") as f:
        f.write(ascii_data)

    print(f"Written: font_ascii.bin ({len(ascii_data)} bytes), font_cjk.bin ({len(cjk)} bytes), font_lookup.bin ({len(lookup)} bytes)")
    print("Done.")


def main():
    if "--out-dir" in sys.argv:
        idx = sys.argv.index("--out-dir")
        out_dir = sys.argv[idx + 1]
    else:
        out_dir = os.path.join(PROJ, "src", "font")

    if not os.path.exists(BASE_CT):
        sys.exit(f"ERROR: {BASE_CT} not found.")
    if not os.path.exists(EXTRAS_PATH):
        with open(EXTRAS_PATH, "w") as f:
            json.dump({"font_file_path": "../simsun.ttc", "gb18030": [], "mapped": {}}, f, indent=2, ensure_ascii=False)
        print("Created empty extras.json")
    build(out_dir)


if __name__ == "__main__":
    main()
