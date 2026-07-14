#!/usr/bin/env python3
"""mkfw — Odroid-GO firmware packer.

Packs a firmware image into an .fw container for the Odroid-GO bootloader.

Format:
    24 bytes  — header "ODROIDGO_FIRMWARE_V00_01"
    40 bytes  — description (null-padded)
  8256 bytes  — tile image (86x48, RGB565 raw)
    per partition:
      28 bytes  — partition struct (type, subtype, reserved x2, label[16], flags, length)
       4 bytes  — data_length (uint32 LE)
       N bytes  — raw binary data
     4 bytes  — CRC-32 (over entire file before checksum)

Usage:
    mkfw <description> <tile> <type> <subtype> <length> <label> <binary> \
         [<type> <subtype> <length> <label> <binary> ...]

Example:
    mkfw "ogo-shell-rs(a1b2c3d)" media/tile.raw 0 16 3000000 ogo-shell-rs build/ogo-shell-rs.bin

Output is written to firmware.fw in the current directory.
"""

import struct
import sys
import os
import binascii
import io

HEADER = b"ODROIDGO_FIRMWARE_V00_01"
DESCRIPTION_SIZE = 40
TILE_SIZE = 86 * 48 * 2


def crc32(data: bytes) -> int:
    return binascii.crc32(data) & 0xFFFFFFFF


def write_partition(f: io.BufferedWriter, ptype: int, subtype: int, length: int,
                    label: str, binary_path: str) -> None:
    label_bytes = label.encode("ascii", errors="replace")[:16]
    label_bytes = label_bytes.ljust(16, b"\x00")

    with open(binary_path, "rb") as bf:
        raw = bf.read()

    data_len = len(raw)

    header = struct.pack("<BBBB16sII",
                         ptype & 0xFF,
                         subtype & 0xFF,
                         0, 0,
                         label_bytes,
                         0,          # flags
                         length)      # allocated flash length
    f.write(header)
    f.write(struct.pack("<I", data_len))
    f.write(raw)


def main() -> None:
    args = sys.argv[1:]

    if len(args) < 7 or (len(args) - 2) % 5 != 0:
        print("usage: mkfw <description> <tile> <type> <subtype> <length> <label> <binary> "
              "[<type> <subtype> <length> <label> <binary> ...]")
        sys.exit(1)

    description = args[0]
    tile_path = args[1]
    partitions = args[2:]

    desc_bytes = description.encode("ascii", errors="replace")[:DESCRIPTION_SIZE - 1]
    desc_bytes = desc_bytes.ljust(DESCRIPTION_SIZE, b"\x00")
    if len(desc_bytes) > DESCRIPTION_SIZE:
        desc_bytes = desc_bytes[:DESCRIPTION_SIZE]

    with open(tile_path, "rb") as tf:
        tile_data = tf.read()

    if len(tile_data) != TILE_SIZE:
        print(f"tile file has wrong size (expected {TILE_SIZE}, got {len(tile_data)}): {tile_path}",
              file=sys.stderr)
        sys.exit(1)

    buf = io.BytesIO()
    assert len(HEADER) == 24
    buf.write(HEADER)
    buf.write(desc_bytes)
    buf.write(tile_data)

    for i in range(0, len(partitions), 5):
        ptype = int(partitions[i])
        subtype = int(partitions[i + 1])
        length = int(partitions[i + 2])
        label = partitions[i + 3]
        binary_path = partitions[i + 4]
        write_partition(buf, ptype, subtype, length, label, binary_path)

    full = buf.getvalue()
    checksum = crc32(full)
    full_with_crc = full + struct.pack("<I", checksum)

    out_path = "firmware.fw"
    with open(out_path, "wb") as out:
        out.write(full_with_crc)

    print(f"Wrote {out_path} ({len(full_with_crc)} bytes, crc={checksum:08x})")


if __name__ == "__main__":
    main()
