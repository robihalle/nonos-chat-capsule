#!/usr/bin/env python3
"""Exercise real package signatures, roundtrip and corruption rejection."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile


def call(*args, success=True):
    p = subprocess.run([str(x) for x in args], capture_output=True)
    if (p.returncode == 0) != success:
        raise SystemExit(p.stdout.decode(errors="replace") + p.stderr.decode(errors="replace"))
    return p.stdout


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("upstream", type=Path)
    ap.add_argument("release", type=Path)
    args = ap.parse_args()
    root, release = args.upstream.resolve(), args.release.resolve()
    pack = root / "tools/nonos-pack/target/release/nonos-pack"
    signer = root / "nonos-sign/target/release/capsule-sign"
    info = json.loads((release / "release.json").read_text())
    pkg = release / info["package"]
    data = pkg.read_bytes()
    assert hashlib.sha256(data).hexdigest() == info["package_sha256"]
    call(pack, "verify", "--in", pkg)
    count = struct.unpack_from(">H", data, 6)[0]
    assert data[:4] == b"NOS1" and count == 4
    with tempfile.TemporaryDirectory(prefix="nonos-chat-package-test-") as tmp:
        tmp = Path(tmp)
        unpack = tmp / "unpacked"
        call(pack, "unpack", "--in", pkg, "--out-dir", unpack)
        for ext, digest in info["artifacts_sha256"].items():
            assert hashlib.sha256((unpack / (pkg.stem + ext)).read_bytes()).hexdigest() == digest
        base = unpack / pkg.stem
        verify = [signer, "verify-manifest", "--manifest", str(base) + ".manifest.bin",
                  "--cert", str(base) + ".nonos_id_cert.bin",
                  "--policy", root / "nonos-data/trust/policy/nonos_trust_anchor.policy.bin",
                  "--elf", str(base) + ".elf"]
        call(*verify)
        # Mutate the payload of each section, plus framing and signature bytes.
        for i in range(count):
            kind, offset, length = struct.unpack_from(">H2xII4x", data, 8 + i * 16)
            assert length > 0 and offset + length <= len(data), (kind, offset, length)
            bad = bytearray(data)
            bad[offset + length // 2] ^= 1
            path = tmp / ("bad-section-" + str(kind) + ".nonos")
            path.write_bytes(bad)
            call(pack, "verify", "--in", path, success=False)
        for name, bad in (("bad-magic", b"BAD!" + data[4:]),
                          ("truncated", data[:-1]),
                          ("bad-signature", data[:-1] + bytes([data[-1] ^ 1])),
                          ("trailing-data", data + b"x")):
            path = tmp / (name + ".nonos")
            path.write_bytes(bad)
            call(pack, "verify", "--in", path, success=False)
        # The committed upstream policy is a different, official trust domain.
        original = call("git", "-C", root / "nonos-data", "show",
                        "HEAD:trust/policy/nonos_trust_anchor.policy.bin")
        official = tmp / "official-policy.bin"
        official.write_bytes(original)
        if original != (root / "nonos-data/trust/policy/nonos_trust_anchor.policy.bin").read_bytes():
            alternate = list(verify)
            alternate[alternate.index("--policy") + 1] = official
            call(*alternate, success=False)
        elf = Path(str(base) + ".elf")
        body = bytearray(elf.read_bytes())
        body[-1] ^= 1
        elf.write_bytes(body)
        call(*verify, success=False)
    print("PASS: package hash, dual signatures, 4-section roundtrip, manifest/ELF binding, "
          "8 corrupt packages rejected, stale ELF rejected.")


if __name__ == "__main__":
    main()
