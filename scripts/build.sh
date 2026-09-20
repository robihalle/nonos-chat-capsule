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
(cd userland/capsule_chat && cargo generate-lockfile)
# Enroll only capsules shipped by the QEMU desktop profile.
cat > mk/25-chat.mk <<'MAKE'
NONOS_ENROLLED_CAPSULES := $(DESKTOP_BASE_SLUGS) std-proof ripgrep sd flacprobe csview huniq tokei jsonxf pastel dotenv-linter grex
MAKE
make NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-desktop-gui-prod
make NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-esp
make NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-iso
make NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-trust-ledger
make NONOS_JOBS=1 NONOS_DEVICE_BINDING=unbound nonos-mk-verify-image
