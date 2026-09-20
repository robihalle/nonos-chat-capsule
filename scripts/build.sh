#!/usr/bin/env bash
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
tree="${1:?Usage: scripts/build.sh /path/to/pinned/microkernel}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export PATH="$HOME/.cargo/bin:$PATH"
python3 "$repo/scripts/integrate.py" "$tree"
cd "$tree"
(cd nonos-sign && cargo build --release --bin capsule-sign)
python3 "$repo/scripts/init-trust.py" "$tree"
(cd userland/capsule_chat && cargo generate-lockfile --locked)
# Enroll only capsules shipped by the QEMU desktop profile.
caps="proof-io ramfs keyring entropy crypto vfs driver-virtio-rng driver-virtio-blk driver-virtio-gpu driver-virtio-net driver-ps2-input driver-xhci driver-usb-hid net-core net-sockets net-nym socks5 policy wallpaper_catalog installer input-router compositor wm desktop-shell image-codec clipboard login wallpaper toolkit about boot-splash calculator snake browser chat wallet-nonos terminal file-manager text-editor settings process-manager attest power audio driver-hda audio_player video-player"
make NONOS_ENROLLED_CAPSULES="$caps" NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-chat-desktop-prod
make NONOS_ENROLLED_CAPSULES="$caps" NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-esp
make NONOS_ENROLLED_CAPSULES="$caps" NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-iso
make NONOS_ENROLLED_CAPSULES="$caps" NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-trust-ledger
make NONOS_ENROLLED_CAPSULES="$caps" NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-verify-image

make NONOS_ENROLLED_CAPSULES="$caps" NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-usb-img
