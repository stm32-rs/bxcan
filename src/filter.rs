use core::marker::PhantomData;

use crate::pac::can::RegisterBlock;
use crate::{bb, Id, IdReg, Instance};

/// Filter with an optional mask.
pub struct Filter {
    id: u32,
    mask: u32,
}

impl Filter {
    /// Creates a filter that accepts all messages.
    pub fn accept_all() -> Self {
        Self { id: 0, mask: 0 }
    }

    /// Creates a filter that accepts frames with the specified identifier.
    pub fn new(id: Id) -> Self {
        match id {
            Id::Standard(id) => Filter {
                id: u32::from(id.as_raw()) << IdReg::STANDARD_SHIFT,
                mask: IdReg::STANDARD_MASK | IdReg::IDE_MASK | IdReg::RTR_MASK,
            },
            Id::Extended(id) => Filter {
                id: id.as_raw() << IdReg::EXTENDED_SHIFT | IdReg::IDE_MASK,
                mask: IdReg::EXTENDED_MASK | IdReg::IDE_MASK | IdReg::RTR_MASK,
            },
        }
    }

    /// Only look at the bits of the indentifier which are set to 1 in the mask.
    ///
    /// A mask of 0 accepts all identifiers.
    pub fn with_mask(&mut self, mask: u32) -> &mut Self {
        if self.is_extended() {
            self.mask = (self.mask & !IdReg::EXTENDED_MASK) | (mask << IdReg::EXTENDED_SHIFT);
        } else {
            self.mask = (self.mask & !IdReg::STANDARD_MASK) | (mask << IdReg::STANDARD_SHIFT);
        }
        self
    }

    /// Makes this filter accept both data and remote frames.
    pub fn allow_remote(&mut self) -> &mut Self {
        self.mask &= !IdReg::RTR_MASK;
        self
    }

    /// Makes this filter accept only remote frames.
    pub fn only_remote(&mut self) -> &mut Self {
        self.id |= IdReg::RTR_MASK;
        self.mask |= IdReg::RTR_MASK;
        self
    }

    fn is_extended(&self) -> bool {
        self.id & IdReg::IDE_MASK != 0
    }

    fn matches_single_id(&self) -> bool {
        ((self.mask & (IdReg::IDE_MASK | IdReg::RTR_MASK)) == (IdReg::IDE_MASK | IdReg::RTR_MASK))
            && if self.is_extended() {
                (self.mask & IdReg::EXTENDED_MASK) == IdReg::EXTENDED_MASK
            } else {
                (self.mask & IdReg::STANDARD_MASK) == IdReg::STANDARD_MASK
            }
    }

    fn reg_to_16bit(reg: u32) -> u32 {
        (reg & IdReg::STANDARD_MASK) >> 16
            | (reg & IdReg::IDE_MASK) << 1
            | (reg & IdReg::RTR_MASK) << 3
    }

    fn id_to_16bit(&self) -> u32 {
        Self::reg_to_16bit(self.id)
    }

    fn mask_to_16bit(&self) -> u32 {
        Self::reg_to_16bit(self.mask)
    }
}

/// Interface to the filter banks of a CAN peripheral.
pub struct Filters<I> {
    start_idx: usize,
    stop_idx: usize,
    count: usize,
    _can: PhantomData<I>,
}

impl<I> Filters<I>
where
    I: Instance,
{
    pub unsafe fn new(start_idx: usize, stop_idx: usize) -> Self {
        Self {
            start_idx,
            stop_idx,
            count: 0,
            _can: PhantomData,
        }
    }

    fn registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS }
    }

    /// Returns the number of available filters.
    ///
    /// This can number can be larger than the number of filter banks if
    /// `Can::split_filters_advanced()` was used.
    pub fn num_available(&self) -> usize {
        let can = self.registers();

        let mut filter_count = self.stop_idx - self.start_idx;

        let owned_bits = ((1 << filter_count) - 1) << self.start_idx;
        let mode_list = can.fm1r.read().bits() & owned_bits;
        let scale_16bit = !can.fs1r.read().bits() & owned_bits;

        filter_count += mode_list.count_ones() as usize;
        filter_count += scale_16bit.count_ones() as usize;
        filter_count += (mode_list & scale_16bit).count_ones() as usize;
        filter_count
    }

    /// Adds a filter. Returns `Err` if the maximum number of filters was reached.
    pub fn add(&mut self, filter: &Filter) -> Result<(), ()> {
        let can = self.registers();

        let idx = self.start_idx + self.count;
        if idx >= self.stop_idx {
            return Err(());
        }

        let mode_list = (can.fm1r.read().bits() & (1 << idx)) != 0;
        let scale_16bit = (can.fs1r.read().bits() & (1 << idx)) == 0;
        let bank_enabled = (can.fa1r.read().bits() & (1 << idx)) != 0;

        // Make sure the filter is supported by the filter bank configuration.
        if (mode_list && !filter.matches_single_id()) || (scale_16bit && filter.is_extended()) {
            return Err(());
        }

        // Disable the filter bank so it can be modified.
        unsafe { bb::clear(&can.fa1r, idx as u8) };

        let filter_bank = &can.fb[idx];
        let fr1 = filter_bank.fr1.read().bits();
        let fr2 = filter_bank.fr2.read().bits();
        let (fr1, fr2) = match (mode_list, scale_16bit, bank_enabled) {
            // 29bit id + mask
            (false, false, _) => {
                self.count += 1;
                (filter.id, filter.mask)
            }
            // 2x 29bit id
            (true, false, false) => (filter.id, filter.id),
            (true, false, true) => {
                self.count += 1;
                (fr1, filter.id)
            }
            // 2x 11bit id + mask
            (false, true, false) => (
                filter.mask_to_16bit() << 16 | filter.id_to_16bit(),
                filter.mask_to_16bit() << 16 | filter.id_to_16bit(),
            ),
            (false, true, true) => {
                self.count += 1;
                (fr1, filter.mask_to_16bit() << 16 | filter.id_to_16bit())
            }
            // 4x 11bit id
            (true, true, false) => (
                filter.id_to_16bit() << 16 | filter.id_to_16bit(),
                filter.id_to_16bit() << 16 | filter.id_to_16bit(),
            ),
            (true, true, true) => {
                let f = [fr1 & 0xFFFF, fr1 >> 16, fr2 & 0xFFFF, fr2 >> 16];

                if f[0] == f[1] {
                    // One filter available, add the second.
                    (filter.id_to_16bit() << 16 | f[0], fr2)
                } else if f[0] == f[2] {
                    // Two filters available, add the third.
                    (fr1, f[0] << 16 | filter.id_to_16bit())
                } else if f[0] == f[3] {
                    // Three filters available, add the fourth.
                    self.count += 1;
                    (fr1, filter.id_to_16bit() << 16 | f[2])
                } else {
                    unreachable!()
                }
            }
        };

        let can = self.registers();
        let filter_bank = &can.fb[idx];
        filter_bank.fr1.write(|w| unsafe { w.bits(fr1) });
        filter_bank.fr2.write(|w| unsafe { w.bits(fr2) });
        unsafe { bb::set(&can.fa1r, idx as u8) }; // Enable the filter bank
        Ok(())
    }

    /// Disables all enabled filter banks.
    pub fn clear(&mut self) {
        let can = self.registers();

        assert!(self.start_idx + self.count <= self.stop_idx);
        for i in self.start_idx..(self.start_idx + self.count) {
            // Bitbanding required because the filters are shared between CAN1 and CAN2
            unsafe { bb::clear(&can.fa1r, i as u8) };
        }
        self.count = 0;
    }
}
