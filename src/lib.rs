#![no_std]
// associated re-typing not supported in rust yet
#![allow(clippy::type_complexity)]
#![warn(missing_docs)]

//!
//! # lcd-async
//!
//! An **async-first** display driver for TFT displays implementing the [MIPI Display Command Set](https://www.mipi.org/specifications/display-command-set).
//!
//! This crate is a fork of [mipidsi](https://github.com/almindor/mipidsi), redesigned for modern, async-native embedded Rust. It features a framebuffer-centric workflow, enabling efficient, non-blocking rendering and seamless integration with async runtimes like [embassy](https://embassy.dev/).
//!
//! ## Key Features
//!
//! - **Fully Asynchronous:** All display I/O is performed via async traits, making it ideal for async runtimes and DMA-driven workflows.
//! - **Framebuffer-Based Drawing:** Draw your scene into an in-memory buffer using `embedded-graphics`, then send the entire frame to the display in one efficient async operation.
//! - **Separation of Concerns:** Drawing is synchronous and CPU-bound; sending to the display is async and I/O-bound. This enables double buffering and advanced rendering patterns.
//! - **Multiple Interface Support:**
//!   - SPI ([`interface::SpiInterface`])
//!   - 8080-style parallel via GPIO ([`interface::ParallelInterface`])
//!
//! ## Supported Models
//!
//! - GC9107
//! - GC9A01
//! - ILI9225
//! - ILI9341
//! - ILI9342C
//! - ILI9486
//! - ILI9488
//! - RM67162
//! - ST7735
//! - ST7789
//! - ST7796
//!
//! ## Example: Minimal Framebuffer Workflow
//!
//! ```rust
//! use embedded_graphics::prelude::*;
//! use embedded_graphics::pixelcolor::Rgb565;
//! use embedded_graphics::primitives::{Circle, PrimitiveStyle};
//! use lcd_async::raw_framebuf::RawFrameBuf;
//!
//! const WIDTH: usize = 240;
//! const HEIGHT: usize = 240;
//! let mut buffer = [0u8; WIDTH * HEIGHT * 2];
//! let mut fbuf = RawFrameBuf::<Rgb565, _>::new(&mut buffer[..], WIDTH, HEIGHT);
//! fbuf.clear(Rgb565::BLACK).unwrap();
//! Circle::new(Point::new(120, 120), 80)
//!     .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
//!     .draw(&mut fbuf)
//!     .unwrap();
//! // See the examples/ directory for full async display usage
//! ```
//!
//! ## Troubleshooting
//!
//! If you experience issues such as blank screens or incorrect colors, refer to the troubleshooting section in the README or open an issue on the repository.
//!
//! ## License
//!
//! Licensed under MIT, same as the original mipidsi crate.

use dcs::SetAddressMode;

pub mod interface;

use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;

pub mod options;
use options::MemoryMapping;

mod builder;
pub use builder::*;

pub mod dcs;

pub mod models;
pub mod raw_framebuf;
use models::Model;

mod graphics;

mod test_image;
pub use test_image::TestImage;

///
/// Display driver to connect to TFT displays.
///
pub struct Display<DI, MODEL, RST>
where
    DI: interface::Interface,
    MODEL: Model,
    RST: OutputPin,
{
    // DCS provider
    di: DI,
    // Model
    model: MODEL,
    // Reset pin
    rst: Option<RST>,
    // Model Options, includes current orientation
    options: options::ModelOptions,
    // Current MADCTL value copy for runtime updates
    #[allow(dead_code)]
    madctl: SetAddressMode,
    // State monitor for sleeping TODO: refactor to a Model-connected state machine
    sleeping: bool,
}

impl<DI, M, RST> Display<DI, M, RST>
where
    DI: interface::Interface,
    M: Model,
    RST: OutputPin,
{
    ///
    /// Returns currently set [options::Orientation]
    ///
    pub fn orientation(&self) -> options::Orientation {
        self.options.orientation
    }

    ///
    /// Sets display [options::Orientation] with mirror image parameter
    ///
    /// # Examples
    ///
    /// ```
    /// use lcd_async::options::{Orientation, Rotation};
    ///
    /// # tokio_test::block_on(async {
    /// # let mut display = lcd_async::_mock::new_mock_display().await;
    /// display.set_orientation(Orientation::default().rotate(Rotation::Deg180)).await.unwrap();
    /// # });
    /// ```
    pub async fn set_orientation(
        &mut self,
        orientation: options::Orientation,
    ) -> Result<(), DI::Error> {
        self.options.orientation = orientation;
        self.model.update_options(&mut self.di, &self.options).await
    }

    /// Sends a raw pixel data slice to the specified rectangular region of the display.
    pub async fn show_raw_data<DW>(
        &mut self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        pixel_data: &[DW],
    ) -> Result<(), DI::Error>
    where
        DI: interface::Interface<Word = DW>,
        DW: Copy,
    {
        let ex = x + width - 1;
        let ey = y + height - 1;

        self.set_address_window(x, y, ex, ey).await?;
        M::write_memory_start(&mut self.di).await?;
        self.di.send_data_slice(pixel_data).await
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
    pub async fn set_vertical_scroll_region(
        &mut self,
        top_fixed_area: u16,
        bottom_fixed_area: u16,
    ) -> Result<(), DI::Error> {
        M::set_vertical_scroll_region(&mut self.di, top_fixed_area, bottom_fixed_area).await
    }

    /// Sets the vertical scroll offset.
    ///
    /// Setting the vertical scroll offset shifts the vertical scroll region
    /// upwards by `offset` pixels.
    ///
    /// Use [`set_vertical_scroll_region`](Self::set_vertical_scroll_region) to setup the scroll region, before
    /// using this method.
    pub async fn set_vertical_scroll_offset(&mut self, offset: u16) -> Result<(), DI::Error> {
        M::set_vertical_scroll_offset(&mut self.di, offset).await
    }

    ///
    /// Release resources allocated to this driver back.
    /// This returns the display interface, reset pin and and the model deconstructing the driver.
    ///
    pub fn release(self) -> (DI, M, Option<RST>) {
        (self.di, self.model, self.rst)
    }

    // Sets the address window for the display.
    async fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), DI::Error> {
        // add clipping offsets if present
        let mut offset = self.options.display_offset;
        let mapping = MemoryMapping::from(self.options.orientation);
        if mapping.reverse_columns {
            offset.0 = M::FRAMEBUFFER_SIZE.0 - (self.options.display_size.0 + offset.0);
        }
        if mapping.reverse_rows {
            offset.1 = M::FRAMEBUFFER_SIZE.1 - (self.options.display_size.1 + offset.1);
        }
        if mapping.swap_rows_and_columns {
            offset = (offset.1, offset.0);
        }

        let (sx, sy, ex, ey) = (sx + offset.0, sy + offset.1, ex + offset.0, ey + offset.1);

        M::update_address_window(
            &mut self.di,
            self.options.orientation.rotation,
            sx,
            sy,
            ex,
            ey,
        )
        .await
    }

    ///
    /// Configures the tearing effect output.
    ///
    pub async fn set_tearing_effect(
        &mut self,
        tearing_effect: options::TearingEffect,
    ) -> Result<(), DI::Error> {
        M::set_tearing_effect(&mut self.di, tearing_effect, &self.options).await
    }

    ///
    /// Returns `true` if display is currently set to sleep.
    ///
    pub fn is_sleeping(&self) -> bool {
        self.sleeping
    }

    ///
    /// Puts the display to sleep, reducing power consumption.
    /// Need to call [Self::wake] before issuing other commands
    ///
    pub async fn sleep<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DI::Error> {
        M::sleep(&mut self.di, delay).await?;
        self.sleeping = true;
        Ok(())
    }

    ///
    /// Wakes the display after it's been set to sleep via [Self::sleep]
    ///
    pub async fn wake<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DI::Error> {
        M::wake(&mut self.di, delay).await?;
        self.sleeping = false;
        Ok(())
    }

    /// Returns the DCS interface for sending raw commands.
    ///
    /// # Safety
    ///
    /// Sending raw commands to the controller can lead to undefined behaviour,
    /// because the rest of the code isn't aware of any state changes that were caused by sending raw commands.
    /// The user must ensure that the state of the controller isn't altered in a way that interferes with the normal
    /// operation of this crate.
    pub unsafe fn dcs(&mut self) -> &mut DI {
        &mut self.di
    }
}

/// Mock implementations of embedded-hal and interface traits for async architecture.
///
/// Do not use types in this module outside of doc tests.
#[doc(hidden)]
pub mod _mock {
    use core::convert::Infallible;

    use embedded_hal::{digital, spi};
    use embedded_hal_async::delay::DelayNs;

    use crate::{
        interface::{Interface, InterfaceKind},
        models::ILI9341Rgb565,
        Builder, Display, NoResetPin,
    };

    pub async fn new_mock_display() -> Display<MockDisplayInterface, ILI9341Rgb565, NoResetPin> {
        Builder::new(ILI9341Rgb565, MockDisplayInterface)
            .init(&mut MockDelay)
            .await
            .unwrap()
    }

    pub struct MockOutputPin;

    impl digital::OutputPin for MockOutputPin {
        fn set_low(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn set_high(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    impl digital::ErrorType for MockOutputPin {
        type Error = core::convert::Infallible;
    }

    pub struct MockSpi;

    impl spi::SpiDevice for MockSpi {
        fn transaction(
            &mut self,
            _operations: &mut [spi::Operation<'_, u8>],
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    impl spi::ErrorType for MockSpi {
        type Error = core::convert::Infallible;
    }

    pub struct MockDelay;

    impl DelayNs for MockDelay {
        async fn delay_ns(&mut self, _ns: u32) {}
    }

    pub struct MockDisplayInterface;

    impl Interface for MockDisplayInterface {
        type Word = u8;
        type Error = Infallible;

        const KIND: InterfaceKind = InterfaceKind::Serial4Line;

        async fn send_command(&mut self, _command: u8, _args: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn send_data_slice(&mut self, _data: &[Self::Word]) -> Result<(), Self::Error> {
            Ok(())
        }
    }
}
