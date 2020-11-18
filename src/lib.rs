//! Driver for the STM32 bxCAN peripheral.

#![doc(html_root_url = "https://docs.rs/bxcan/0.0.0")]
// Deny a few warnings in doctests, since rustdoc `allow`s many warnings by default
#![doc(test(attr(deny(unused_imports, unused_must_use))))]
#![no_std]

mod bb;
mod filter;
mod frame;
mod id;
mod pac;
mod readme;

pub use crate::filter::{Filter, Filters};
pub use crate::frame::Frame;
pub use crate::id::{ExtendedId, Id, StandardId};

use crate::pac::can::RegisterBlock;
use core::cmp::{Ord, Ordering};
use core::convert::{Infallible, TryInto};
use core::marker::PhantomData;

use frame::Data;

use self::pac::generic::*; // To make the PAC extraction build

/// A bxCAN peripheral instance.
///
/// This trait is meant to be implemented for a HAL-specific type that represent ownership of
/// the CAN peripheral (and any pins required by it, although that is entirely up to the HAL).
///
/// # Safety
///
/// It is only safe to implement this trait, when:
///
/// * The implementing type has ownership of the peripheral, preventing any other accesses to the
///   register block.
/// * `REGISTERS` is a pointer to that peripheral's register block and can be safely accessed for as
///   long as ownership or a borrow of the implementing type is present.
pub unsafe trait Instance {
    /// Pointer to the instance's register block.
    const REGISTERS: *mut RegisterBlock;
}

// TODO: what to do with these?
/*#[derive(Debug, Copy, Clone, Eq, PartialEq, Format)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Format)]
pub enum Error {
    Stuff,
    Form,
    Acknowledgement,
    BitRecessive,
    BitDominant,
    Crc,
    Software,
}*/

/// Identifier of a CAN message.
///
/// Can be either a standard identifier (11bit, Range: 0..0x3FF) or a
/// extendended identifier (29bit , Range: 0..0x1FFFFFFF).
///
/// The `Ord` trait can be used to determine the frame’s priority this ID
/// belongs to.
/// Lower identifier values have a higher priority. Additionally standard frames
/// have a higher priority than extended frames and data frames have a higher
/// priority than remote frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IdReg(u32);

impl IdReg {
    const STANDARD_SHIFT: u32 = 21;
    const STANDARD_MASK: u32 = 0x7FF << Self::STANDARD_SHIFT;

    const EXTENDED_SHIFT: u32 = 3;
    const EXTENDED_MASK: u32 = 0x1FFF_FFFF << Self::EXTENDED_SHIFT;

    const IDE_MASK: u32 = 0x0000_0004;

    const RTR_MASK: u32 = 0x0000_0002;

    /// Creates a new standard identifier (11bit, Range: 0..0x7FF)
    ///
    /// Panics for IDs outside the allowed range.
    fn new_standard(id: StandardId) -> Self {
        Self(u32::from(id.as_raw()) << Self::STANDARD_SHIFT)
    }

    /// Creates a new extendended identifier (29bit , Range: 0..0x1FFFFFFF).
    ///
    /// Panics for IDs outside the allowed range.
    fn new_extended(id: ExtendedId) -> IdReg {
        Self(id.as_raw() << Self::EXTENDED_SHIFT | Self::IDE_MASK)
    }

    fn from_register(reg: u32) -> IdReg {
        Self(reg & 0xFFFF_FFFE)
    }

    /// Sets the remote transmission (RTR) flag. This marks the identifier as
    /// being part of a remote frame.
    #[must_use = "returns a new IdReg without modifying `self`"]
    fn with_rtr(self, rtr: bool) -> IdReg {
        if rtr {
            Self(self.0 | Self::RTR_MASK)
        } else {
            Self(self.0 & !Self::RTR_MASK)
        }
    }

    /// Returns the identifier.
    fn to_id(self) -> Id {
        if self.is_extended() {
            Id::Extended(unsafe { ExtendedId::new_unchecked(self.0 >> Self::EXTENDED_SHIFT) })
        } else {
            Id::Standard(unsafe {
                StandardId::new_unchecked((self.0 >> Self::STANDARD_SHIFT) as u16)
            })
        }
    }

    /// Returns `true` if the identifier is an extended identifier.
    fn is_extended(self) -> bool {
        self.0 & Self::IDE_MASK != 0
    }

    /// Returns `true` if the identifier is a standard identifier.
    fn is_standard(self) -> bool {
        !self.is_extended()
    }

    /// Returns `true` if the identifer is part of a remote frame (RTR bit set).
    fn rtr(self) -> bool {
        self.0 & Self::RTR_MASK != 0
    }
}

impl Ord for IdReg {
    fn cmp(&self, other: &Self) -> Ordering {
        // When the IDs match, data frames have priority over remote frames.
        let rtr = self.rtr().cmp(&other.rtr()).reverse();

        let id_a = self.to_id();
        let id_b = other.to_id();
        match (id_a, id_b) {
            (Id::Standard(a), Id::Standard(b)) => {
                // Lower IDs have priority over higher IDs.
                a.as_raw().cmp(&b.as_raw()).reverse().then(rtr)
            }
            (Id::Extended(a), Id::Extended(b)) => a.as_raw().cmp(&b.as_raw()).reverse().then(rtr),
            (Id::Standard(a), Id::Extended(b)) => {
                // Standard frames have priority over extended frames if their Base IDs match.
                a.as_raw()
                    .cmp(&b.standard_id().as_raw())
                    .reverse()
                    .then(Ordering::Greater)
            }
            (Id::Extended(a), Id::Standard(b)) => a
                .standard_id()
                .as_raw()
                .cmp(&b.as_raw())
                .reverse()
                .then(Ordering::Less),
        }
    }
}

impl PartialOrd for IdReg {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Configuration proxy to be used with `Can::configure()`.
pub struct CanConfig<I> {
    _can: PhantomData<I>,
}

impl<I> CanConfig<I>
where
    I: Instance,
{
    fn registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS }
    }

    /// Configures the bit timings.
    ///
    /// Use http://www.bittiming.can-wiki.info/ to calculate the `btr` parameter.
    pub fn set_bit_timing(&mut self, btr: u32) {
        let can = self.registers();
        can.btr.modify(|r, w| unsafe {
            let mode_bits = r.bits() & 0xC000_0000;
            w.bits(mode_bits | btr)
        });
    }

    /// Enables or disables loopback mode: Internally connects the TX and RX
    /// signals together.
    pub fn set_loopback(&mut self, enabled: bool) {
        let can = self.registers();
        can.btr.modify(|_, w| w.lbkm().bit(enabled));
    }

    /// Enables or disables silent mode: Disconnects the TX signal from the pin.
    pub fn set_silent(&mut self, enabled: bool) {
        let can = self.registers();
        can.btr.modify(|_, w| w.silm().bit(enabled));
    }
}

/// Interface to the CAN peripheral.
pub struct Can<I: Instance> {
    _can: PhantomData<I>,
    tx: Option<Tx<I>>,
    rx: Option<Rx<I>>,
}

impl<I> Can<I>
where
    I: Instance,
{
    /// Creates a CAN interface.
    pub fn new() -> Can<I> {
        Self::new_internal()
    }

    fn registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS }
    }

    fn new_internal() -> Can<I> {
        Can {
            _can: PhantomData,
            tx: Some(Tx { _can: PhantomData }),
            rx: Some(Rx { _can: PhantomData }),
        }
    }

    /// Configure bit timings and silent/loop-back mode.
    ///
    /// Acutal configuration happens on the `CanConfig` that is passed to the
    /// closure. It must be done this way because those configuration bits can
    /// only be set if the CAN controller is in a special init mode.
    /// Puts the peripheral in sleep mode afterwards. `Can::enable()` must be
    /// called to exit sleep mode and start reception and transmission.
    pub fn configure<F>(&mut self, f: F)
    where
        F: FnOnce(&mut CanConfig<I>),
    {
        let can = self.registers();

        // Enter init mode.
        can.mcr
            .modify(|_, w| w.sleep().clear_bit().inrq().set_bit());
        while can.msr.read().inak().bit_is_clear() {}

        let mut config = CanConfig { _can: PhantomData };
        f(&mut config);

        // Leave init mode: go back to sleep.
        self.sleep();
    }

    /// Configures the automatic wake-up feature.
    pub fn set_automatic_wakeup(&mut self, enabled: bool) {
        let can = self.registers();
        can.mcr.modify(|_, w| w.awum().bit(enabled));
    }

    /// Start reception and transmission.
    ///
    /// Waits for 11 consecutive recessive bits to sync to the CAN bus.
    pub fn enable(&mut self) -> nb::Result<(), Infallible> {
        let can = self.registers();
        let msr = can.msr.read();
        if msr.slak().bit_is_set() {
            can.mcr
                .modify(|_, w| w.abom().set_bit().sleep().clear_bit());
            Err(nb::Error::WouldBlock)
        } else {
            Ok(())
        }
    }

    /// Puts the peripheral in a sleep mode to save power.
    ///
    /// Reception and transmission is disabled.
    pub fn sleep(&mut self) {
        let can = self.registers();
        can.mcr
            .modify(|_, w| w.sleep().set_bit().inrq().clear_bit());
        while can.msr.read().slak().bit_is_clear() {}
    }

    /// Enables the wake-up state change interrupt (CANn_SCE).
    ///
    /// Call `Can::enable()` in the ISR when the automatic wake-up is not enabled.
    pub fn enable_wakeup_interrupt(&mut self) {
        unsafe {
            let can = self.registers();
            bb::set(&can.ier, 16); // WKUIE
        }
    }

    /// Clears all state-change interrupt flags.
    pub fn clear_interrupt_flags(&mut self) {
        let can = self.registers();
        can.msr.write(|w| w.wkui().set_bit());
    }

    /// Returns the transmitter interface.
    ///
    /// Only the first calls returns a valid transmitter. Subsequent calls
    /// return `None`.
    pub fn take_tx(&mut self) -> Option<Tx<I>> {
        self.tx.take()
    }

    /// Returns the receiver interface.
    ///
    /// Takes ownership of filters which must be otained by `Can::split_filters()`.
    /// Only the first calls returns a valid receiver. Subsequent calls return `None`.
    pub fn take_rx(&mut self, _filters: Filters<I>) -> Option<Rx<I>> {
        self.rx.take()
    }
}

/// Interface to the CAN transmitter part.
pub struct Tx<I> {
    _can: PhantomData<I>,
}

const fn ok_mask(idx: usize) -> u32 {
    0x02 << (8 * idx)
}

const fn abort_mask(idx: usize) -> u32 {
    0x80 << (8 * idx)
}

impl<I> Tx<I>
where
    I: Instance,
{
    fn registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS }
    }

    /// Puts a CAN frame in a free transmit mailbox for transmission on the bus.
    ///
    /// Frames are transmitted to the bus based on their priority (identifier).
    /// Transmit order is preserved for frames with identical identifiers.
    /// If all transmit mailboxes are full, a higher priority frame replaces the
    /// lowest priority frame, which is returned as `Ok(Some(frame))`.
    pub fn transmit(&mut self, frame: &Frame) -> nb::Result<Option<Frame>, Infallible> {
        let can = self.registers();

        // Get the index of the next free mailbox or the one with the lowest priority.
        let tsr = can.tsr.read();
        let idx = tsr.code().bits() as usize;

        let frame_is_pending =
            tsr.tme0().bit_is_clear() || tsr.tme1().bit_is_clear() || tsr.tme2().bit_is_clear();
        let pending_frame = if frame_is_pending {
            // High priority frames are transmitted first by the mailbox system.
            // Frames with identical identifier shall be transmitted in FIFO order.
            // The controller schedules pending frames of same priority based on the
            // mailbox index instead. As a workaround check all pending mailboxes
            // and only accept higher priority frames.
            self.check_priority(0, frame.id)?;
            self.check_priority(1, frame.id)?;
            self.check_priority(2, frame.id)?;

            let all_frames_are_pending =
                tsr.tme0().bit_is_clear() && tsr.tme1().bit_is_clear() && tsr.tme2().bit_is_clear();
            if all_frames_are_pending {
                // No free mailbox is available. This can only happen when three frames with
                // descending priority were requested for transmission and all of them are
                // blocked by bus traffic with even higher priority.
                // To prevent a priority inversion abort and replace the lowest priority frame.
                self.read_pending_mailbox(idx)
            } else {
                // There was a free mailbox.
                None
            }
        } else {
            // All mailboxes are available: Send frame without performing any checks.
            None
        };

        self.write_mailbox(idx, frame);
        Ok(pending_frame)
    }

    /// Returns `Ok` when the mailbox is free or has a lower priority than
    /// identifier than `id`.
    fn check_priority(&self, idx: usize, id: IdReg) -> nb::Result<(), Infallible> {
        let can = self.registers();

        // Read the pending frame's id to check its priority.
        assert!(idx < 3);
        let tir = &can.tx[idx].tir.read();

        // Check the priority by comparing the identifiers. But first make sure the
        // frame has not finished transmission (`TXRQ` == 0) in the meantime.
        if tir.txrq().bit_is_set() && id >= IdReg::from_register(tir.bits()) {
            // There's a mailbox whose priority is higher or equal
            // the priority of the new frame.
            return Err(nb::Error::WouldBlock);
        }

        Ok(())
    }

    fn write_mailbox(&mut self, idx: usize, frame: &Frame) {
        let can = self.registers();

        debug_assert!(idx < 3);
        let mb = unsafe { &can.tx.get_unchecked(idx) };

        mb.tdtr
            .write(|w| unsafe { w.dlc().bits(frame.dlc() as u8) });
        mb.tdlr.write(|w| unsafe {
            w.bits(u32::from_ne_bytes(
                frame.data.bytes[0..4].try_into().unwrap(),
            ))
        });
        mb.tdhr.write(|w| unsafe {
            w.bits(u32::from_ne_bytes(
                frame.data.bytes[4..8].try_into().unwrap(),
            ))
        });
        mb.tir
            .write(|w| unsafe { w.bits(frame.id.0).txrq().set_bit() });
    }

    fn read_pending_mailbox(&mut self, idx: usize) -> Option<Frame> {
        if self.abort(idx) {
            let can = self.registers();
            debug_assert!(idx < 3);
            let mb = unsafe { &can.tx.get_unchecked(idx) };

            // Read back the pending frame.
            let mut pending_frame = Frame {
                id: IdReg(mb.tir.read().bits()),
                data: Data::empty(),
            };
            pending_frame.data[0..4].copy_from_slice(&mb.tdlr.read().bits().to_ne_bytes());
            pending_frame.data[4..8].copy_from_slice(&mb.tdhr.read().bits().to_ne_bytes());
            pending_frame.data.len = mb.tdtr.read().dlc().bits();

            Some(pending_frame)
        } else {
            // Abort request failed because the frame was already sent (or being sent) on
            // the bus. All mailboxes are now free. This can happen for small prescaler
            // values (e.g. 1MBit/s bit timing with a source clock of 8MHz) or when an ISR
            // has preemted the execution.
            None
        }
    }

    /// Tries to abort a pending frame. Returns `true` when aborted.
    fn abort(&mut self, idx: usize) -> bool {
        let can = self.registers();

        can.tsr.write(|w| unsafe { w.bits(abort_mask(idx)) });

        // Wait for the abort request to be finished.
        loop {
            let tsr = can.tsr.read().bits();
            if tsr & abort_mask(idx) == 0 {
                break tsr & ok_mask(idx) == 0;
            }
        }
    }

    /// Returns `true` if no frame is pending for transmission.
    pub fn is_idle(&self) -> bool {
        let can = self.registers();
        let tsr = can.tsr.read();
        tsr.tme0().bit_is_set() && tsr.tme1().bit_is_set() && tsr.tme2().bit_is_set()
    }

    /// Enables the transmit interrupt CANn_TX.
    ///
    /// The interrupt flags must be cleared with `Tx::clear_interrupt_flags()`.
    pub fn enable_interrupt(&mut self) {
        unsafe {
            let can = self.registers();
            bb::set(&can.ier, 0); // TMEIE
        }
    }

    /// Disables the transmit interrupt.
    pub fn disable_interrupt(&mut self) {
        unsafe {
            let can = self.registers();
            bb::clear(&can.ier, 0); // TMEIE
        }
    }

    /// Clears the request complete flag for all mailboxes.
    pub fn clear_interrupt_flags(&mut self) {
        let can = self.registers();
        can.tsr
            .write(|w| w.rqcp2().set_bit().rqcp1().set_bit().rqcp0().set_bit());
    }
}

/// Interface to the CAN receiver part.
pub struct Rx<I> {
    _can: PhantomData<I>,
}

impl<I> Rx<I>
where
    I: Instance,
{
    /// Returns a received frame if available.
    ///
    /// Returns `Err` when a frame was lost due to buffer overrun.
    pub fn receive(&mut self) -> nb::Result<Frame, ()> {
        match self.receive_fifo(0) {
            Err(nb::Error::WouldBlock) => self.receive_fifo(1),
            result => result,
        }
    }

    fn registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS }
    }

    fn receive_fifo(&mut self, fifo_nr: usize) -> nb::Result<Frame, ()> {
        let can = self.registers();

        assert!(fifo_nr < 2);
        let rfr = &can.rfr[fifo_nr];
        let rx = &can.rx[fifo_nr];

        // Check if a frame is available in the mailbox.
        let rfr_read = rfr.read();
        if rfr_read.fmp().bits() == 0 {
            return Err(nb::Error::WouldBlock);
        }

        // Check for RX FIFO overrun.
        if rfr_read.fovr().bit_is_set() {
            rfr.write(|w| w.fovr().set_bit());
            return Err(nb::Error::Other(()));
        }

        // Read the frame.
        let mut frame = Frame {
            id: IdReg(rx.rir.read().bits()),
            data: Data::empty(),
        };
        frame.data[0..4].copy_from_slice(&rx.rdlr.read().bits().to_ne_bytes());
        frame.data[4..8].copy_from_slice(&rx.rdhr.read().bits().to_ne_bytes());
        frame.data.len = rx.rdtr.read().dlc().bits();

        // Release the mailbox.
        rfr.write(|w| w.rfom().set_bit());

        Ok(frame)
    }

    /// Enables the receive interrupts CANn_RX0 and CANn_RX1.
    ///
    /// Make sure to register interrupt handlers for both.
    /// The interrupt flags are cleared by reading frames with `Rx::receive()`.
    pub fn enable_interrupts(&mut self) {
        unsafe {
            let can = self.registers();
            bb::set(&can.ier, 1); // FMPIE0
            bb::set(&can.ier, 4); // FMPIE1
        }
    }

    /// Disables the receive interrupts.
    pub fn disable_interrupts(&mut self) {
        unsafe {
            let can = self.registers();
            bb::clear(&can.ier, 1); // FMPIE0
            bb::clear(&can.ier, 4); // FMPIE1
        }
    }
}
