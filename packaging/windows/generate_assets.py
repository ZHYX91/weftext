"""Rasterize the retained brand's M/L/Z, even-odd artwork (stdlib only).

Fail closed on unsupported SVG features. Output uses 4x4 area sampling; --check
compares decoded pixels, not the runtime-dependent zlib representation.
"""
import argparse
import math
from pathlib import Path
import re
import struct
import xml.etree.ElementTree as ET
import zlib

ROOT = Path(__file__).resolve().parents[2]
BRAND = ROOT / 'brand'
WINDOWS = ROOT / 'packaging/windows/assets'
SAMPLES = 4


def geometry(path):
    # This is a renderer for the two retained assets, not a general SVG engine.
    # Validate the whole document before selecting shapes: ignored siblings,
    # animation, CSS or nested elements could otherwise change browser output.
    source = path.read_text(encoding='utf-8')
    def require(condition):
        if not condition:
            raise ValueError(f'Unsupported brand SVG structure: {path.name}')
    require('<?' not in source and '<!' not in source)
    svg = ET.fromstring(source)
    ns = '{http://www.w3.org/2000/svg}'
    require(svg.tag == ns + 'svg')
    require(svg.attrib == {'viewBox': '0 0 1254 1254', 'role': 'img', 'aria-labelledby': 'title desc'})
    require(path.name in ('app-icon.svg', 'logo-mark.svg'))
    application = path.name == 'app-icon.svg'
    expected = ['title', 'desc', 'rect', 'path'] if application else ['title', 'desc', 'path']
    require([child.tag for child in svg] == [ns + tag for tag in expected])
    require(not (svg.text or '').strip())
    for child, tag in zip(svg, expected):
        require(len(child) == 0 and not (child.tail or '').strip())
        if tag in ('title', 'desc'):
            require(child.attrib == {'id': tag} and bool((child.text or '').strip()))
        else:
            require(not (child.text or '').strip())
    if application:
        require(svg[2].attrib == dict(x='0', y='0', width='1254', height='1254', rx='220', fill='#07172D'))
    node = svg[-1]
    require(set(node.attrib) == {'d', 'fill', 'fill-rule'})
    require(node.attrib['fill-rule'] == 'evenodd')
    require(node.attrib['fill'] == ('#F4EFE4' if application else 'currentColor'))
    number = r'-?\d+(?:\.\d+)?'
    require(not re.sub(r'[A-Za-z]|' + number + r'|\s+', '', node.attrib['d']))
    tokens = re.findall(r'[A-Za-z]|' + number, node.attrib['d'])
    polygons, points = [], []
    while tokens:
        command = tokens.pop(0)
        if command in ('M', 'L'):
            # Each supported subpath is exactly M (x y), then explicit L pairs,
            # then Z. An internal M is not a connecting line in SVG.
            require((command == 'M' and not points) or (command == 'L' and bool(points)))
            require(len(tokens) >= 2 and all(re.fullmatch(number, token) for token in tokens[:2]))
            point = (float(tokens.pop(0)), float(tokens.pop(0)))
            require(all(math.isfinite(coordinate) for coordinate in point))
            points.append(point)
        elif command == 'Z':
            require(len(points) >= 3)
            polygons.append(points)
            points = []
        else:
            raise ValueError(f'Unsupported brand path command: {command}')
    require(not points and len(polygons) == 3)
    return svg, polygons, bytes.fromhex(node.attrib['fill'][1:]) if node.attrib['fill'].startswith('#') else None


def image(size):
    svg, polygons, ink = geometry(BRAND / 'app-icon.svg')
    rect = svg.find('{http://www.w3.org/2000/svg}rect')
    assert rect.attrib == dict(x='0', y='0', width='1254', height='1254', rx='220', fill='#07172D')
    background = bytes.fromhex(rect.attrib['fill'][1:])
    scale = size * SAMPLES / 1254
    width = size * SAMPLES
    totals = [[0, 0, 0, 0] for _ in range(size * size)]
    for y in range(width):
        sy = (y + .5) / scale
        corner = max(220 - sy, sy - (1254 - 220), 0)
        edge = 220 - math.sqrt(220 ** 2 - corner ** 2) if corner else 0
        intersections = sorted(a[0] + (sy - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
                               for polygon in polygons for a, b in zip(polygon, polygon[1:] + polygon[:1])
                               if (a[1] > sy) != (b[1] > sy))
        assert len(intersections) % 2 == 0
        spans = list(zip(intersections[::2], intersections[1::2]))
        for x in range(width):
            sx = (x + .5) / scale
            if not edge <= sx < 1254 - edge:
                continue
            color = ink if any(left <= sx < right for left, right in spans) else background
            pixel = totals[(y // SAMPLES) * size + x // SAMPLES]
            for channel in range(3):
                pixel[channel] += color[channel]
            pixel[3] += 1
    rows = bytearray()
    for index, pixel in enumerate(totals):
        if index % size == 0:
            rows.append(0)
        count = pixel[3]
        rows.extend([round(value / count) if count else 0 for value in pixel[:3]] + [round(count * 255 / SAMPLES ** 2)])
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b'')


def decoded_png(data):
    """Validate the complete supported PNG format before comparing pixels.

    Only non-interlaced 8-bit RGBA with unfiltered rows is emitted here. IDAT
    may be split into adjacent chunks, with any equivalent zlib compression.
    """
    def require(condition):
        if not condition:
            raise ValueError('Incomplete or unsupported generated PNG')
    require(data[:8] == b'\x89PNG\r\n\x1a\n')
    offset, compressed, header = 8, bytearray(), None
    saw_data, ended = False, False
    while offset < len(data):
        require(not ended and len(data) - offset >= 12)
        length = struct.unpack_from('>I', data, offset)[0]
        require(length <= len(data) - offset - 12)
        kind, body = data[offset + 4:offset + 8], data[offset + 8:offset + 8 + length]
        crc = struct.unpack_from('>I', data, offset + 8 + length)[0]
        require(crc == zlib.crc32(kind + body))
        if kind == b'IHDR':
            require(offset == 8 and header is None and length == 13)
            header = body
            width, height, depth, color, compression, filtering, interlace = struct.unpack('>IIBBBBB', header)
            require(0 < width < 2 ** 31 and 0 < height < 2 ** 31)
            require((depth, color, compression, filtering, interlace) == (8, 6, 0, 0, 0))
        elif kind == b'IDAT':
            require(header is not None)
            saw_data = True
            compressed.extend(body)
        elif kind == b'IEND':
            require(header is not None and saw_data and length == 0)
            ended = True
        else:
            require(False)
        offset += length + 12
    require(ended)
    row_length = 1 + width * 4
    expected_length = height * row_length
    decoder = zlib.decompressobj()
    rows = decoder.decompress(compressed, expected_length + 1)
    require(decoder.eof and not decoder.unused_data and not decoder.unconsumed_tail)
    require(len(rows) == expected_length)
    require(all(rows[y * row_length] == 0 for y in range(height)))
    return header, rows


def decoded_ico(data):
    assert struct.unpack_from('<HHH', data) == (0, 1, 4)
    images = []
    offset = 70
    for index, size in enumerate((32, 16, 48, 256)):
        w, h, colors, reserved, planes, bits, length, start = struct.unpack_from('<BBBBHHII', data, 6 + 16 * index)
        assert (w, h, colors, reserved, planes, bits, start) == (size % 256, size % 256, 0, 0, 1, 32, offset)
        images.append(decoded_png(data[start:start + length]))
        offset += length
    assert offset == len(data)
    return images


def assets():
    # Both retained SVG originals carry the identical principal path.
    if geometry(BRAND / 'logo-mark.svg')[1] != geometry(BRAND / 'app-icon.svg')[1]:
        raise ValueError('Principal brand paths differ between retained SVG assets')
    result = {WINDOWS / name: image(size) for name, size in
              (('StoreLogo.png', 50), ('Square150x150Logo.png', 150), ('Square44x44Logo.png', 44))}
    images = [(size, image(size)) for size in (32, 16, 48, 256)]
    offset, headers = 70, []
    for size, data in images:
        headers.append(struct.pack('<BBBBHHII', size % 256, size % 256, 0, 0, 1, 32, len(data), offset))
        offset += len(data)
    result[WINDOWS / 'weftext.ico'] = struct.pack('<HHH', 0, 1, 4) + b''.join(headers) + b''.join(data for _, data in images)
    result[ROOT / 'app/public/brand/app-icon.svg'] = (BRAND / 'app-icon.svg').read_bytes()
    return result


def maintain(check=False):
    for path, data in assets().items():
        if check:
            actual = path.read_bytes()
            decode = {'.ico': decoded_ico, '.png': decoded_png}.get(path.suffix, lambda value: value)
            assert decode(actual) == decode(data), f'Generated brand asset differs: {path.relative_to(ROOT)}'
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    print('PASS: brand geometry and generated native/package/WebUI assets' if check else 'Generated native/package/WebUI brand assets')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    maintain(parser.parse_args().check)
