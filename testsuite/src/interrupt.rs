use core::cell::RefCell;

use crate::pac::interrupt;

irq::scoped_interrupts! {
    #[allow(non_camel_case_types)]
    pub enum Interrupt {
        USB_HP_CAN_TX,
        USB_LP_CAN_RX0,
        CAN_RX1,
        CAN_SCE,
        CAN2_TX,
        CAN2_RX0,
        CAN2_RX1,
        CAN2_SCE,
    }

    use #[interrupt];
}

pub use Interrupt::CAN_RX1 as CAN1_RX1;
pub use Interrupt::CAN_SCE as CAN1_SCE;
pub use Interrupt::USB_HP_CAN_TX as CAN1_TX;
pub use Interrupt::USB_LP_CAN_RX0 as CAN1_RX0;
pub use Interrupt::{CAN2_RX0, CAN2_RX1, CAN2_SCE, CAN2_TX};

pub struct Mutex<T> {
    object: cortex_m::interrupt::Mutex<RefCell<T>>,
}

impl<T> Mutex<T> {
    pub const fn new(object: T) -> Self {
        Self {
            object: cortex_m::interrupt::Mutex::new(RefCell::new(object)),
        }
    }

    pub fn lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        cortex_m::interrupt::free(|cs| f(&mut *self.object.borrow(cs).borrow_mut()))
    }
}
