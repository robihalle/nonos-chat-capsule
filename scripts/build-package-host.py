#!/usr/bin/env python3
"""Build a signed test host with Chat's automatic kernel spawn disabled.

Run scripts/build.sh once first. This reuses its trust policy, enrollment and
already-built capsules, but selects a separate kernel feature profile.
Production deployment is a separate action; this script never restarts a VM.
"""
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import tomllib

root = Path(sys.argv[1]).resolve()
repo = Path(__file__).resolve().parent.parent
rev = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
if rev != (repo / "UPSTREAM_REV").read_text().strip():
    raise SystemExit("Wrong upstream revision.")
cargo = root / "Cargo.toml"
text = cargo.read_text()
features = tomllib.loads(text)["features"]
profile = [x for x in features["microkernel-desktop-base"] if x != "nonos-capsule-chat"]
if "nonos-capsule-chat" in profile or len(profile) != len(features["microkernel-desktop-base"]) - 1:
    raise SystemExit("Unexpected desktop profile.")
name = "microkernel-chat-package-host"
if name in features:
    if features[name] != profile:
        raise SystemExit("Existing package-host profile differs.")
else:
    definition = name + " = [\n" + "".join('  "' + x + '",\n' for x in profile) + "]\n"
    text = text.replace("[features]\n", "[features]\n" + definition, 1)
    cargo.write_text(text)
mk = """.PHONY: chat-package-host
chat-package-host: $(DESKTOP_BASE_CAPSULE_ARTIFACTS) $(snake_ARTIFACTS) $(ZK_POLICY_ROOT) $(foreach s,$(NONOS_ENROLLED_CAPSULES),$($(s)_VERIFY)) nonos-mk-check-deps nonos-mk-ensure-signing-key
\t$(call nonos_kernel_build,Chat package host,microkernel-chat-package-host$(_boot_comma)nonos-stark-attest)
"""
env = dict(os.environ, CARGO_BUILD_JOBS="1")
env["PATH"] = str(Path.home() / ".cargo/bin") + ":" + env["PATH"]
caps = "proof-io ramfs keyring entropy crypto vfs driver-virtio-rng driver-virtio-blk driver-virtio-gpu driver-virtio-net driver-ps2-input driver-xhci driver-usb-hid net-core net-sockets net-nym socks5 policy wallpaper_catalog installer input-router compositor wm desktop-shell image-codec clipboard login wallpaper toolkit about boot-splash calculator snake browser chat wallet-nonos terminal file-manager text-editor settings process-manager attest power audio driver-hda audio_player video-player"
common = ["make", "NONOS_ENROLLED_CAPSULES=" + caps, "NONOS_JOBS=1", "NONOS_DEVICE_BINDING=unbound"]
with tempfile.NamedTemporaryFile(mode="w", suffix=".mk") as rules:
    rules.write(mk)
    rules.flush()
    subprocess.run(common + ["-f", "Makefile", "-f", rules.name, "chat-package-host"], cwd=root, env=env, check=True)
for target in ("nonos-mk-esp", "nonos-mk-iso", "nonos-mk-trust-ledger",
               "nonos-mk-verify-image", "nonos-mk-usb-img"):
    subprocess.run(common + [target], cwd=root, env=env, check=True)
print("Package-test host image ready at " + str(root / "target/nonos.img"))
