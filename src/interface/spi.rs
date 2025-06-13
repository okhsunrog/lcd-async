use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiDevice;

use super::{Interface, InterfaceKind};

#[derive(Clone, Copy, Debug)]
pub enum SpiError<SPI, DC> {
    Spi(SPI),
    Dc(DC),
}

pub struct SpiInterface<SPI, DC> {
    spi: SPI,
    dc: DC,
}

impl<SPI, DC> SpiInterface<SPI, DC>
where
    SPI: SpiDevice,
    DC: OutputPin,
{
    pub fn new(spi: SPI, dc: DC) -> Self {
        Self { spi, dc }
    }

    /// Release the DC pin and SPI peripheral back, deconstructing the interface
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

    async fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(SpiError::Dc)?;
        self.spi.write(&[command]).await.map_err(SpiError::Spi)?;
        self.dc.set_high().map_err(SpiError::Dc)?;
        self.spi.write(args).await.map_err(SpiError::Spi)?;
        Ok(())
    }

    async fn send_data_slice(&mut self, data: &[Self::Word]) -> Result<(), Self::Error> {
        // data is &[u8] because Self::Word = u8
        // Directly send the user's framebuffer slice.
        // The underlying SPI driver might do its own buffering/chunking if necessary.
        self.spi.write(data).await.map_err(SpiError::Spi)?;
        Ok(())
    }
}
