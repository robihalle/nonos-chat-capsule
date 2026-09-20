// SPDX-License-Identifier: AGPL-3.0-or-later
#![no_std]
#![no_main]
extern crate alloc;
mod app;
mod keymap;
mod net;
mod protocol;
mod transport;
#[used]
#[link_section = ".nonos.caps"]
static CAPS: u64 = 0x183d;
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    nonos_app_skeleton::run(app::Chat::new)
}
