//! Filter bank API.

use core::marker::PhantomData;
use defmt::Format;

use crate::pac::can::RegisterBlock;
use crate::{ExtendedId, FilterOwner, Id, Instance, MasterInstance, StandardId};

/// A 16-bit filter list entry.
///
/// This can match data and remote frames using standard IDs.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Format)]
pub struct ListEntry16(u16);

/// A 32-bit filter list entry.
///
/// This can match data and remote frames using extended or standard IDs.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Format)]
pub struct ListEntry32(u32);

/// A 16-bit identifier mask.
#[derive(Debug, Copy, Clone, Format)]
pub struct Mask16 {
    id: u16,
    mask: u16,
}

/// A 32-bit identifier mask.
#[derive(Debug, Copy, Clone, Format)]
pub struct Mask32 {
    id: u32,
    mask: u32,
}

impl ListEntry16 {
    /// Creates a filter list entry that accepts data frames with the given standard ID.
    ///
    /// This entry will *not* accept remote frames with the same ID.
    pub fn data_frames_with_id(id: StandardId) -> Self {
        Self(id.as_raw() << 5)
    }

    /// Creates a filter list entry that accepts remote frames with the given standard ID.
    pub fn remote_frames_with_id(id: StandardId) -> Self {
        Self(id.as_raw() << 5 | 1 << 4)
    }
}

impl ListEntry32 {
    /// Creates a filter list entry that accepts data frames with the given ID.
    ///
    /// This entry will *not* accept remote frames with the same ID.
    ///
    /// The filter will only accept *either* standard *or* extended frames, depending on `id`.
    pub fn data_frames_with_id(id: impl Into<Id>) -> Self {
        match id.into() {
            Id::Standard(id) => Self(u32::from(id.as_raw()) << 21 | 0b000),
            Id::Extended(id) => Self(id.as_raw() << 3 | 0b100),
        }
    }

    /// Creates a filter list entry that accepts remote frames with the given ID.
    pub fn remote_frames_with_id(id: impl Into<Id>) -> Self {
        match id.into() {
            Id::Standard(id) => Self(u32::from(id.as_raw()) << 21 | 0b010),
            Id::Extended(id) => Self(id.as_raw() << 3 | 0b110),
        }
    }
}

impl Mask16 {
    /// Creates a 16-bit identifier mask that accepts all frames.
    ///
    /// This will accept both standard and extended data and remote frames with any ID.
    pub fn accept_all() -> Self {
        Self { id: 0, mask: 0 }
    }

    /// Creates a 16-bit identifier mask that accepts all standard frames with the given ID.
    ///
    /// Both data and remote frames with `id` will be accepted. Any extended frames will be
    /// rejected.
    pub fn frames_with_std_id(id: StandardId) -> Self {
        Self {
            id: id.as_raw() << 5,
            mask: 0x7ff << 5 | 0b1000, // also require IDE = 0
        }
    }
}

impl Mask32 {
    /// Creates a 32-bit identifier mask that accepts all frames.
    ///
    /// This will accept both standard and extended data and remote frames with any ID.
    pub fn accept_all() -> Self {
        Self { id: 0, mask: 0 }
    }

    /// Creates a 32-bit identifier mask that accepts all frames with the given extended ID.
    ///
    /// Both data and remote frames with `id` will be accepted. Standard frames will be rejected.
    pub fn frames_with_ext_id(id: ExtendedId) -> Self {
        Self {
            id: id.as_raw() << 3 | 0b100,
            mask: 0x1FFF_FFFF << 3 | 0b100, // also require IDE = 1
        }
    }

    /// Creates a 32-bit identifier mask that accepts standard and extended frames with the given
    /// ID.
    ///
    /// Both data and remote frames with `id` will be accepted.
    pub fn frames_with_std_id(id: StandardId) -> Self {
        Self {
            id: u32::from(id.as_raw()) << 21,
            mask: 0x1FFF_FFFF << 21 | 0b100, // also require IDE = 0
        }
    }
}

/// The configuration of a filter bank.
#[derive(Debug, Copy, Clone, Format)]
pub enum BankConfig {
    List16([ListEntry16; 4]),
    List32([ListEntry32; 2]),
    Mask16([Mask16; 2]),
    Mask32(Mask32),
}

/// Interface to the filter banks of a CAN peripheral.
pub struct MasterFilters<'a, I: FilterOwner> {
    /// Number of assigned filter banks.
    ///
    /// On chips with splittable filter banks, this value can be dynamic.
    bank_count: u8,
    _can: PhantomData<&'a mut I>,
}

// NOTE: This type mutably borrows the CAN instance and has unique access to the registers while it
// exists.
impl<I: FilterOwner> MasterFilters<'_, I> {
    pub(crate) unsafe fn new() -> Self {
        let can = &*I::REGISTERS;

        // Enable initialization mode.
        can.fmr.modify(|_, w| w.finit().set_bit());

        // Read the filter split value.
        let bank_count = can.fmr.read().can2sb().bits();

        // (Reset value of CAN2SB is 0x0E, 14, which, in devices with 14 filter banks, assigns all
        // of them to the master peripheral, and in devices with 28, assigns them 50/50 to
        // master/slave instances)

        Self {
            bank_count,
            _can: PhantomData,
        }
    }

    fn registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS }
    }

    fn banks_imm(&self) -> FilterBanks<'_> {
        FilterBanks {
            start_idx: 0,
            bank_count: self.bank_count,
            can: self.registers(),
        }
    }

    /// Returns the number of filter banks currently assigned to this instance.
    ///
    /// Chips with splittable filter banks may start out with some banks assigned to the master
    /// instance and some assigned to the slave instance.
    pub fn num_banks(&self) -> u8 {
        self.bank_count
    }

    /// Disables all enabled filter banks.
    ///
    /// This causes all incoming frames to be disposed.
    pub fn clear(&mut self) {
        self.banks_imm().clear();
    }

    /// Disables a filter bank.
    ///
    /// If `index` is out of bounds, this will panic.
    pub fn disable_bank(&mut self, index: u8) {
        self.banks_imm().disable(index);
    }

    /// Configures a filter bank according to `config` and enables it.
    pub fn enable_bank(&mut self, index: u8, config: BankConfig) {
        self.banks_imm().enable(index, config);
    }
}

impl<I: MasterInstance> MasterFilters<'_, I> {
    /// Sets the index at which the filter banks owned by the slave peripheral start.
    pub fn set_split(&mut self, split_index: u8) {
        assert!(split_index <= I::NUM_FILTER_BANKS);
        self.registers()
            .fmr
            .modify(|_, w| unsafe { w.can2sb().bits(split_index) });
        self.bank_count = split_index;
    }

    /// Accesses the filters assigned to the slave peripheral.
    pub fn slave_filters(&mut self) -> SlaveFilters<'_, I::Slave> {
        // NB: This mutably borrows `self`, so it has full access to the filter bank registers.
        SlaveFilters {
            start_idx: self.bank_count,
            bank_count: I::NUM_FILTER_BANKS - self.bank_count,
            _can: PhantomData,
        }
    }
}

impl<I: FilterOwner> Drop for MasterFilters<'_, I> {
    #[inline]
    fn drop(&mut self) {
        let can = self.registers();

        // Leave initialization mode.
        can.fmr.modify(|_, w| w.finit().clear_bit());
    }
}

/// Interface to the filter banks assigned to a slave peripheral.
pub struct SlaveFilters<'a, I: Instance> {
    start_idx: u8,
    bank_count: u8,
    _can: PhantomData<&'a mut I>,
}

impl<I: Instance> SlaveFilters<'_, I> {
    fn registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS }
    }

    fn banks_imm(&self) -> FilterBanks<'_> {
        FilterBanks {
            start_idx: self.start_idx,
            bank_count: self.bank_count,
            can: self.registers(),
        }
    }

    /// Returns the number of filter banks currently assigned to this instance.
    ///
    /// Chips with splittable filter banks may start out with some banks assigned to the master
    /// instance and some assigned to the slave instance.
    pub fn num_banks(&self) -> u8 {
        self.bank_count
    }

    /// Disables all enabled filter banks.
    ///
    /// This causes all incoming frames to be disposed.
    pub fn clear(&mut self) {
        self.banks_imm().clear();
    }

    /// Disables a filter bank.
    ///
    /// If `index` is out of bounds, this will panic.
    pub fn disable_bank(&mut self, index: u8) {
        self.banks_imm().disable(index);
    }

    /// Configures a filter bank according to `config` and enables it.
    pub fn enable_bank(&mut self, index: u8, config: BankConfig) {
        self.banks_imm().enable(index, config);
    }
}

struct FilterBanks<'a> {
    start_idx: u8,
    bank_count: u8,
    can: &'a RegisterBlock,
}

impl FilterBanks<'_> {
    fn clear(&mut self) {
        let mask = filter_bitmask(self.start_idx, self.bank_count);

        self.can.fa1r.modify(|r, w| {
            let bits = r.bits();
            // Clear all bits in `mask`.
            unsafe { w.bits(bits & !mask) }
        });
    }

    fn assert_bank_index(&self, index: u8) {
        assert!((self.start_idx..self.start_idx + self.bank_count).contains(&index));
    }

    fn disable(&mut self, index: u8) {
        self.assert_bank_index(index);

        self.can
            .fa1r
            .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << index)) })
    }

    fn enable(&mut self, index: u8, config: BankConfig) {
        self.assert_bank_index(index);

        // Configure mode.
        let mode = matches!(config, BankConfig::List16(_) | BankConfig::List32(_));
        self.can.fm1r.modify(|r, w| {
            let mut bits = r.bits();
            if mode {
                bits |= 1 << index;
            } else {
                bits &= !(1 << index);
            }
            unsafe { w.bits(bits) }
        });

        // Configure scale.
        let scale = matches!(config, BankConfig::List32(_) | BankConfig::Mask32(_));
        self.can.fs1r.modify(|r, w| {
            let mut bits = r.bits();
            if scale {
                bits |= 1 << index;
            } else {
                bits &= !(1 << index);
            }
            unsafe { w.bits(bits) }
        });

        // Configure filter register.
        let (fxr1, fxr2);
        match config {
            BankConfig::List16([a, b, c, d]) => {
                fxr1 = (u32::from(b.0) << 16) | u32::from(a.0);
                fxr2 = (u32::from(d.0) << 16) | u32::from(c.0);
            }
            BankConfig::List32([a, b]) => {
                fxr1 = a.0;
                fxr2 = b.0;
            }
            BankConfig::Mask16([a, b]) => {
                fxr1 = (u32::from(a.mask) << 16) | u32::from(a.id);
                fxr2 = (u32::from(b.mask) << 16) | u32::from(b.id);
            }
            BankConfig::Mask32(a) => {
                fxr1 = a.id;
                fxr2 = a.mask;
            }
        };
        let bank = &self.can.fb[usize::from(index)];
        bank.fr1.write(|w| unsafe { w.bits(fxr1) });
        bank.fr2.write(|w| unsafe { w.bits(fxr2) });

        // Set active.
        self.can
            .fa1r
            .modify(|r, w| unsafe { w.bits(r.bits() | (1 << index)) })
    }
}

/// Computes a bitmask for per-filter-bank registers that only includes filters in the given range.
fn filter_bitmask(start_idx: u8, bank_count: u8) -> u32 {
    let count_mask = (1 << bank_count) - 1; // `bank_count` 1-bits
    count_mask << start_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_bitmask() {
        assert_eq!(filter_bitmask(0, 1), 0x1);
        assert_eq!(filter_bitmask(1, 1), 0b10);
        assert_eq!(filter_bitmask(0, 4), 0xf);
        assert_eq!(filter_bitmask(1, 3), 0xe);
        assert_eq!(filter_bitmask(8, 1), 0x100);
        assert_eq!(filter_bitmask(8, 4), 0xf00);
    }
}
