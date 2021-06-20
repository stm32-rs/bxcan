//! bxCAN on-device testsuite definitions.
//!
//! This is meant to run on an STM32F105, and will probably break on other chips.
//!
//! We deliberately avoid depending on any STM32 HAL here, since that can cause weird cyclic
//! dependencies once bxcan is used by them.

#![no_std]

pub mod interrupt;

use cortex_m::peripheral::NVIC;
use defmt_rtt as _;
use panic_probe as _;

use bxcan::{Can, FilterOwner, Frame, Instance, MasterInstance};

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

fn init(p: pac::Peripherals) -> (CAN1, CAN2) {
    // Enable CAN interrupts
    // Safety: `irq` is safe when all interrupts it manages are enabled.
    unsafe {
        NVIC::unmask(pac::Interrupt::USB_HP_CAN_TX);
        NVIC::unmask(pac::Interrupt::USB_LP_CAN_RX0);
        NVIC::unmask(pac::Interrupt::CAN_RX1);
        NVIC::unmask(pac::Interrupt::CAN_SCE);
        NVIC::unmask(pac::Interrupt::CAN2_TX);
        NVIC::unmask(pac::Interrupt::CAN2_RX0);
        NVIC::unmask(pac::Interrupt::CAN2_RX1);
        NVIC::unmask(pac::Interrupt::CAN2_SCE);
    }

    // Initialize CAN peripherals
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

pub struct State {
    pub can1: Can<CAN1>,
    pub can2: Can<CAN2>,
}

impl State {
    pub fn init() -> Self {
        let periph = defmt::unwrap!(pac::Peripherals::take());
        let (can1, can2) = init(periph);
        let mut can1 = Can::new(can1);
        let can2 = Can::new(can2);
        can1.modify_filters().clear();

        let mut state = Self { can1, can2 };
        state.go_fast();
        state
    }

    /// Configures the slowest possible speed.
    ///
    /// This is useful for testing recovery when the mailboxes are full.
    pub fn go_slow(&mut self) {
        self.can1
            .modify_config()
            .set_loopback(true)
            .set_silent(true)
            .set_bit_timing(0x007f_03ff)
            .enable();
        self.can2
            .modify_config()
            .set_loopback(true)
            .set_silent(true)
            .set_bit_timing(0x007f_03ff)
            .enable();
    }

    /// Configures the default (fast) speed.
    pub fn go_fast(&mut self) {
        self.can1
            .modify_config()
            .set_loopback(true)
            .set_silent(true)
            .set_bit_timing(0x00050000)
            .enable();
        self.can2
            .modify_config()
            .set_loopback(true)
            .set_silent(true)
            .set_bit_timing(0x00050000)
            .enable();
    }

    pub fn roundtrip_frame(&mut self, frame: &Frame) -> bool {
        nb::block!(self.can1.transmit(frame)).unwrap();
        defmt::assert!(!self.can1.is_transmitter_idle());

        // Wait until the transmission has completed.
        while !self.can1.is_transmitter_idle() {}

        match self.can1.receive() {
            Ok(received) => {
                defmt::assert_eq!(received, *frame);
                true
            }
            Err(nb::Error::WouldBlock) => false,
            Err(nb::Error::Other(e)) => defmt::panic!("{:?}", e),
        }
    }
}
