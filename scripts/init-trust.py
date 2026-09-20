#!/usr/bin/env python3
"""Initialize a private build identity in a dedicated upstream clone, once."""
from pathlib import Path
import subprocess, re, os, sys
root=Path(sys.argv[1]).resolve()
os.chdir(root)
marker=root/".keys/chat-owner-identity"
if marker.exists():
 print("Existing private build identity retained.");sys.exit(0)
cs=root/"nonos-sign/target/release/capsule-sign"
if not cs.exists():raise SystemExit("Build nonos-sign first.")
(root/".keys").mkdir(mode=0o700,exist_ok=True)
os.chmod(root/".keys",0o700)
keydir=root/"nonos-data/trust/keys";keydir.mkdir(parents=True,exist_ok=True)
names=["nonos_trust_anchor"]
for p in (root/"userland").glob("*/Capsule.mk"):
 m=re.search(r"^CAPSULE_BIN_NAME\s*:=\s*(\S+)",p.read_text(),re.M)
 if m:names.append(m.group(1)+"_publisher")
for name in sorted(set(names)):
 for alg in ("ed25519","mldsa65"):
  prefix=root/".keys"/(name+"_"+alg)
  if prefix.with_suffix(".seed").exists():raise SystemExit("Partial identity found; inspect before regenerating.")
  subprocess.run([str(cs),"keygen","--alg",alg,"--out",str(prefix)],check=True,stdout=subprocess.DEVNULL)
  os.chmod(prefix.with_suffix(".seed"),0o600)
  prefix.with_suffix(".pub").replace(keydir/(prefix.name+".pub"))
# These are committed upstream signatures in this disposable build clone only.
# They must be rebuilt under our own identity; source and original VPS image remain intact.
for pattern in ("capsules/*.nonos_id_cert.bin","capsules/*.manifest.bin","policy/nonos_trust_anchor.policy.bin"):
 for p in (root/"nonos-data/trust").glob(pattern):p.unlink()
marker.write_text("Private image identity; never commit .keys or seed files.\n")
os.chmod(marker,0o600)
print("Private signing identity initialized.")
