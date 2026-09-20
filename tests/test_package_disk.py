import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("package_disk", Path(__file__).resolve().parents[1] / "scripts/make-package-disk.py")
disk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(disk)


class TransferDiskTests(unittest.TestCase):
    def test_guest_store_layout_and_exact_payload(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            package = root / "chat.nonos"
            body = b"NOS1" + bytes(range(256)) * 16
            package.write_bytes(body)
            out = root / "store.img"
            disk.stage(package, out)
            self.assertEqual(out.stat().st_size, disk.DISK_SIZE)
            with out.open("rb") as f:
                f.seek(disk.BASE)
                self.assertEqual(f.read(8), b"NONOSTR1")
                self.assertEqual(struct.unpack("<II", f.read(8)), (1, 1))
                f.seek(disk.BASE + 32)
                self.assertEqual(f.read(96).rstrip(b"\0"), b"/pkgs/chat.nonos")
                offset, length = struct.unpack("<QQ", f.read(16))
                self.assertEqual(offset % 512, 0)
                self.assertGreaterEqual(offset, disk.BASE + disk.TOC_SIZE)
                f.seek(offset)
                self.assertEqual(f.read(length), body)
            # Simulate a full native TOC rewrite after installing more capsules.
            with out.open("r+b") as f:
                f.seek(disk.BASE)
                f.write(bytes(disk.TOC_SIZE))
                f.seek(offset)
                self.assertEqual(f.read(length), body)

    def test_existing_disk_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            package, out = root / "chat.nonos", root / "important.img"
            package.write_bytes(b"NOS1xxxx")
            out.write_bytes(b"KEEP")
            with self.assertRaises(FileExistsError):
                disk.stage(package, out)
            self.assertEqual(out.read_bytes(), b"KEEP")

    def test_bad_packages_create_no_disk(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, body in (("bad.nonos", b"invalid!"), ("bad.zip", b"NOS1xxxx"), ("bad name.nonos", b"NOS1xxxx")):
                package, out = root / name, root / "store.img"
                package.write_bytes(body)
                with self.assertRaises(ValueError):
                    disk.stage(package, out)
                self.assertFalse(out.exists())

    def test_oversized_package_is_rejected_before_read(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            package = root / "huge.nonos"
            with package.open("wb") as f:
                f.write(b"NOS1")
                f.truncate(disk.MAX_PACKAGE + 1)
            with self.assertRaises(ValueError):
                disk.stage(package, root / "store.img")
            self.assertFalse((root / "store.img").exists())


if __name__ == "__main__":
    unittest.main()
