"""Brand regression checks: actual consumer bindings and compressed-image semantics."""
import importlib.util
from pathlib import Path
import struct
import unittest
import zlib

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('brand_assets', ROOT / 'packaging/windows/generate_assets.py')
brand = importlib.util.module_from_spec(spec)
spec.loader.exec_module(brand)


class BrandAssets(unittest.TestCase):
    def test_generated_assets_match(self):
        brand.maintain(check=True)

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
