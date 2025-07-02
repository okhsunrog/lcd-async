//! # Troubleshooting guide
//!
//! This guide lists common issues that can cause a blank or corrupted display.
//!
//! ## Display stays black/blank
//!
//! ### Reset pin
//!
//! The reset pin on all supported display controllers is active low, requiring
//! it to be driven **high** in order for the display to operate. It is
//! recommended to connect the reset pin to a GPIO pin and let this crate
//! control the pin by passing it to the builder via the `reset_pin` method. If
//! this isn't possible in the target application the user must make sure that
//! the reset pin on the display controller is kept in the high state before
//! `init` is called.
//!
//! ### Backlight pin
//!
//! This driver does **NOT** handle the backlight pin to keep the code simpler.
//! Users must control the backlight manually. First thing to try is to see if
//! setting the backlight pin to high fixes the issue.
//!
//! ### Transport misconfiguration (e.g. SPI)
//!
//! Make sure that the transport layer is configured correctly. Typical mistakes
//! are the use of wrong SPI MODE or too fast transfer speeds that are not
//! supported by the display
//!
//! ## Incorrect colors
//!
//! The way colors are displayed depend on the subpixel layout and technology
//! (like TN or IPS) used by the LCD panel. These physical parameters aren't
//! known by the display controller and must be manually set by the user as
//! `Builder` settings when the display is initialized.
//!
//! To make it easier to identify the correct settings the `lcd-async` crate
//! provides a [`TestImage`](crate::TestImage), which can be used to verify the
//! color settings and adjust them in case they are incorrect.
//!
//! ```
//! use embedded_graphics::prelude::*;
//! use embedded_graphics::pixelcolor::Rgb565;
//! use lcd_async::{Builder, TestImage, models::ILI9341Rgb565, raw_framebuf::RawFrameBuf};
//!
//! # tokio_test::block_on(async {
//! # let di = lcd_async::_mock::MockDisplayInterface;
//! # let rst = lcd_async::_mock::MockOutputPin;
//! # let mut delay = lcd_async::_mock::MockDelay;
//! let mut display = Builder::new(ILI9341Rgb565, di)
//!     .reset_pin(rst)
//!     .init(&mut delay)
//!     .await
//!     .unwrap();
//!
//! // Create framebuffer for drawing
//! const WIDTH: usize = 240;
//! const HEIGHT: usize = 320;
//! let mut buffer = [0u8; WIDTH * HEIGHT * 2]; // 2 bytes per pixel for RGB565
//! let mut framebuf = RawFrameBuf::<Rgb565, _>::new(&mut buffer[..], WIDTH, HEIGHT);
//!
//! // Draw test image to framebuffer
//! TestImage::new().draw(&mut framebuf)?;
//!
//! // IMPORTANT: After drawing to the framebuffer, you must send it to the display!
//! // This is the key step that actually updates the screen.
//! // display.show_raw_data(0, 0, WIDTH as u16, HEIGHT as u16, &buffer).await.unwrap();
//!
//! // For a complete working example, see: examples/spi-st7789-esp32-c3/src/main.rs
//! # Ok::<(), core::convert::Infallible>(())
//! # });
//! ```
//!
//! The expected output from drawing the test image is:
//!
#![doc = include_str!("../docs/colors_correct.svg")]
//!
//! If the test image isn't displayed as expected use one of the reference image
//! below the determine which settings need to be added to the
//! [`Builder`](crate::Builder).
//!
//! ### Wrong subpixel order
//!
#![doc = include_str!("../docs/colors_wrong_subpixel_order.svg")]
//!
//! ```
//! # use embedded_graphics::prelude::*;
//! # use embedded_graphics::pixelcolor::Rgb565;
//! # use lcd_async::{Builder, TestImage, models::ILI9341Rgb565, raw_framebuf::RawFrameBuf};
//! #
//! # tokio_test::block_on(async {
//! # let di = lcd_async::_mock::MockDisplayInterface;
//! # let mut delay = lcd_async::_mock::MockDelay;
//! # let mut display = Builder::new(ILI9341Rgb565, di)
//! .color_order(lcd_async::options::ColorOrder::Bgr)
//! # .init(&mut delay).await.unwrap();
//! # });
//! ```
//!
//! ### Wrong color inversion
//!
#![doc = include_str!("../docs/colors_wrong_color_inversion.svg")]
//!
//! ```
//! # use embedded_graphics::prelude::*;
//! # use embedded_graphics::pixelcolor::Rgb565;
//! # use lcd_async::{Builder, TestImage, models::ILI9341Rgb565, raw_framebuf::RawFrameBuf};
//! #
//! # tokio_test::block_on(async {
//! # let di = lcd_async::_mock::MockDisplayInterface;
//! # let mut delay = lcd_async::_mock::MockDelay;
//! # let mut display = Builder::new(ILI9341Rgb565, di)
//! .invert_colors(lcd_async::options::ColorInversion::Inverted)
//! # .init(&mut delay).await.unwrap();
//! # });
//! ```
//!
//! ### Wrong subpixel order and color inversion
//!
#![doc = include_str!("../docs/colors_both_wrong.svg")]
//!
//! ```
//! # use embedded_graphics::prelude::*;
//! # use embedded_graphics::pixelcolor::Rgb565;
//! # use lcd_async::{Builder, TestImage, models::ILI9341Rgb565, raw_framebuf::RawFrameBuf};
//! #
//! # tokio_test::block_on(async {
//! # let di = lcd_async::_mock::MockDisplayInterface;
//! # let mut delay = lcd_async::_mock::MockDelay;
//! # let mut display = Builder::new(ILI9341Rgb565, di)
//! .color_order(lcd_async::options::ColorOrder::Bgr)
//! .invert_colors(lcd_async::options::ColorInversion::Inverted)
//! # .init(&mut delay).await.unwrap();
//! # });
//! ```
