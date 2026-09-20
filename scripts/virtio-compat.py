#!/usr/bin/env python3
"""Support legacy VirtIO queue sizes up to 1024 without weakening device isolation."""
from pathlib import Path
import re, shutil, sys
root=Path(sys.argv[1]);repo=Path(__file__).resolve().parents[1]
def change(path,old,new):
 p=root/path;s=p.read_text()
 if new in s:return
 if old not in s:raise SystemExit(f"VirtIO patch anchor missing: {path}")
 p.write_text(s.replace(old,new))
blk=Path("userland/capsule_driver_virtio_blk/src")
net=Path("userland/capsule_driver_virtio_net/src")
change(blk/"constants/queue.rs","MAX_QUEUE_SIZE: u16 = 256","MAX_QUEUE_SIZE: u16 = 1024")
change(blk/"constants/queue.rs","VQ_REGION_SIZE: usize = 16384","VQ_REGION_SIZE: usize = 32768")
change(net/"constants/queue.rs","VQ_REGION_SIZE: usize = 12288","VQ_REGION_SIZE: usize = 32768")
shutil.copy2(repo/"compat/virtqueue_layout.rs",root/net/"queue/layout.rs")
change(net/"queue/mod.rs","mod clear;","mod clear;\npub(crate) mod layout;")
change(net/"init/program_queue.rs","queue_size_hint: u16,","_queue_size_hint: u16,")
change(net/"init/program_queue.rs","let qsize = core::cmp::min(qmax, queue_size_hint);","""if !crate::queue::layout::valid_queue_size(qmax) {
            regs.w8(LEG_STATUS, regs.r8(LEG_STATUS) | STATUS_FAILED);
            return Err("virtio-net: unsupported physical queue size");
        }
        let qsize = qmax;""")
for name,limit in (("rx_queue.rs","RX_QUEUE_SIZE"),("tx_queue.rs","TX_QUEUE_SIZE")):
 p=net/"queue"/name
 change(p,"    pub buf_count: u16,","    pub buf_count: u16,\n    pub ring_slots: u16,")
 change(p,"        buf_count: u16,","        ring_slots: u16,")
 change(p,"            buf_count,","            buf_count: ring_slots.min("+limit+"),\n            ring_slots,")
for name in ("post.rs","post_packet.rs"):
 p=root/net/"queue"/name;s=p.read_text()
 s=s.replace("RING_SLOTS, ","").replace("VQ_AVAIL_OFFSET, ","")
 s=s.replace(".add(VQ_AVAIL_OFFSET)",".add(super::layout::avail_offset(self.ring_slots))")
 s=s.replace("idx % RING_SLOTS","idx % self.ring_slots");p.write_text(s)
p=root/net/"queue/used.rs";s=p.read_text().replace("use crate::constants::VQ_USED_OFFSET;\n","")
s=s.replace("VQ_USED_OFFSET +","super::layout::used_offset(self.ring_slots) +");p.write_text(s)
p=root/net/"rx.rs";s=p.read_text().replace("RING_SLOTS, ","").replace("rx.last_used % RING_SLOTS","rx.last_used % rx.ring_slots");p.write_text(s)
print("VirtIO drivers support device-reported legacy queue sizes through 1024.")

# Remove the obsolete fixed 256-slot geometry and enforce allocation capacity.
p=root/net/"constants/queue.rs";s=p.read_text()
s=re.sub(r"pub const (?:VQ_AVAIL_OFFSET|VQ_USED_OFFSET|RING_SLOTS):[^;]+;\n","",s)
s=re.sub(r"// Physical virtqueue slot count\.[\s\S]*?// number of buffers the driver chooses to prime\.\n","// Ring geometry follows the device-reported size in queue/layout.rs.\n",s)
p.write_text(s)
p=root/net/"constants/mod.rs";s=p.read_text()
for name in ("VQ_AVAIL_OFFSET","VQ_USED_OFFSET","RING_SLOTS"):s=re.sub(r"\b"+name+r",\s*","",s)
p.write_text(s)
change(net/"queue/mod.rs","pub(crate) mod layout;","pub(crate) mod layout;\nconst _: () = assert!(crate::constants::VQ_REGION_SIZE == layout::REGION_SIZE);")
