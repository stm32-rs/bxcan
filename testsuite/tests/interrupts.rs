#![no_std]
#![no_main]

#[defmt_test::tests]
mod tests {
    use core::sync::atomic::{AtomicBool, Ordering};

    use bxcan::{filter::Mask32, Interrupts, Mailbox, StandardId};
    use bxcan::{Frame, Interrupt};

    use irq::handler;
    use nb::block;
    use testsuite::{
        interrupt::{self, Mutex},
        State,
    };

    #[init]
    fn init() -> State {
        let mut state = State::init();

        // Accept all messages.
        state
            .can1
            .modify_filters()
            .set_split(1)
            .clear()
            .enable_bank(0, Mask32::accept_all())
            .slave_filters()
            .clear()
            .enable_bank(1, Mask32::accept_all());

        state
    }

    #[test]
    fn tx_interrupt(state: &mut State) {
        state.can1.enable_interrupt(Interrupt::TransmitMailboxEmpty);

        let m = Mutex::new(&mut *state);
        let tx_fired = AtomicBool::new(false);
        handler!(
            can1_tx = || {
                defmt::debug!("CAN1 TX interrupt");
                defmt::assert_eq!(
                    m.lock(|state| state.can1.clear_request_completed_flag()),
                    Some(Mailbox::Mailbox0)
                );
                defmt::assert_eq!(
                    m.lock(|state| state.can1.clear_request_completed_flag()),
                    None
                );
                tx_fired.store(true, Ordering::Relaxed);
            }
        );
        irq::scope(|scope| {
            scope.register(interrupt::CAN1_TX, can1_tx);

            defmt::assert!(!tx_fired.load(Ordering::Relaxed));
            let frame = Frame::new_data(StandardId::new(0).unwrap(), []);
            defmt::assert!(m.lock(|state| state.roundtrip_frame(&frame)));
            defmt::assert!(tx_fired.load(Ordering::Relaxed));
        });

        state
            .can1
            .disable_interrupt(Interrupt::TransmitMailboxEmpty);
    }

    #[test]
    fn rx_interrupt_message_pending(state: &mut State) {
        state.can1.enable_interrupt(Interrupt::Fifo0MessagePending);

        let m = Mutex::new(&mut *state);
        let interrupt_fired = AtomicBool::new(false);
        handler!(
            can1_rx = || {
                defmt::debug!("interrupt: FIFO 0 message pending");
                let frame = m.lock(|state| state.can1.receive().unwrap());
                defmt::debug!("received {:?}", frame);

                interrupt_fired.store(true, Ordering::Relaxed);
            }
        );
        irq::scope(|scope| {
            scope.register(interrupt::CAN1_RX0, can1_rx);

            let frame = Frame::new_data(StandardId::new(0).unwrap(), []);
            defmt::debug!("transmitting {:?}", frame);
            defmt::assert!(!interrupt_fired.load(Ordering::Relaxed));
            defmt::unwrap!(block!(m.lock(|state| state.can1.transmit(&frame))));

            m.lock(|state|
                // Wait until the transmission has completed.
                while !state.can1.is_transmitter_idle() {}
            );

            defmt::assert!(interrupt_fired.load(Ordering::Relaxed));
        });

        state.can1.disable_interrupt(Interrupt::Fifo0MessagePending);
    }

    #[test]
    fn rx_interrupt_fifo_full(state: &mut State) {
        state.can1.enable_interrupt(Interrupt::Fifo0Full);

        let m = Mutex::new(&mut *state);
        let interrupt_fired = AtomicBool::new(false);
        handler!(
            can1_rx = || {
                defmt::debug!("interrupt: FIFO 0 is full");
                let frame = m.lock(|state| state.can1.receive().unwrap());
                defmt::debug!("received {:?}", frame);
                let frame = m.lock(|state| state.can1.receive().unwrap());
                defmt::debug!("received {:?}", frame);
                let frame = m.lock(|state| state.can1.receive().unwrap());
                defmt::debug!("received {:?}", frame);

                interrupt_fired.store(true, Ordering::Relaxed);
            }
        );
        irq::scope(|scope| {
            scope.register(interrupt::CAN1_RX0, can1_rx);

            let frame = Frame::new_data(StandardId::new(0).unwrap(), []);
            defmt::debug!("transmitting {:?} 3 times", frame);
            defmt::unwrap!(block!(m.lock(|state| state.can1.transmit(&frame))));
            defmt::unwrap!(block!(m.lock(|state| state.can1.transmit(&frame))));
            defmt::assert!(!interrupt_fired.load(Ordering::Relaxed));
            defmt::unwrap!(block!(m.lock(|state| state.can1.transmit(&frame))));

            m.lock(|state|
                // Wait until all transmissions have completed.
                while !state.can1.is_transmitter_idle() {}
            );

            defmt::assert!(interrupt_fired.load(Ordering::Relaxed));
        });

        state.can1.disable_interrupt(Interrupt::Fifo0Full);
    }

    #[test]
    fn rx_interrupt_fifo_overrun(state: &mut State) {
        state.can1.enable_interrupt(Interrupt::Fifo0Overrun);

        let m = Mutex::new(&mut *state);
        let interrupt_fired = AtomicBool::new(false);
        handler!(
            can1_rx = || {
                defmt::debug!("interrupt: FIFO 0 overrun");
                m.lock(|state| state.can1.receive().unwrap_err());

                interrupt_fired.store(true, Ordering::Relaxed);
            }
        );
        irq::scope(|scope| {
            scope.register(interrupt::CAN1_RX0, can1_rx);

            let frame = Frame::new_data(StandardId::new(0).unwrap(), []);
            defmt::debug!("transmitting {:?} 4 times", frame);
            defmt::unwrap!(block!(m.lock(|state| state.can1.transmit(&frame))));
            defmt::unwrap!(block!(m.lock(|state| state.can1.transmit(&frame))));
            defmt::unwrap!(block!(m.lock(|state| state.can1.transmit(&frame))));

            m.lock(|state|
                // Wait until all transmissions have completed.
                while !state.can1.is_transmitter_idle() {}
            );

            // FIFO should be full, but not have overrun.
            defmt::assert!(!interrupt_fired.load(Ordering::Relaxed));

            defmt::unwrap!(block!(m.lock(|state| state.can1.transmit(&frame))));

            m.lock(|state|
                // Wait until all transmissions have completed.
                while !state.can1.is_transmitter_idle() {}
            );

            // Reception of the 4th message should have caused an overrun interrupt.
            defmt::assert!(interrupt_fired.load(Ordering::Relaxed));
        });

        state.can1.disable_interrupt(Interrupt::Fifo0Overrun);
    }

    #[test]
    fn sce_interrupt_sleep(state: &mut State) {
        state.can1.enable_interrupts(Interrupts::SLEEP);

        let m = Mutex::new(&mut *state);
        let sleep_interrupt_fired = AtomicBool::new(false);
        handler!(
            on_sleep = || {
                defmt::debug!("interrupt: entered sleep mode");
                m.lock(|state| state.can1.clear_sleep_interrupt());
                sleep_interrupt_fired.store(true, Ordering::Relaxed);
            }
        );
        irq::scope(|scope| {
            scope.register(interrupt::CAN1_SCE, on_sleep);

            defmt::assert!(!sleep_interrupt_fired.load(Ordering::Relaxed));
            m.lock(|state| state.can1.sleep());
            defmt::assert!(sleep_interrupt_fired.load(Ordering::Relaxed));
        });

        state.can1.disable_interrupts(Interrupts::SLEEP);
    }

    #[test]
    fn sce_interrupt_wakeup(state: &mut State) {
        // The wakeup interrupt does not fire when calling `can.wakeup()`, it requires an incoming
        // message. This test uses CAN2 to send that message.

        state.can1.enable_interrupt(Interrupt::Wakeup);

        // Turn off the loopback modes.
        state
            .can1
            .modify_config()
            .set_loopback(false)
            .set_silent(false)
            .set_bit_timing(0x00050000);
        state
            .can2
            .modify_config()
            .set_loopback(false)
            .set_silent(false)
            .set_bit_timing(0x00050000);
        block!(state.can1.enable()).unwrap();
        block!(state.can2.enable()).unwrap();

        let m = Mutex::new(&mut *state);
        let wakeup_interrupt_fired = AtomicBool::new(false);
        handler!(
            on_wakeup = || {
                defmt::debug!("interrupt: left sleep mode");
                m.lock(|state| state.can1.clear_wakeup_interrupt());
                wakeup_interrupt_fired.store(true, Ordering::Relaxed);
            }
        );
        irq::scope(|scope| {
            scope.register(interrupt::CAN1_SCE, on_wakeup);

            m.lock(|state| {
                state.can1.set_automatic_wakeup(true);
                state.can1.sleep();
            });
            let frame = Frame::new_data(StandardId::new(0).unwrap(), []);
            defmt::unwrap!(block!(m.lock(|state| state.can2.transmit(&frame))));
            m.lock(|state|
                // Wait until all transmissions have completed.
                while !state.can2.is_transmitter_idle() {}
            );
            defmt::assert!(wakeup_interrupt_fired.load(Ordering::Relaxed));
        });

        state.can1.disable_interrupt(Interrupt::Wakeup);
        state.go_fast();
    }
}
