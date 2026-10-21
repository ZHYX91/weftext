"""Maintain native/package icons from the repository's simple vector geometry."""
import argparse
from pathlib import Path
import struct
import xml.etree.ElementTree as ET
import zlib

ROOT = Path(__file__).resolve().parent


def image(size):
    svg = ET.parse(ROOT / 'icon.svg').getroot()
    shapes = []
    for node in svg:
        color = bytes.fromhex(node.attrib['fill'][1:])
        if node.tag.endswith('rect'):
            x, y, width, height = (float(node.attrib[name]) for name in ('x', 'y', 'width', 'height'))
            points = [(x, y), (x + width, y), (x + width, y + height), (x, y + height)]
        else:
            points = [tuple(map(float, point.split(','))) for point in node.attrib['points'].split()]
        shapes.append((points, color))
    def inside(x, y, points):
        hit = False
        previous = points[-1]
        for current in points:
            if (current[1] > y) != (previous[1] > y) and x < (previous[0] - current[0]) * (y - current[1]) / (previous[1] - current[1]) + current[0]:
                hit = not hit
            previous = current
        return hit
    pixels = bytearray()
    for y in range(size):
        pixels.append(0)
        for x in range(size):
            totals = [0, 0, 0, 0]
            for sy in range(3):
                for sx in range(3):
                    color = None
                    for points, fill in shapes:
                        if inside((x + (sx + .5) / 3) * 100 / size, (y + (sy + .5) / 3) * 100 / size, points):
                            color = fill
                    if color is not None:
                        for channel in range(3):
                            totals[channel] += color[channel]
                        totals[3] += 255
            alpha = totals[3] // 9
            pixels.extend(bytes([round(total * 255 / totals[3]) if totals[3] else 0 for total in totals[:3]] + [alpha]))
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(pixels)) + chunk(b'IEND', b'')


def assets():
    result = {name: image(size) for name, size in (('StoreLogo.png', 50), ('Square150x150Logo.png', 150), ('Square44x44Logo.png', 44))}
    images = [(size, image(size)) for size in (16, 32, 48, 256)]
    offset = 6 + 16 * len(images)
    headers = []
    for size, data in images:
        headers.append(struct.pack('<BBBBHHII', size % 256, size % 256, 0, 0, 1, 32, len(data), offset))
        offset += len(data)
    result['weftext.ico'] = struct.pack('<HHH', 0, 1, len(images)) + b''.join(headers) + b''.join(data for _, data in images)
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, data in assets().items():
        path = ROOT / 'assets' / name
        if args.check:
            if path.read_bytes() != data:
                raise ValueError(f'Generated icon differs from vector source: {name}')
        else:
            path.parent.mkdir(exist_ok=True)
            with path.open('xb') as stream:
                stream.write(data)
