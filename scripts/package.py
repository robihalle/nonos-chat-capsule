#!/usr/bin/env python3
"""Seal the already-built Chat artifacts into the upstream NOS1 package format."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def run(*args):
    subprocess.run([str(x) for x in args], check=True)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("upstream", type=Path)
    ap.add_argument("--out", type=Path, required=True, help="New release directory; must not exist")
    args = ap.parse_args()
    repo = Path(__file__).resolve().parent.parent
    root = args.upstream.resolve()
    out = args.out.resolve()
    revision = (repo / "UPSTREAM_REV").read_text().strip()
    actual = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
    if actual != revision:
        raise SystemExit("Wrong upstream revision; use UPSTREAM_REV.")
    if out.exists():
        raise SystemExit("Output already exists; choose a new release directory.")
    for name in ("app.rs", "protocol.rs", "transport.rs"):
        if (repo / "capsule/src" / name).read_bytes() != (root / "userland/capsule_chat/src" / name).read_bytes():
            raise SystemExit("Integrated Chat source differs: " + name)
    pack = root / "tools/nonos-pack/target/release/nonos-pack"
    signer = root / "nonos-sign/target/release/capsule-sign"
    policy = root / "nonos-data/trust/policy/nonos_trust_anchor.policy.bin"
    trust = root / "nonos-data/trust/capsules"
    artifacts = {
        ".elf": root / "userland/capsule_chat/target/x86_64-nonos-user/release/chat",
        ".manifest.bin": trust / "chat.manifest.bin",
        ".nonos_id_cert.bin": trust / "chat.nonos_id_cert.bin",
        ".zk_trailer.bin": trust / "chat.zk_trailer.bin",
    }
    seeds = {alg: root / (".keys/chat_publisher_" + alg + ".seed") for alg in ("ed25519", "mldsa65")}
    for p in [pack, signer, policy, *artifacts.values(), *seeds.values()]:
        if not p.is_file():
            raise SystemExit("Missing build input: " + str(p))
    if not artifacts[".zk_trailer.bin"].stat().st_size:
        raise SystemExit("This release requires its existing STARK trailer.")
    for p in seeds.values():
        if os.name == "posix" and p.stat().st_mode & 0o077:
            raise SystemExit("Signing seed must be private (chmod 600): " + str(p))
    run(signer, "verify-manifest", "--manifest", artifacts[".manifest.bin"],
        "--cert", artifacts[".nonos_id_cert.bin"], "--policy", policy,
        "--elf", artifacts[".elf"])
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".chat-package-", dir=out.parent) as scratch:
        stage = Path(scratch) / "release"
        stage.mkdir()
        pkg = stage / "nonos-chat-0.1.0.nonos"
        run(pack, "pack", "--out", pkg, "--manifest", artifacts[".manifest.bin"],
            "--elf", artifacts[".elf"], "--id-cert", artifacts[".nonos_id_cert.bin"],
            "--trailer", artifacts[".zk_trailer.bin"],
            "--seed", "ed25519=" + str(seeds["ed25519"]),
            "--seed", "mldsa65=" + str(seeds["mldsa65"]))
        run(pack, "verify", "--in", pkg)
        unpacked = Path(scratch) / "unpacked"
        run(pack, "unpack", "--in", pkg, "--out-dir", unpacked)
        for ext, original in artifacts.items():
            if (unpacked / (pkg.stem + ext)).read_bytes() != original.read_bytes():
                raise SystemExit("Package roundtrip changed artifact " + ext)
        metadata = {
            "schema": 1, "name": "NONOS Chat", "version": "0.1.0",
            "package": pkg.name, "package_sha256": sha(pkg),
            "package_bytes": pkg.stat().st_size, "format": "NOS1",
            "namespace": "local.nonoschat.app.chat", "install_name": "chat",
            "target": "x86_64-nonos-user", "upstream_revision": revision,
            "trust_policy_sha256": sha(policy),
            "attestation_root_sha256": sha(root / "nonos-data/trust/policy/zk_capsule_policy_root.bin"),
            "required_caps": "0x183d", "optional_caps": "0x0",
            "permissions": ["CoreExec", "Network", "IPC", "Memory", "Crypto",
                            "GraphicsDisplayQuery", "GraphicsSurfaceCreate"],
            "publisher_public_key_sha256": {
                alg: sha(root / ("nonos-data/trust/keys/chat_publisher_" + alg + ".pub"))
                for alg in seeds
            },
            "artifacts_sha256": {ext: sha(p) for ext, p in artifacts.items()},
            "source": "https://github.com/robihalle/nonos-chat-capsule",
            "compatibility": "Requires a host trusting this publisher certificate and attestation root. Not an official NONOS release.",
            "contains_credentials": False,
        }
        (stage / "release.json").write_text(json.dumps(metadata, indent=2) + "\n")
        shutil.copyfile(repo / "docs/INSTALL.md", stage / "INSTALL.md")
        shutil.copyfile(repo / "LICENSE", stage / "LICENSE")
        files = sorted(stage.iterdir())
        (stage / "SHA256SUMS").write_text("".join(sha(p) + "  " + p.name + "\n" for p in files))
        # Publish only this allowlist; no recursive copy of the build checkout or .keys.
        stage.rename(out)
    print("Release ready: " + str(out))


if __name__ == "__main__":
    main()
