//! Display models.

use crate::{
    dcs::{self, InterfaceExt, SetAddressMode},
    interface::Interface,
    options::{self, ModelOptions, Rotation},
    ConfigurationError,
};
use embedded_graphics_core::prelude::RgbColor;
use embedded_hal_async::delay::DelayNs;

// existing model implementations
mod gc9107;
mod gc9a01;
mod ili9225;
mod ili9341;
mod ili9342c;
mod ili934x;
mod ili9486;
mod ili9488;
mod ili948x;
mod rm67162;
mod st7735s;
mod st7789;
mod st7796;

pub use gc9107::*;
pub use gc9a01::*;
pub use ili9225::*;
pub use ili9341::*;
pub use ili9342c::*;
pub use ili9486::*;
pub use ili9488::*;
pub use rm67162::*;
pub use st7735s::*;
pub use st7789::*;
pub use st7796::*;

/// Display model.
pub trait Model {
    /// The color format.
    type ColorFormat: RgbColor;

    /// The framebuffer size in pixels.
    const FRAMEBUFFER_SIZE: (u16, u16);

    /// Duration of the active low reset pulse in µs.
    const RESET_DURATION: u32 = 10;

    /// Initializes the display for this model with MADCTL from [crate::Display]
    /// and returns the value of MADCTL set by init
    fn init<DELAY, DI>(
        &mut self,
        di: &mut DI,
        delay: &mut DELAY,
        options: &ModelOptions,
    ) -> impl core::future::Future<Output = Result<SetAddressMode, ModelInitError<DI::Error>>>
    where
        DELAY: DelayNs,
        DI: Interface;

    /// Updates the address window of the display.
    fn update_address_window<DI>(
        di: &mut DI,
        _rotation: Rotation,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> impl core::future::Future<Output = Result<(), DI::Error>>
    where
        DI: Interface,
    {
        async move {
            di.write_command(dcs::SetColumnAddress::new(sx, ex)).await?;
            di.write_command(dcs::SetPageAddress::new(sy, ey)).await
        }
    }

    ///
    /// Need to call [Self::wake] before issuing other commands
    ///
    fn sleep<DI, DELAY>(
        di: &mut DI,
        delay: &mut DELAY,
    ) -> impl core::future::Future<Output = Result<(), DI::Error>>
    where
        DI: Interface,
        DELAY: DelayNs,
    {
        async move {
            di.write_command(dcs::EnterSleepMode).await?;
            // All supported models requires a 120ms delay before issuing other commands
            delay.delay_us(120_000).await;
            Ok(())
        }
    }
    ///
    /// Wakes the display after it's been set to sleep via [Self::sleep]
    ///
    fn wake<DI, DELAY>(
        di: &mut DI,
        delay: &mut DELAY,
    ) -> impl core::future::Future<Output = Result<(), DI::Error>>
    where
        DI: Interface,
        DELAY: DelayNs,
    {
        async move {
            di.write_command(dcs::ExitSleepMode).await?;
            // ST7789 and st7735s have the highest minimal delay of 120ms
            delay.delay_us(120_000).await;
            Ok(())
        }
    }
    ///
    /// We need WriteMemoryStart befor write pixel
    ///
    fn write_memory_start<DI>(
        di: &mut DI,
    ) -> impl core::future::Future<Output = Result<(), DI::Error>>
    where
        DI: Interface,
    {
        async move { di.write_command(dcs::WriteMemoryStart).await }
    }
    ///
    /// SoftReset
    ///
    fn software_reset<DI>(di: &mut DI) -> impl core::future::Future<Output = Result<(), DI::Error>>
    where
        DI: Interface,
    {
        async move { di.write_command(dcs::SoftReset).await }
    }
    ///
    /// This function will been called if user update options
    ///
    fn update_options<DI>(
        &self,
        di: &mut DI,
        options: &ModelOptions,
    ) -> impl core::future::Future<Output = Result<(), DI::Error>>
    where
        DI: Interface,
    {
        async move {
            let madctl = SetAddressMode::from(options);
            di.write_command(madctl).await
        }
    }

    ///
    /// Configures the tearing effect output.
    ///
    fn set_tearing_effect<DI>(
        di: &mut DI,
        tearing_effect: options::TearingEffect,
        _options: &ModelOptions,
    ) -> impl core::future::Future<Output = Result<(), DI::Error>>
    where
        DI: Interface,
    {
        async move {
            di.write_command(dcs::SetTearingEffect::new(tearing_effect))
                .await
        }
    }

    /// Sets the vertical scroll region.
    ///
    /// The `top_fixed_area` and `bottom_fixed_area` arguments can be used to
    /// define an area on the top and/or bottom of the display which won't be
    /// affected by scrolling.
    ///
    /// Note that this method is not affected by the current display orientation
    /// and will always scroll vertically relative to the default display
    /// orientation.
    ///
    /// The combined height of the fixed area must not larger than the
    /// height of the framebuffer height in the default orientation.
    ///
    /// After the scrolling region is defined the [`set_vertical_scroll_offset`](Self::set_vertical_scroll_offset) can be
    /// used to scroll the display.
    fn set_vertical_scroll_region<DI>(
        di: &mut DI,
        top_fixed_area: u16,
        bottom_fixed_area: u16,
    ) -> impl core::future::Future<Output = Result<(), DI::Error>>
    where
        DI: Interface,
    {
        async move {
            let rows = Self::FRAMEBUFFER_SIZE.1;

            let vscrdef = if top_fixed_area + bottom_fixed_area > rows {
                dcs::SetScrollArea::new(rows, 0, 0)
            } else {
                dcs::SetScrollArea::new(
                    top_fixed_area,
                    rows - top_fixed_area - bottom_fixed_area,
                    bottom_fixed_area,
                )
            };

            di.write_command(vscrdef).await
        }
    }

    /// Sets the vertical scroll offset.
    ///
    /// Setting the vertical scroll offset shifts the vertical scroll region
    /// upwards by `offset` pixels.
    ///
    /// Use [`set_vertical_scroll_region`](Self::set_vertical_scroll_region) to setup the scroll region, before
    /// using this method.
    fn set_vertical_scroll_offset<DI>(
        di: &mut DI,
        offset: u16,
    ) -> impl core::future::Future<Output = Result<(), DI::Error>>
    where
        DI: Interface,
    {
        async move {
            let vscad = dcs::SetScrollStart::new(offset);
            di.write_command(vscad).await
        }
    }
}

/// Error returned by [`Model::init`].
///
/// This error type is used internally by implementations of the [`Model`]
/// trait.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ModelInitError<DiError> {
    /// Error caused by the display interface.
    Interface(DiError),

    /// Invalid configuration error.
    ///
    /// This error is returned when the configuration passed to the builder is
    /// invalid. For example, when the combination of bit depth and interface
    /// kind isn't supported by the selected model.
    InvalidConfiguration(ConfigurationError),
}

impl<DiError> From<DiError> for ModelInitError<DiError> {
    fn from(value: DiError) -> Self {
        Self::Interface(value)
    }
}

#[cfg(test)]
mod tests {
    use embedded_graphics::pixelcolor::Rgb565;

    use crate::{
        Builder,
        _mock::{MockDelay, MockDisplayInterface},
        dcs::SetAddressMode,
        interface::InterfaceKind,
        ConfigurationError, InitError,
    };

    use super::*;

    struct OnlyOneKindModel(InterfaceKind);

    impl Model for OnlyOneKindModel {
        type ColorFormat = Rgb565;

        const FRAMEBUFFER_SIZE: (u16, u16) = (16, 16);

        async fn init<DELAY, DI>(
            &mut self,
            _di: &mut DI,
            _delay: &mut DELAY,
            _options: &ModelOptions,
        ) -> Result<SetAddressMode, ModelInitError<DI::Error>>
        where
            DELAY: DelayNs,
            DI: Interface,
        {
            if DI::KIND != self.0 {
                return Err(ModelInitError::InvalidConfiguration(
                    ConfigurationError::UnsupportedInterface,
                ));
            }

            Ok(SetAddressMode::default())
        }
    }

    #[test]
    fn test_assert_interface_kind_serial() {
        tokio_test::block_on(async {
            Builder::new(
                OnlyOneKindModel(InterfaceKind::Serial4Line),
                MockDisplayInterface,
            )
            .init(&mut MockDelay)
            .await
            .unwrap();
        });
    }

    #[test]
    fn test_assert_interface_kind_parallel() {
        tokio_test::block_on(async {
            assert!(matches!(
                Builder::new(
                    OnlyOneKindModel(InterfaceKind::Parallel8Bit),
                    MockDisplayInterface,
                )
                .init(&mut MockDelay)
                .await,
                Err(InitError::InvalidConfiguration(
                    ConfigurationError::UnsupportedInterface
                ))
            ));
        });
    }
}
