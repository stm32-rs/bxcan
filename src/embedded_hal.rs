//! `embedded_hal` trait impls.

use crate::{Can, Data, ExtendedId, Frame, Id, Instance, OverrunError, StandardId};

use embedded_can as can;

impl<I> can::nb::Can for Can<I>
where
    I: Instance,
{
    type Frame = Frame;

    type Error = OverrunError;

    fn transmit(&mut self, frame: &Self::Frame) -> nb::Result<Option<Self::Frame>, Self::Error> {
        match self.transmit(frame) {
            Ok(status) => Ok(status.dequeued_frame().cloned()),
            Err(nb::Error::WouldBlock) => Err(nb::Error::WouldBlock),
            Err(nb::Error::Other(e)) => match e {},
        }
    }

    fn receive(&mut self) -> nb::Result<Self::Frame, Self::Error> {
        self.receive()
    }
}

impl can::Error for OverrunError {
    fn kind(&self) -> can::ErrorKind {
        can::ErrorKind::Overrun
    }
}

impl can::Frame for Frame {
    fn new(id: impl Into<can::Id>, data: &[u8]) -> Option<Self> {
        let id = match id.into() {
            can::Id::Standard(id) => unsafe {
                Id::Standard(StandardId::new_unchecked(id.as_raw()))
            },
            can::Id::Extended(id) => unsafe {
                Id::Extended(ExtendedId::new_unchecked(id.as_raw()))
            },
        };

        let data = Data::new(data)?;
        Some(Frame::new_data(id, data))
    }

    fn new_remote(id: impl Into<can::Id>, dlc: usize) -> Option<Self> {
        let id = match id.into() {
            can::Id::Standard(id) => unsafe {
                Id::Standard(StandardId::new_unchecked(id.as_raw()))
            },
            can::Id::Extended(id) => unsafe {
                Id::Extended(ExtendedId::new_unchecked(id.as_raw()))
            },
        };

        if dlc <= 8 {
            Some(Frame::new_remote(id, dlc as u8))
        } else {
            None
        }
    }

    #[inline]
    fn is_extended(&self) -> bool {
        self.is_extended()
    }

    #[inline]
    fn is_remote_frame(&self) -> bool {
        self.is_remote_frame()
    }

    #[inline]
    fn id(&self) -> can::Id {
        match self.id() {
            Id::Standard(id) => unsafe {
                can::Id::Standard(can::StandardId::new_unchecked(id.as_raw()))
            },
            Id::Extended(id) => unsafe {
                can::Id::Extended(can::ExtendedId::new_unchecked(id.as_raw()))
            },
        }
    }

    #[inline]
    fn dlc(&self) -> usize {
        self.dlc().into()
    }

    fn data(&self) -> &[u8] {
        if let Some(data) = self.data() {
            data
        } else {
            &[]
        }
    }
}
