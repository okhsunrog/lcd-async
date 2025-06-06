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

/// GC9107 display in Rgb565 color mode.
pub struct GC9107;

impl Model for GC9107 {
    type ColorFormat = Rgb565;
    const FRAMEBUFFER_SIZE: (u16, u16) = (128, 160);

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

        delay.delay_ms(200).await;

        di.write_raw(0xFE, &[]).await?;
        delay.delay_ms(5).await;
        di.write_raw(0xEF, &[]).await?;
        delay.delay_ms(5).await;

        di.write_raw(0xB0, &[0xC0]).await?;
        di.write_raw(0xB2, &[0x2F]).await?;
        di.write_raw(0xB3, &[0x03]).await?;
        di.write_raw(0xB6, &[0x19]).await?;
        di.write_raw(0xB7, &[0x01]).await?;

        let madctl = SetAddressMode::from(options);
        di.write_command(madctl).await?;

        di.write_raw(0xAC, &[0xCB]).await?;
        di.write_raw(0xAB, &[0x0E]).await?;

        di.write_raw(0xB4, &[0x04]).await?;

        di.write_raw(0xA8, &[0x19]).await?;

        let pf = PixelFormat::with_all(BitsPerPixel::from_rgb_color::<Self::ColorFormat>());
        di.write_command(SetPixelFormat::new(pf)).await?;

        di.write_raw(0xB8, &[0x08]).await?;

        di.write_raw(0xE8, &[0x24]).await?;

        di.write_raw(0xE9, &[0x48]).await?;

        di.write_raw(0xEA, &[0x22]).await?;

        di.write_raw(0xC6, &[0x30]).await?;
        di.write_raw(0xC7, &[0x18]).await?;

        di.write_raw(
            0xF0,
            &[
                0x01, 0x2b, 0x23, 0x3c, 0xb7, 0x12, 0x17, 0x60, 0x00, 0x06, 0x0c, 0x17, 0x12, 0x1f,
            ],
        )
        .await?;

        di.write_raw(
            0xF1,
            &[
                0x05, 0x2e, 0x2d, 0x44, 0xd6, 0x15, 0x17, 0xa0, 0x02, 0x0d, 0x0d, 0x1a, 0x18, 0x1f,
            ],
        )
        .await?;

        di.write_command(SetInvertMode::new(options.invert_colors))
            .await?;

        di.write_command(ExitSleepMode).await?; // turn off sleep
        delay.delay_ms(120).await;

        di.write_command(SetDisplayOn).await?; // turn on display

        Ok(madctl)
    }
}
