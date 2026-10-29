"""Brand regression checks: actual consumer bindings and compressed-image semantics."""
import importlib.util
from pathlib import Path
import struct
import tempfile
from unittest.mock import patch
import xml.etree.ElementTree as ET
import unittest
import zlib

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('brand_assets', ROOT / 'packaging/windows/generate_assets.py')
brand = importlib.util.module_from_spec(spec)
spec.loader.exec_module(brand)


class BrandAssets(unittest.TestCase):
    def test_generated_assets_match(self):
        brand.maintain(check=True)

    def test_rejects_svg_changes_that_would_be_ignored_by_rasterizer(self):
        ns = '{http://www.w3.org/2000/svg}'
        originals = {name: (brand.BRAND / name).read_bytes() for name in ('app-icon.svg', 'logo-mark.svg')}
        for name, original in originals.items():
            def changed(action):
                svg = ET.fromstring(original)
                action(svg)
                return ET.tostring(svg)
            mutations = {
                'extra rectangle': changed(lambda svg: ET.SubElement(svg, ns + 'rect', x='0', y='0', width='1254', height='1254', fill='red')),
                'extra principal path': changed(lambda svg: svg.append(ET.fromstring(ET.tostring(svg[-1])))),
                'duplicate title': changed(lambda svg: ET.SubElement(svg, ns + 'title', id='title')),
                'foreign namespace': changed(lambda svg: ET.SubElement(svg, '{urn:other}rect')),
                'nested title animation': changed(lambda svg: ET.SubElement(svg[0], ns + 'set', attributeName='fill', to='red')),
                'nested description': changed(lambda svg: ET.SubElement(svg[1], ns + 'path', d='M 0 0 L 1254 1254')),
                'nested path animation': changed(lambda svg: ET.SubElement(svg[-1], ns + 'animate', attributeName='d', to='M 0 0 Z')),
                'path transform': changed(lambda svg: svg[-1].set('transform', 'translate(100 0)')),
                'root style': changed(lambda svg: svg.set('style', 'opacity:0')),
                'path color': changed(lambda svg: svg[-1].set('fill', '#FF0000')),
                'wrong metadata element': original.replace(b'<title', b'<path', 1).replace(b'</title>', b'</path>', 1),
                'root visible text': changed(lambda svg: setattr(svg, 'text', 'unsupported')),
                'child tail text': changed(lambda svg: setattr(svg[-1], 'tail', 'unsupported')),
                'external stylesheet': b'<?xml-stylesheet href="other.css" type="text/css"?>' + original,
                'doctype': b'<!DOCTYPE svg>' + original,
            }
            if name == 'app-icon.svg':
                mutations['rectangle animation'] = changed(lambda svg: ET.SubElement(svg[2], ns + 'animate', attributeName='fill', to='red'))
                mutations['rectangle above path'] = changed(lambda svg: svg.__setitem__(slice(None), [svg[0], svg[1], svg[3], svg[2]]))
            for case, modified in mutations.items():
                with self.subTest(asset=name, case=case), tempfile.TemporaryDirectory(prefix='weftext-brand-') as folder:
                    directory = Path(folder)
                    for asset, data in originals.items():
                        (directory / asset).write_bytes(modified if asset == name else data)
                    with patch.object(brand, 'BRAND', directory), self.assertRaises(ValueError):
                        brand.assets()

    def test_path_state_is_validated_through_the_asset_consumer(self):
        originals = {name: (brand.BRAND / name).read_bytes() for name in ('app-icon.svg', 'logo-mark.svg')}
        path = ET.fromstring(originals['app-icon.svg'])[-1].attrib['d']
        mutations = {
            'internal moveto': path.replace(' L ', ' M ', 1),
            'initial lineto': path.replace('M ', 'L ', 1),
            'initial close': 'Z ' + path,
            'close without active subpath': path.replace(' Z M ', ' Z Z M ', 1),
            'lineto after closed subpath': path.replace(' Z M ', ' Z L ', 1),
            'missing close before moveto': path.replace(' Z M ', ' M ', 1),
            'unclosed final subpath': path.rsplit(' Z', 1)[0],
            'missing coordinate': path.replace('619.12 ', '', 1),
            'incomplete trailing lineto': path.rsplit(' Z', 1)[0] + ' L 5',
            'incomplete trailing moveto': path + ' M 5',
            'implicit lineto coordinates': path.replace(' L ', ' ', 1),
            'relative moveto': path.replace('M ', 'm ', 1),
            'relative lineto': path.replace(' L ', ' l ', 1),
            'unsupported curve': path.replace(' L ', ' C ', 1),
            'unsupported separator': path.replace('619.12 ', '619.12,', 1),
            'empty path': '',
            'degenerate subpaths': 'M 1 1 L 2 2 Z M 3 3 L 4 4 Z M 5 5 L 6 6 Z',
        }
        baseline = brand.assets()
        # Mutate BOTH SVGs identically so the path-equality guard cannot mask a
        # parser failure. Exercise the actual native/package/WebUI producer.
        positive = path.replace(' L ', '\r\nL\t').replace(' M ', '\nM\t')
        for case, modified in {**mutations, 'explicit commands with whitespace': positive}.items():
            with self.subTest(case=case), tempfile.TemporaryDirectory(prefix='weftext-brand-path-') as folder:
                directory = Path(folder)
                for name, original in originals.items():
                    svg = ET.fromstring(original)
                    svg[-1].set('d', modified)
                    (directory / name).write_bytes(ET.tostring(svg))
                with patch.object(brand, 'BRAND', directory):
                    if case in mutations:
                        with self.assertRaises(ValueError):
                            brand.assets()
                    else:
                        actual = brand.assets()
                        for output, expected in baseline.items():
                            if output.suffix in ('.png', '.ico'):
                                self.assertEqual(actual[output], expected)
                        self.assertEqual(actual[ROOT / 'app/public/brand/app-icon.svg'], (directory / 'app-icon.svg').read_bytes())

    def test_path_lexing_rejects_unicode_numbers_and_whitespace_in_actual_assets(self):
        originals = {name: (brand.BRAND / name).read_bytes() for name in ('app-icon.svg', 'logo-mark.svg')}
        path = ET.fromstring(originals['app-icon.svg'])[-1].attrib['d']
        mutations = {f'space U+{ord(character):04X}': path.replace('619.12 ', '619.12' + character, 1)
                     for character in ('\u00a0', '\u2003', '\u2009', '\u2028', '\u3000')}
        mutations.update({f'digit U+{ord(character):04X}': path.replace('619.12', character + '19.12', 1)
                          for character in ('\uff16', '\u0666', '\u096c')})
        for scope in ('app only', 'both originals'):
            for case, modified in mutations.items():
                with self.subTest(scope=scope, case=case), tempfile.TemporaryDirectory(prefix='weftext-brand-lex-') as folder:
                    directory = Path(folder)
                    for name, original in originals.items():
                        if scope == 'both originals' or name == 'app-icon.svg':
                            svg = ET.fromstring(original)
                            svg[-1].set('d', modified)
                            original = ET.tostring(svg)
                        (directory / name).write_bytes(original)
                    with patch.object(brand, 'BRAND', directory), self.assertRaises(ValueError):
                        brand.assets()

    def test_compression_bytes_are_not_pixel_identity(self):
        original = brand.image(16)
        header, rows = brand.decoded_png(original)
        def chunk(kind, data):
            return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
        alternative = original[:8] + chunk(b'IHDR', header) + chunk(b'IDAT', zlib.compress(rows, level=0)) + chunk(b'IEND', b'')
        self.assertNotEqual(original, alternative)
        self.assertEqual(brand.decoded_png(original), brand.decoded_png(alternative))
        damaged = bytearray(alternative)
        damaged[50] ^= 1
        with self.assertRaises(ValueError):
            brand.decoded_png(damaged)

    def test_png_requires_complete_supported_chunk_sequence_and_rows(self):
        original = brand.image(16)
        header, rows = brand.decoded_png(original)
        signature = original[:8]
        def chunk(kind, data):
            return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
        ihdr = chunk(b'IHDR', header)
        idat = chunk(b'IDAT', zlib.compress(rows))
        iend = chunk(b'IEND', b'')
        def png(info=header, pixels=rows, compressed=None):
            return signature + chunk(b'IHDR', info) + chunk(b'IDAT', zlib.compress(pixels) if compressed is None else compressed) + iend
        variants = {
            'missing IEND': original[:-12],
            'truncated chunk': original[:-1],
            'missing IHDR': signature + idat + iend,
            'IHDR after IDAT': signature + idat + ihdr + iend,
            'duplicate IHDR': signature + ihdr + ihdr + idat + iend,
            'missing IDAT': signature + ihdr + iend,
            'IEND before IDAT': signature + ihdr + iend + idat,
            'duplicate IEND': original + iend,
            'trailing bytes': original + b'ignored',
            'nonempty IEND': signature + ihdr + idat + chunk(b'IEND', b'x'),
            'noncontiguous IDAT': signature + ihdr + idat + chunk(b'tEXt', b'key\0value') + idat + iend,
            'wrong IHDR size': png(info=header[:-1]),
            'zero width': png(info=bytes(4) + header[4:]),
            'unsupported color format': png(info=header[:9] + bytes((2,)) + header[10:]),
            'wrong row length': png(pixels=rows[:-1]),
            'extra decoded rows': png(pixels=rows + bytes(65)),
            'unsupported filter': png(pixels=bytes((1,)) + rows[1:]),
            'truncated zlib stream': png(compressed=zlib.compress(rows)[:-1]),
            'trailing zlib data': png(compressed=zlib.compress(rows) + b'ignored'),
        }
        for case, data in variants.items():
            with self.subTest(case=case), self.assertRaises(ValueError):
                brand.decoded_png(data)
        # Adjacent IDAT chunks remain a valid representation of the same image.
        compressed = zlib.compress(rows, level=0)
        split = signature + ihdr + chunk(b'IDAT', compressed[:7]) + chunk(b'IDAT', compressed[7:]) + iend
        self.assertEqual(brand.decoded_png(original), brand.decoded_png(split))

    def test_actual_consumer_bindings(self):
        for name in ('index.html', 'hosted.html', 'capture.html'):
            self.assertIn('href="/brand/app-icon.svg"', (ROOT / 'app' / name).read_text(encoding='utf-8'))
        for name in ('main.ts', 'hosted.ts'):
            self.assertIn('/brand/app-icon.svg', (ROOT / 'app/src' / name).read_text(encoding='utf-8'))
        self.assertIn('assets/weftext.ico', (ROOT / 'crates/weftext-app/build.rs').read_text())
        tray = (ROOT / 'crates/weftext-app/src/native/tray.rs').read_text()
        self.assertIn('packaging/windows/assets/weftext.ico', tray)
        # Tray loads the first ICO image at 32px; preserve a native 32px first entry.
        ico = (brand.WINDOWS / 'weftext.ico').read_bytes()
        self.assertEqual(ico[6:8], bytes((32, 32)))
        package = (ROOT / 'scripts/windows_package.py').read_text()
        for name in ('StoreLogo.png', 'Square150x150Logo.png', 'Square44x44Logo.png', 'weftext.ico'):
            self.assertIn('Assets/' + name, package)
        self.assertIn("'BackgroundColor': '#07172D'", package)


if __name__ == '__main__':
    unittest.main()
