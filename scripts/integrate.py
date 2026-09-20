#!/usr/bin/env python3
"""Apply this capsule to the exact upstream checkout; no trust checks disabled."""
from pathlib import Path
import shutil, subprocess, sys, re
repo=Path(__file__).resolve().parents[1]
root=Path(sys.argv[1]).resolve()
expected=(repo/"UPSTREAM_REV").read_text().strip()
actual=subprocess.check_output(["git","-C",str(root),"rev-parse","HEAD"],text=True).strip()
if actual!=expected: raise SystemExit(f"Expected upstream {expected}, got {actual}")
def change(path,old,new):
 p=root/path;s=p.read_text()
 if new in s:return
 if old not in s:raise SystemExit(f"Integration anchor missing: {path}")
 p.write_text(s.replace(old,new,1))
target=root/"userland/capsule_chat"
shutil.copytree(repo/"capsule",target,dirs_exist_ok=True)
# Reuse the existing socket IPC adapters and key map with their original license.
src=root/"userland/capsule_browser/src/browser"
shutil.copytree(src/"net",target/"src/net",dirs_exist_ok=True)
shutil.copy2(src/"keymap.rs",target/"src/keymap.rs")
# Start in direct transport mode. The shared adapters default to direct sockets.
change(Path("mk/20-build.mk"),"include userland/capsule_browser/Capsule.mk","include userland/capsule_browser/Capsule.mk\ninclude userland/capsule_chat/Capsule.mk")
change(Path("mk/20-build.mk"),"browser wallet-nonos terminal file-manager text-editor","browser chat wallet-nonos terminal file-manager text-editor")
change(Path("Cargo.toml"),'nonos-capsule-browser           = []','nonos-capsule-browser           = []\nnonos-capsule-chat              = []')
change(Path("Cargo.toml"),'  "nonos-capsule-browser",','  "nonos-capsule-browser",\n  "nonos-capsule-chat",')
change(Path("src/userspace/mod.rs"),"pub mod capsule_browser;","pub mod capsule_browser;\npub mod capsule_chat;")
change(Path("src/userspace/init/spawn_plan/apps.rs"),"    super::apps_tools::spawn();","    super::apps_tools::spawn();\n    spawn_chat();")
apps=root/"src/userspace/init/spawn_plan/apps.rs"
if "fn spawn_chat()" not in apps.read_text():
 with apps.open("a") as f:f.write('''
#[cfg(feature = "nonos-capsule-chat")]
fn spawn_chat() {
    use crate::userspace::capsule_chat as c;
    super::boot::capsule("APP-CHAT", "app_chat", c::spawn_chat_capsule, c::shared_state);
}
#[cfg(not(feature = "nonos-capsule-chat"))]
fn spawn_chat() {}
''')
mirror=root/"src/userspace/capsule_chat";mirror.mkdir(exist_ok=True)
for name in ("mod.rs","state.rs","embed.rs","spawn.rs"):
 s=(root/"src/userspace/capsule_browser"/name).read_text()
 s=s.replace("browser","chat").replace("BROWSER","CHAT")
 s=s.replace("systems.nonos","local.nonoschat")
 for a,b in [("4760","4990"),("4761","4991"),("4762","4992"),("4763","4993"),("4764","4994"),("4765","4995"),("4766","4996"),("4767","4997")]:s=s.replace(a,b)
 (mirror/name).write_text(s)
# Match the browser mirror's declared optional instance endpoints.
p=target/"Capsule.mk";s=p.read_text()
s=s.replace("include nonos-mk/capsule.mk",'''CAPSULE_INSTANCE_ENDPOINTS := service:4992:app.chat.1 reply:4993:endpoint.app.chat.1.reply service:4994:app.chat.2 reply:4995:endpoint.app.chat.2.reply service:4996:app.chat.3 reply:4997:endpoint.app.chat.3.reply
include nonos-mk/capsule.mk''')
p.write_text(s)
print("Chat capsule integrated. Signatures and STARK admission remain enforced.")

# Add a launcher entry and the same verified instance-spawn path used by other apps.
change(Path("userland/capsule_desktop_shell/src/state/apps.rs"),
       "pub const LAUNCHER_APPS: [LauncherApp; 12] = [",
       'pub const LAUNCHER_APPS: [LauncherApp; 13] = [\n    LauncherApp { icon: LauncherIcon::TextEditor, label: b"Chat", service: b"app.chat" },')
change(Path("src/userspace/init/instance_spawn/queue.rs"),"    Browser,","    Browser,\n    Chat,")
change(Path("src/userspace/init/instance_spawn/queue.rs"),'            PendingApp::Browser => b"app.browser",','            PendingApp::Browser => b"app.browser",\n            PendingApp::Chat => b"app.chat",')
change(Path("src/syscall/microkernel/spawn_instance.rs"),'        "app.browser" => PendingApp::Browser,','        "app.browser" => PendingApp::Browser,\n        "app.chat" => PendingApp::Chat,')
change(Path("src/userspace/init/instance_spawn/service.rs"),"            PendingApp::Browser => spawn_browser(),","            PendingApp::Browser => spawn_browser(),\n            PendingApp::Chat => spawn_chat(),")
service=root/"src/userspace/init/instance_spawn/service.rs"
if "fn spawn_chat()" not in service.read_text():
 with service.open("a") as f:f.write('''
#[cfg(feature = "nonos-capsule-chat")]
fn spawn_chat() -> Result<u32, SpawnError> {
    crate::userspace::capsule_chat::spawn_chat_instance()
}
#[cfg(not(feature = "nonos-capsule-chat"))]
fn spawn_chat() -> Result<u32, SpawnError> {
    Err(SpawnError::FeatureDisabled)
}
''')

# Signed QEMU desktop with all graphical applications, without unrelated std CLI tools.
(root/"mk/25-chat.mk").write_text("""\
.PHONY: nonos-mk-chat-desktop-prod
nonos-mk-chat-desktop-prod: $(DESKTOP_BASE_CAPSULE_ARTIFACTS) $(snake_ARTIFACTS) $(ZK_POLICY_ROOT) $(foreach s,$(NONOS_ENROLLED_CAPSULES),$($(s)_VERIFY)) nonos-mk-check-deps nonos-mk-ensure-signing-key
\t$(call nonos_kernel_build,microkernel-desktop-base + nonos-stark-attest,microkernel-desktop-base$(_boot_comma)nonos-stark-attest)
""")

subprocess.run([sys.executable,str(repo/"scripts/virtio-compat.py"),str(root)],check=True)
