//! bxCAN on-device testsuite definitions.
//!
//! This is meant to run on an STM32F105, and will probably break on other chips.
//!
//! We deliberately avoid depending on any STM32 HAL here, since that can cause weird cyclic
//! dependencies once bxcan is used by them.

#![no_std]

use defmt_rtt as _;
use panic_probe as _;

use bxcan::{FilterOwner, Instance, MasterInstance};

pub use stm32f1::stm32f107 as pac;

pub struct CAN1 {
    _private: (),
}

pub struct CAN2 {
    _private: (),
}

unsafe impl Instance for CAN1 {
    const REGISTERS: *mut bxcan::RegisterBlock = 0x4000_6400 as *mut _;
}

unsafe impl MasterInstance for CAN1 {}

unsafe impl FilterOwner for CAN1 {
    /// F105 is a connectivity-line device, which have 28 total filter banks.
    const NUM_FILTER_BANKS: u8 = 28;
}

unsafe impl Instance for CAN2 {
    const REGISTERS: *mut bxcan::RegisterBlock = 0x4000_6800 as *mut _;
}

pub fn init(p: pac::Peripherals) -> (CAN1, CAN2) {
    p.RCC
        .apb1enr
        .modify(|_, w| w.can1en().enabled().can2en().enabled());
    p.RCC
        .apb2enr
        .modify(|_, w| w.iopaen().enabled().iopben().enabled().afioen().enabled());

    p.RCC
        .apb1rstr
        .modify(|_, w| w.can1rst().reset().can2rst().reset());
    p.RCC
        .apb1rstr
        .modify(|_, w| w.can1rst().clear_bit().can2rst().clear_bit());

    // CAN1: PA11 + PA12
    // CAN2: PB5 + PB6
    p.AFIO
        .mapr
        .modify(|_, w| unsafe { w.can1_remap().bits(0).can2_remap().set_bit() });
    p.GPIOA
        .crh
        .modify(|_, w| w.mode12().output().cnf12().alt_push_pull());
    p.GPIOB
        .crl
        .modify(|_, w| w.mode6().output().cnf6().alt_push_pull());

    let _ = (p.CAN1, p.CAN2);

    (CAN1 { _private: () }, CAN2 { _private: () })
}
