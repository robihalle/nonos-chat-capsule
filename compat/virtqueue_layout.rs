// SPDX-License-Identifier: AGPL-3.0-or-later
// Legacy VirtIO uses the device's full queue size, not the number of buffers posted.
pub const MAX_SLOTS: u16 = 1024;
pub const REGION_SIZE: usize = 32768;
pub const fn valid_queue_size(slots: u16) -> bool {
    slots != 0 && slots <= MAX_SLOTS && slots.is_power_of_two()
}
pub const fn avail_offset(slots: u16) -> usize {
    slots as usize * 16
}
pub const fn used_offset(slots: u16) -> usize {
    (avail_offset(slots) + 6 + slots as usize * 2 + 4095) & !4095
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_256_slot_layout() {
        assert_eq!(avail_offset(256), 4096);
        assert_eq!(used_offset(256), 8192);
    }
    #[test]
    fn qemu_1024_slot_layout() {
        assert_eq!(avail_offset(1024), 16384);
        assert_eq!(used_offset(1024), 20480);
    }
    #[test]
    fn every_supported_ring_fits_without_overlap() {
        for exponent in 0..=10 {
            let slots = 1u16 << exponent;
            assert!(valid_queue_size(slots));
            assert!(used_offset(slots) >= avail_offset(slots) + 6 + 2 * slots as usize);
            assert_eq!(used_offset(slots) % 4096, 0);
            assert!(used_offset(slots) + 6 + 8 * slots as usize <= REGION_SIZE);
        }
    }
    #[test]
    fn reject_invalid_device_sizes() {
        for slots in [0, 3, 255, 1023, 1025, 2048, u16::MAX] {
            assert!(!valid_queue_size(slots));
        }
    }
}
