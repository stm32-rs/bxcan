//! Interrupt types.

use core::ops;

use defmt::Format;

/// bxCAN interrupt sources.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Format)]
#[non_exhaustive]
pub enum Interrupt {
    Sleep = 17,
    Wakeup = 16,
    Error = 15,
    Fifo1Overrun = 6,
    Fifo1Full = 5,
    Fifo1MessagePending = 4,
    Fifo0Overrun = 3,
    Fifo0Full = 2,
    Fifo0MessagePending = 1,
    TransmitMailboxEmpty = 0,
}

bitflags::bitflags! {
    /// A set of bxCAN interrupts.
    pub struct Interrupts: u32 {
        const SLEEP = 1 << 17;
        const WAKEUP = 1 << 16;
        const ERROR = 1 << 15;
        const FIFO1_OVERRUN = 1 << 6;
        const FIFO1_FULL = 1 << 5;
        const FIFO1_MESSAGE_PENDING = 1 << 4;
        const FIFO0_OVERRUN = 1 << 3;
        const FIFO0_FULL = 1 << 2;
        const FIFO0_MESSAGE_PENDING = 1 << 1;
        const TRANSMIT_MAILBOX_EMPTY = 1 << 0;
    }
}

impl From<Interrupt> for Interrupts {
    fn from(i: Interrupt) -> Self {
        Self::from_bits_truncate(i as u32)
    }
}

/// Adds an interrupts to the interrupt set.
impl ops::BitOrAssign<Interrupt> for Interrupts {
    fn bitor_assign(&mut self, rhs: Interrupt) {
        *self |= Self::from(rhs);
    }
}
