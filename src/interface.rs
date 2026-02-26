//! Interface traits and implementations

mod spi;
pub use spi::*;

mod parallel;
pub use parallel::*;

/// Command and pixel interface
pub trait Interface {
    /// The native width of the interface
    ///
    /// In most cases this will be u8, except for larger parallel interfaces such as
    /// 16 bit (currently supported)
    /// or 9 or 18 bit (currently unsupported)
    type Word: Copy;

    /// Error type
    type Error: core::fmt::Debug;

    /// Kind
    const KIND: InterfaceKind;

    /// Send a command with optional parameters
    fn send_command(
        &mut self,
        command: u8,
        args: &[u8],
    ) -> impl core::future::Future<Output = Result<(), Self::Error>>;

    /// Send a raw slice of data, typically pre-formatted pixel data.
    /// `WriteMemoryStart` (or equivalent) must be sent before calling this function.
    /// The data is assumed to be in the correct format for the display and interface.
    /// If Self::Word is u8, data is &[u8]. If Self::Word is u16, data is &[u16].
    fn send_data_slice(
        &mut self,
        data: &[Self::Word],
    ) -> impl core::future::Future<Output = Result<(), Self::Error>>;
}

impl<T: Interface + ?Sized> Interface for &mut T {
    type Word = T::Word;
    type Error = T::Error;
    const KIND: InterfaceKind = T::KIND;

    async fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        T::send_command(self, command, args).await
    }

    async fn send_data_slice(&mut self, data: &[Self::Word]) -> Result<(), Self::Error> {
        T::send_data_slice(self, data).await
    }
}

/// Interface kind.
///
/// Specifies the kind of physical connection to the display controller that is
/// supported by this interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum InterfaceKind {
    /// Serial interface with data/command pin.
    ///
    /// SPI style interface with 8 bits per word and an additional pin to
    /// distinguish between data and command words.
    Serial4Line,

    /// 8 bit parallel interface.
    ///
    /// 8080 style parallel interface with 8 data pins and chip select, write enable,
    /// and command/data signals.
    Parallel8Bit,

    /// 16 bit parallel interface.
    ///
    /// 8080 style parallel interface with 16 data pins and chip select, write enable,
    /// and command/data signals.
    Parallel16Bit,
}
