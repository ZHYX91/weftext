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
        with self.assertRaises(AssertionError):
            brand.decoded_png(damaged)

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
