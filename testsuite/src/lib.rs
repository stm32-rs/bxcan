//! bxCAN on-device testsuite definitions.
//!
//! This is meant to run on an STM32F103, aka a Blue Pill, and will probably break on other chips.
//!
//! We deliberately avoid depending on any STM32 HAL here, since that can cause weird cyclic
//! dependencies once bxcan is used by them.

#![no_std]

use defmt_rtt as _;
use panic_probe as _;

use bxcan::{FilterOwner, Instance};

pub use stm32f1::stm32f103 as pac;

pub struct CAN1 {
    _private: (),
}

pub struct CAN2 {
    _private: (),
}

unsafe impl Instance for CAN1 {
    const REGISTERS: *mut bxcan::RegisterBlock = 0x4000_6400 as *mut _;
}

// unsafe impl MasterInstance for CAN1 {}

unsafe impl FilterOwner for CAN1 {
    /// F103 is a medium-density device, which have 14 total filter banks.
    const NUM_FILTER_BANKS: u8 = 14;
}

/*unsafe impl Instance for CAN2 {
    const REGISTERS: *mut bxcan::RegisterBlock = 0x4000_6800 as *mut _;
}*/

pub fn init(can1: pac::CAN1, can2: pac::CAN2, rcc: &mut pac::RCC) -> (CAN1, CAN2) {
    // Turn on RCC clocks.
    rcc.apb1enr.modify(|_, w| w.canen().enabled());
    rcc.apb1rstr.modify(|_, w| w.canrst().reset());
    rcc.apb1rstr.modify(|_, w| w.canrst().clear_bit());

    let _ = (can1, can2);

    (CAN1 { _private: () }, CAN2 { _private: () })
}
