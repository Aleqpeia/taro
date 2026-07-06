#!/usr/bin/env python3
"""Crop (and optionally upscale) an 8-bit RGB/RGBA PNG using only the stdlib.

The Read tool downscales large images, so close-up inspection of a capture
(e.g. one card, or the reading-panel text) needs a crop first. No PIL/numpy
here, so this does it by hand with zlib.

Usage: pngcrop.py in.png out.png x y w h [scale]
  x y w h  crop rectangle in source pixels
  scale    integer nearest-neighbour upscale of the crop (default 1)
"""
import struct
import sys
import zlib


def read_png(path):
    with open(path, "rb") as f:
        data = f.read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos = 8
    width = height = bit_depth = color_type = None
    idat = b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        ctype = data[pos + 4:pos + 8]
        chunk = data[pos + 8:pos + 8 + length]
        if ctype == b"IHDR":
            width, height, bit_depth, color_type = struct.unpack(">IIBB", chunk[:10])
        elif ctype == b"IDAT":
            idat += chunk
        elif ctype == b"IEND":
            break
        pos += 12 + length
    assert bit_depth == 8 and color_type in (2, 6), "only 8-bit RGB/RGBA"
    channels = 3 if color_type == 2 else 4
    raw = zlib.decompress(idat)
    stride = width * channels
    rows = []
    prev = bytearray(stride)
    p = 0
    for _ in range(height):
        ft = raw[p]; p += 1
        line = bytearray(raw[p:p + stride]); p += stride
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = prev[i]
            c = prev[i - channels] if i >= channels else 0
            if ft == 1:
                line[i] = (line[i] + a) & 0xFF
            elif ft == 2:
                line[i] = (line[i] + b) & 0xFF
            elif ft == 3:
                line[i] = (line[i] + ((a + b) >> 1)) & 0xFF
            elif ft == 4:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 0xFF
        rows.append(line)
        prev = line
    return width, height, channels, rows


def write_png(path, width, height, channels, rows):
    out = bytearray()
    for line in rows:
        out.append(0)
        out += line
    color_type = 2 if channels == 3 else 6
    ihdr = struct.pack(">IIBBBBB", width, height, 8, color_type, 0, 0, 0)

    def chunk(tag, body):
        return struct.pack(">I", len(body)) + tag + body + struct.pack(
            ">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", ihdr))
        f.write(chunk(b"IDAT", zlib.compress(bytes(out), 9)))
        f.write(chunk(b"IEND", b""))


def main():
    if len(sys.argv) not in (7, 8):
        sys.exit(__doc__)
    inp, outp = sys.argv[1], sys.argv[2]
    x, y, w, h = (int(v) for v in sys.argv[3:7])
    scale = int(sys.argv[7]) if len(sys.argv) == 8 else 1
    W, H, ch, rows = read_png(inp)
    x = max(0, min(x, W)); y = max(0, min(y, H))
    w = min(w, W - x); h = min(h, H - y)
    crop = []
    for j in range(h):
        src = rows[y + j]
        line = bytearray()
        for i in range(w):
            px = src[(x + i) * ch:(x + i) * ch + ch]
            line += px * scale
        for _ in range(scale):
            crop.append(line)
    write_png(outp, w * scale, h * scale, ch, crop)
    print(f"wrote {outp} ({w * scale}x{h * scale})")


if __name__ == "__main__":
    main()
