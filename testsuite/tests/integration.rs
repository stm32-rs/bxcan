#![no_std]
#![no_main]

use nb::block;
use testsuite::{self, State};

#[defmt_test::tests]
mod tests {
    use bxcan::filter::{ListEntry32, Mask16, Mask32};
    use bxcan::{ExtendedId, Frame, StandardId};

    use super::*;

    #[init]
    fn init() -> State {
        State::init()
    }

    #[test]
    fn split_filters(state: &mut State) {
        let mut filt = state.can1.modify_filters();

        filt.set_split(0);
        defmt::assert_eq!(filt.num_banks(), 0);
        defmt::assert_eq!(filt.slave_filters().num_banks(), 28);

        filt.set_split(1);
        defmt::assert_eq!(filt.num_banks(), 1);
        defmt::assert_eq!(filt.slave_filters().num_banks(), 27);

        filt.set_split(13);
        defmt::assert_eq!(filt.num_banks(), 13);
        defmt::assert_eq!(filt.slave_filters().num_banks(), 15);

        filt.set_split(14);
        defmt::assert_eq!(filt.num_banks(), 14);
        defmt::assert_eq!(filt.slave_filters().num_banks(), 14);

        filt.set_split(28);
        defmt::assert_eq!(filt.num_banks(), 28);
        defmt::assert_eq!(filt.slave_filters().num_banks(), 0);
    }

    #[test]
    fn basic_roundtrip(state: &mut State) {
        state
            .can1
            .modify_filters()
            .clear()
            .enable_bank(0, Mask32::accept_all());

        let frame = Frame::new_data(StandardId::new(0).unwrap(), []);
        defmt::assert!(state.roundtrip_frame(&frame));

        let frame = Frame::new_data(ExtendedId::new(0xFFFF).unwrap(), [1, 2, 3, 4, 5]);
        defmt::assert!(state.roundtrip_frame(&frame));
    }

    #[test]
    fn no_filters_no_frames(state: &mut State) {
        state.can1.modify_filters().clear();

        let frame = Frame::new_data(ExtendedId::new(0).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));
        let frame = Frame::new_data(StandardId::new(0).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));
    }

    #[test]
    fn filter_mask32_std(state: &mut State) {
        let target_id = StandardId::new(42).unwrap();
        let mask = StandardId::MAX; // Exact match required

        state
            .can1
            .modify_filters()
            .clear()
            .enable_bank(0, Mask32::frames_with_std_id(target_id, mask));

        // Data frames with matching IDs should be accepted.
        let frame = Frame::new_data(target_id, []);
        defmt::assert!(state.roundtrip_frame(&frame));

        let frame = Frame::new_data(target_id, [1, 2, 3, 4, 5, 6, 7, 8]);
        defmt::assert!(state.roundtrip_frame(&frame));

        // ...remote frames with the same IDs should also be accepted.
        let frame = Frame::new_remote(target_id, 0);
        defmt::assert!(state.roundtrip_frame(&frame));

        let frame = Frame::new_remote(target_id, 7);
        defmt::assert!(state.roundtrip_frame(&frame));

        let frame = Frame::new_remote(target_id, 8);
        defmt::assert!(state.roundtrip_frame(&frame));

        // Different IDs should *not* be received.
        let frame = Frame::new_data(StandardId::new(1000).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));

        // Extended IDs that match the filter should be *rejected*.
        let frame = Frame::new_data(ExtendedId::new(target_id.as_raw().into()).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));

        // ...even when shifted upwards to match the standard ID bits.
        let frame = Frame::new_data(
            ExtendedId::new(u32::from(target_id.as_raw()) << 18).unwrap(),
            [],
        );
        defmt::assert!(!state.roundtrip_frame(&frame));
    }

    #[test]
    fn filter_mask32_ext(state: &mut State) {
        let target_id = ExtendedId::new(0).unwrap();
        let mask = ExtendedId::MAX; // Exact match required

        state
            .can1
            .modify_filters()
            .clear()
            .enable_bank(0, Mask32::frames_with_ext_id(target_id, mask));

        // Data frames with matching IDs should be accepted.
        let frame = Frame::new_data(target_id, []);
        defmt::assert!(state.roundtrip_frame(&frame));

        let frame = Frame::new_data(target_id, [1, 2, 3, 4, 5, 6, 7, 8]);
        defmt::assert!(state.roundtrip_frame(&frame));

        // ...remote frames with the same IDs should also be accepted.
        let frame = Frame::new_remote(target_id, 0);
        defmt::assert!(state.roundtrip_frame(&frame));

        let frame = Frame::new_remote(target_id, 7);
        defmt::assert!(state.roundtrip_frame(&frame));

        let frame = Frame::new_remote(target_id, 8);
        defmt::assert!(state.roundtrip_frame(&frame));

        // Different IDs should *not* be received.
        let frame = Frame::new_data(ExtendedId::new(1000).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));

        // Standard IDs should be *rejected* even if their value matches the filter mask.
        let frame = Frame::new_data(StandardId::new(0).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));

        // Different (standard) IDs should *not* be received.
        let frame = Frame::new_data(StandardId::MAX, []);
        defmt::assert!(!state.roundtrip_frame(&frame));
    }

    #[test]
    fn filter_mask16(state: &mut State) {
        let target_id_1 = StandardId::new(16).unwrap();
        let target_id_2 = StandardId::new(17).unwrap();
        let mask = StandardId::MAX; // Exact match required

        state.can1.modify_filters().clear().enable_bank(
            0,
            [
                Mask16::frames_with_std_id(target_id_1, mask),
                Mask16::frames_with_std_id(target_id_2, mask),
            ],
        );

        // Data frames with matching IDs should be accepted.
        let frame = Frame::new_data(target_id_1, []);
        defmt::assert!(state.roundtrip_frame(&frame));
        let frame = Frame::new_data(target_id_2, []);
        defmt::assert!(state.roundtrip_frame(&frame));

        // Incorrect IDs should be rejected.
        let frame = Frame::new_data(StandardId::new(15).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));
        let frame = Frame::new_data(StandardId::new(18).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));

        // Extended frames with the same ID are rejected, because the upper bits do not match.
        let frame = Frame::new_data(ExtendedId::new(16).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));
        let frame = Frame::new_data(ExtendedId::new(17).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));

        // Extended frames whose upper bits match the filter value are *still* rejected.
        let frame = Frame::new_data(ExtendedId::new(16 << 18).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));
        let frame = Frame::new_data(ExtendedId::new(17 << 18).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));
    }

    /// `List32` filter mode accepting standard CAN frames.
    #[test]
    fn filter_list32_std(state: &mut State) {
        let target_id_1 = StandardId::MAX;
        let target_id_2 = StandardId::new(42).unwrap();

        state.can1.modify_filters().clear().enable_bank(
            0,
            [
                ListEntry32::data_frames_with_id(target_id_1),
                ListEntry32::remote_frames_with_id(target_id_2),
            ],
        );

        // Frames with matching IDs should be accepted.
        let frame = Frame::new_data(target_id_1, []);
        defmt::assert!(state.roundtrip_frame(&frame));
        let frame = Frame::new_remote(target_id_2, 8);
        defmt::assert!(state.roundtrip_frame(&frame));

        // Date/Remote frame type must match.
        let frame = Frame::new_remote(target_id_1, 8);
        defmt::assert!(!state.roundtrip_frame(&frame));
        let frame = Frame::new_data(target_id_2, []);
        defmt::assert!(!state.roundtrip_frame(&frame));

        // Frames with matching, but *extended* IDs should be rejected.
        let frame = Frame::new_data(ExtendedId::new(target_id_1.as_raw().into()).unwrap(), []);
        defmt::assert!(!state.roundtrip_frame(&frame));
        let frame = Frame::new_remote(ExtendedId::new(target_id_2.as_raw().into()).unwrap(), 8);
        defmt::assert!(!state.roundtrip_frame(&frame));
    }

    /// `List32` filter mode accepting extended CAN frames.
    #[test]
    fn filter_list32_ext(state: &mut State) {
        let target_id_1 = ExtendedId::MAX;
        let target_id_2 = ExtendedId::new(42).unwrap();

        state.can1.modify_filters().clear().enable_bank(
            0,
            [
                ListEntry32::data_frames_with_id(target_id_1),
                ListEntry32::remote_frames_with_id(target_id_2),
            ],
        );

        // Frames with matching IDs should be accepted.
        let frame = Frame::new_data(target_id_1, []);
        defmt::assert!(state.roundtrip_frame(&frame));
        let frame = Frame::new_remote(target_id_2, 8);
        defmt::assert!(state.roundtrip_frame(&frame));

        // Date/Remote frame type must match.
        let frame = Frame::new_remote(target_id_1, 8);
        defmt::assert!(!state.roundtrip_frame(&frame));
        let frame = Frame::new_data(target_id_2, []);
        defmt::assert!(!state.roundtrip_frame(&frame));

        // Other IDs are rejected.
        let frame = Frame::new_remote(ExtendedId::new(43).unwrap(), 1);
        defmt::assert!(!state.roundtrip_frame(&frame));
        let frame = Frame::new_remote(ExtendedId::new(41).unwrap(), 1);
        defmt::assert!(!state.roundtrip_frame(&frame));

        // Matching standard IDs are rejected.
        let frame = Frame::new_remote(StandardId::new(42).unwrap(), 1);
        defmt::assert!(!state.roundtrip_frame(&frame));
    }

    /// Tests that a low-priority frame in a mailbox is aborted and returned when enqueuing a
    /// higher-priority frame while all mailboxes are full.
    #[test]
    fn dequeue_lower_priority_frame(state: &mut State) {
        state
            .can1
            .modify_filters()
            .clear()
            .enable_bank(0, Mask32::accept_all());

        defmt::assert!(state.can1.is_transmitter_idle());

        state.go_slow();

        // Enqueue several frames with increasing priorities.
        let frame4 = Frame::new_data(ExtendedId::new(4).unwrap(), []);
        defmt::assert!(state.can1.transmit(&frame4).unwrap().is_none());
        let frame3 = Frame::new_data(ExtendedId::new(3).unwrap(), []);
        defmt::assert!(state.can1.transmit(&frame3).unwrap().is_none());
        let frame2 = Frame::new_data(ExtendedId::new(2).unwrap(), []);
        defmt::assert!(state.can1.transmit(&frame2).unwrap().is_none());
        let frame1 = Frame::new_data(ExtendedId::new(1).unwrap(), []);
        defmt::assert!(state.can1.transmit(&frame1).unwrap().is_none());
        // NB: We need 4 frames, even though there are only 3 TX mailboxes, presumably because
        // `frame4` immediately enters some sort of transmit buffer, freeing its mailbox.

        // Now all mailboxes have a pending transmission request, but are still waiting on `frame4`
        // to finish transmission. Enqueuing a higher-priority frame should succeed and abort
        // transmission of a lower-priority frame.
        let frame0 = Frame::new_data(ExtendedId::new(0).unwrap(), []);
        let old = defmt::unwrap!(state.can1.transmit(&frame0).unwrap());

        // The returned frame should be the one with the lowest priority.
        defmt::assert_eq!(old, frame3);

        // All successfully transmitted frames should arrive in priority order, except `frame4`.
        defmt::assert_eq!(block!(state.can1.receive()).unwrap(), frame4);
        defmt::assert_eq!(block!(state.can1.receive()).unwrap(), frame0);
        defmt::assert_eq!(block!(state.can1.receive()).unwrap(), frame1);
        defmt::assert_eq!(block!(state.can1.receive()).unwrap(), frame2);

        // There should be no more data in transit.
        defmt::assert!(state.can1.is_transmitter_idle());
        defmt::assert!(matches!(state.can1.receive(), Err(nb::Error::WouldBlock)));

        state.go_fast();
    }

    /// Performs an external roundtrip from CAN1 to CAN2 and vice-versa.
    ///
    /// Requires that both are hooked up to the same CAN bus.
    #[test]
    fn ext_roundtrip(state: &mut State) {
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

        state
            .can1
            .modify_filters()
            .set_split(1)
            .clear()
            .enable_bank(0, Mask32::accept_all());

        state
            .can1
            .modify_filters()
            .slave_filters()
            .clear()
            .enable_bank(1, Mask32::accept_all());

        block!(state.can1.enable()).unwrap();
        block!(state.can2.enable()).unwrap();

        let frame = Frame::new_data(ExtendedId::new(123).unwrap(), [9, 8, 7]);
        block!(state.can2.transmit(&frame)).unwrap();

        while !state.can2.is_transmitter_idle() {}

        let received = state.can1.receive().unwrap();
        defmt::assert_eq!(frame, received);

        block!(state.can1.transmit(&frame)).unwrap();

        while !state.can1.is_transmitter_idle() {}

        let received = state.can2.receive().unwrap();
        defmt::assert_eq!(frame, received);
    }
}
