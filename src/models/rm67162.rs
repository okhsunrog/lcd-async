use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_hal_async::delay::DelayNs;

use crate::{
    dcs::{
        BitsPerPixel, ExitSleepMode, InterfaceExt, PixelFormat, SetAddressMode, SetDisplayOn,
        SetInvertMode, SetPixelFormat,
    },
    interface::{Interface, InterfaceKind},
    models::{Model, ModelInitError},
    options::ModelOptions,
    ConfigurationError,
};

/// RM67162 AMOLED display driver implementation
///
/// Supports:
/// - 16-bit RGB565 color
/// - 240x536 resolution
///
/// This driver was developed for the Lilygo T-Display-S3 AMOLED display (v2).
/// The initialization sequence is based on Lilygo's Arduino example code.
///
/// Currently only tested with 240x536 resolution displays.
/// While it may work with other display sizes, this is untested and could lead to unexpected behavior.
/// If you encounter issues with different display sizes, please report them.
///
pub struct RM67162;

impl Model for RM67162 {
    type ColorFormat = Rgb565;
    const FRAMEBUFFER_SIZE: (u16, u16) = (240, 536);

    async fn init<DELAY, DI>(
        &mut self,
        di: &mut DI,
        delay: &mut DELAY,
        options: &ModelOptions,
    ) -> Result<SetAddressMode, ModelInitError<DI::Error>>
    where
        DELAY: DelayNs,
        DI: Interface,
    {
        if !matches!(
            DI::KIND,
            InterfaceKind::Serial4Line | InterfaceKind::Parallel8Bit
        ) {
            return Err(ModelInitError::InvalidConfiguration(
                ConfigurationError::UnsupportedInterface,
            ));
        }

        let madctl = SetAddressMode::from(options);

        di.write_raw(0xFE, &[0x04]).await?;
        di.write_raw(0x6A, &[0x00]).await?;
        di.write_raw(0xFE, &[0x05]).await?;
        di.write_raw(0xFE, &[0x07]).await?;
        di.write_raw(0x07, &[0x4F]).await?;
        di.write_raw(0xFE, &[0x01]).await?;
        di.write_raw(0x2A, &[0x02]).await?;
        di.write_raw(0x2B, &[0x73]).await?;
        di.write_raw(0xFE, &[0x0A]).await?;
        di.write_raw(0x29, &[0x10]).await?;
        di.write_raw(0xFE, &[0x00]).await?;
        di.write_raw(0x51, &[0xaf]).await?; // Set brightness
        di.write_raw(0x53, &[0x20]).await?;
        di.write_raw(0x35, &[0x00]).await?;

        let pf = PixelFormat::with_all(BitsPerPixel::from_rgb_color::<Self::ColorFormat>());
        di.write_command(SetPixelFormat::new(pf)).await?;

        di.write_raw(0xC4, &[0x80]).await?; // enable SRAM access via SPI

        di.write_command(madctl).await?;

        di.write_command(SetInvertMode::new(options.invert_colors))
            .await?;

        di.write_command(ExitSleepMode).await?;
        delay.delay_us(120_000).await;

        di.write_command(SetDisplayOn).await?;

        Ok(madctl)
    }
}
