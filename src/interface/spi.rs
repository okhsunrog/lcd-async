//!
//! Async SPI interface for MIPI DCS displays.
//!
//! This module provides an async implementation of the [`Interface`] trait for SPI-based TFT displays.
//! It is designed for use with async runtimes and drivers, and does not require an internal buffer—
//! pixel data is sent directly from the provided slice.
//!
//! # Example
//!
//! ```rust,ignore
//! use lcd_async::interface::SpiInterface;
//! use embedded_hal_async::spi::SpiDevice;
//! use embedded_hal::digital::OutputPin;
//!
//! let spi = /* your async SPI device */;
//! let dc = /* your DC OutputPin */;
//! let mut iface = SpiInterface::new(spi, dc);
//! // Use iface with the display driver
//! ```

use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiDevice;

use super::{Interface, InterfaceKind};

/// Error type for the async SPI interface.
///
/// Wraps errors from the SPI bus or the data/command (DC) pin.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SpiError<SPI, DC> {
    /// SPI bus error
    Spi(SPI),
    /// Data/command pin error
    Dc(DC),
}

/// Async SPI interface for MIPI DCS displays.
///
/// This struct implements the [`Interface`] trait for SPI-based displays, using an async [`SpiDevice`]
/// and a data/command (DC) output pin. Unlike the original mipidsi version, this async variant does not
/// use an internal buffer; all pixel data is sent directly from the provided slice.
///
/// Use [`SpiInterface::new`] to construct, and [`SpiInterface::release`] to deconstruct and recover the SPI and DC resources.
pub struct SpiInterface<SPI, DC> {
    spi: SPI,
    dc: DC,
}

impl<SPI, DC> SpiInterface<SPI, DC>
where
    SPI: SpiDevice,
    DC: OutputPin,
{
    /// Create a new async SPI interface from an SPI device and DC pin.
    pub fn new(spi: SPI, dc: DC) -> Self {
        Self { spi, dc }
    }

    /// Release the DC pin and SPI peripheral back, deconstructing the interface.
    pub fn release(self) -> (SPI, DC) {
        (self.spi, self.dc)
    }
}

impl<SPI, DC> Interface for SpiInterface<SPI, DC>
where
    SPI: SpiDevice, // Assuming async
    DC: OutputPin,  // Ensure OutputPin methods are compatible with your async context
{
    type Word = u8; // For SPI, Word is u8. send_data_slice will take &[u8]
    type Error = SpiError<SPI::Error, DC::Error>;

    const KIND: InterfaceKind = InterfaceKind::Serial4Line;

    /// Send a command and its arguments to the display controller.
    ///
    /// The DC pin is set low for the command byte, then high for the argument bytes.
    async fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(SpiError::Dc)?;
        self.spi.write(&[command]).await.map_err(SpiError::Spi)?;
        self.dc.set_high().map_err(SpiError::Dc)?;
        self.spi.write(args).await.map_err(SpiError::Spi)?;
        Ok(())
    }

    /// Send a slice of pixel or data bytes to the display controller.
    ///
    /// The data is sent as-is over SPI, with the DC pin assumed to be high.
    async fn send_data_slice(&mut self, data: &[Self::Word]) -> Result<(), Self::Error> {
        // data is &[u8] because Self::Word = u8
        // Directly send the user's framebuffer slice.
        // The underlying SPI driver might do its own buffering/chunking if necessary.
        self.spi.write(data).await.map_err(SpiError::Spi)?;
        Ok(())
    }
}
