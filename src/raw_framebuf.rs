//! A framebuffer that stores pixels as raw bytes, suitable for direct display transmission.
//!
//! This module provides [`RawFrameBuf`], a `DrawTarget` implementation that allows
//! `embedded-graphics` primitives to be rendered directly into an in-memory byte buffer.
//! This is a common pattern for "off-screen rendering," where a complete frame is prepared
//! in a buffer before being sent to the display in a single, efficient operation (e.g., via DMA).
//!
//! The framebuffer is generic over a color type `C` and a buffer backend `BUF`. The key
//! component is the [`IntoRawBytes`] trait, which defines how a given `PixelColor` is
//! converted into its byte representation. This allows the framebuffer to automatically
//! handle different color formats (like `Rgb565`, `Rgb888`, etc.) without needing
//! the user to specify the bytes-per-pixel manually.
//!
//! # Usage with an Async Display Driver
//!
//! This framebuffer is ideal for use with asynchronous display drivers, like an async fork of `mipidsi`.
//! The typical workflow is:
//!
//! 1.  Allocate a buffer large enough for one full frame (often on the heap using `alloc`).
//! 2.  Create a `RawFrameBuf` inside a new scope, wrapping a mutable slice of the buffer.
//! 3.  Use `embedded-graphics` commands to draw a complete scene to the `RawFrameBuf`.
//! 4.  Once the scope ends, `RawFrameBuf` is dropped, releasing its borrow on the buffer.
//! 5.  The now-populated buffer is passed to an async method on the display driver to be rendered.
//!
//! # Example
//!
//! ```
//! use embedded_graphics::pixelcolor::Rgb565;
//! use embedded_graphics::prelude::*;
//! use embedded_graphics::primitives::{Circle, PrimitiveStyle};
//! use lcd_async::raw_framebuf::{RawFrameBuf, IntoRawBytes};
//!
//! const WIDTH: usize = 64;
//! const HEIGHT: usize = 64;
//! const FRAME_SIZE: usize = WIDTH * HEIGHT * 2; // Rgb565 = 2 bytes per pixel
//!
//! // Create a static buffer
//! let mut frame_buffer = [0u8; FRAME_SIZE];
//!
//! // Create the framebuffer
//! let mut fbuf = RawFrameBuf::<Rgb565, _>::new(&mut frame_buffer[..], WIDTH, HEIGHT);
//!
//! // Draw a scene to the in-memory framebuffer
//! fbuf.clear(Rgb565::BLACK).unwrap();
//! Circle::new(Point::new(32, 32), 20)
//!     .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
//!     .draw(&mut fbuf)
//!     .unwrap();
//!
//! // The frame_buffer now contains the rendered frame data
//! assert_eq!(fbuf.width(), WIDTH);
//! assert_eq!(fbuf.height(), HEIGHT);
//! ```

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, OriginDimensions},
    pixelcolor::{raw::RawU16, PixelColor, RgbColor},
    prelude::*,
    primitives::Rectangle,
    Pixel,
};

/// A trait for converting a `PixelColor` into its raw byte representation.
///
/// This trait is the bridge between `embedded-graphics` color types and a raw byte
/// buffer. By implementing this trait for a color, you define how it should be serialized
/// into bytes for the display.
pub trait IntoRawBytes: PixelColor + Sized {
    /// The number of bytes used to represent one pixel of this color.
    const BYTES_PER_PIXEL: usize;

    /// The fixed-size array type that holds the raw byte data for a single pixel.
    type Raw: AsRef<[u8]> + AsMut<[u8]> + Copy + Default;

    /// Converts the color instance into its raw byte representation.
    fn into_raw_bytes(self) -> <Self as IntoRawBytes>::Raw;
}

impl IntoRawBytes for embedded_graphics::pixelcolor::Rgb565 {
    const BYTES_PER_PIXEL: usize = 2;
    type Raw = [u8; 2];

    fn into_raw_bytes(self) -> <Self as IntoRawBytes>::Raw {
        RawU16::from(self).into_inner().to_be_bytes()
    }
}

impl IntoRawBytes for embedded_graphics::pixelcolor::Rgb888 {
    const BYTES_PER_PIXEL: usize = 3;
    type Raw = [u8; 3];

    fn into_raw_bytes(self) -> <Self as IntoRawBytes>::Raw {
        [self.r(), self.g(), self.b()]
    }
}

/// A trait for abstracting over a mutable byte buffer.
///
/// This allows [`RawFrameBuf`] to be agnostic to the underlying buffer's storage,
/// accepting anything that can provide a mutable byte slice, such as a `&mut [u8]`,
/// a `Vec<u8>`, or a custom memory-mapped region.
pub trait RawBufferBackendMut {
    /// Returns a mutable slice to the entire buffer.
    fn as_mut_u8_slice(&mut self) -> &mut [u8];

    /// Returns an immutable slice to the entire buffer.
    fn as_u8_slice(&self) -> &[u8];

    /// Returns the total length of the buffer in bytes.
    fn u8_len(&self) -> usize;
}

impl<'a> RawBufferBackendMut for &'a mut [u8] {
    fn as_mut_u8_slice(&mut self) -> &mut [u8] {
        self
    }

    fn as_u8_slice(&self) -> &[u8] {
        self
    }

    fn u8_len(&self) -> usize {
        self.len()
    }
}

/// A framebuffer that writes pixel data directly into a raw byte buffer.
///
/// This struct implements [`DrawTarget`] and is generic over a color format `C`
/// (which must implement [`IntoRawBytes`]) and a buffer backend `BUF` (which must
/// implement [`RawBufferBackendMut`]). See the module-level documentation for a usage example.
pub struct RawFrameBuf<C, BUF>
where
    C: IntoRawBytes,
    BUF: RawBufferBackendMut,
{
    buffer: BUF,
    width: usize,
    height: usize,
    _phantom_color: core::marker::PhantomData<C>,
}

impl<C, BUF> RawFrameBuf<C, BUF>
where
    C: IntoRawBytes,
    BUF: RawBufferBackendMut,
{
    /// Creates a new raw framebuffer.
    ///
    /// # Panics
    ///
    /// Panics if the provided `buffer` is smaller than `width * height * C::BYTES_PER_PIXEL`.
    pub fn new(buffer: BUF, width: usize, height: usize) -> Self {
        let expected_len = width * height * C::BYTES_PER_PIXEL;
        assert!(
            buffer.u8_len() >= expected_len,
            "RawFrameBuf underlying buffer is too small. Expected at least {}, got {}.",
            expected_len,
            buffer.u8_len()
        );
        Self {
            buffer,
            width,
            height,
            _phantom_color: core::marker::PhantomData,
        }
    }

    /// Returns the width of the framebuffer in pixels.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the framebuffer in pixels.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the raw framebuffer data as an immutable byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        let expected_len = self.width * self.height * C::BYTES_PER_PIXEL;
        &self.buffer.as_u8_slice()[0..expected_len]
    }

    /// Returns the raw framebuffer data as a mutable byte slice.
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        let expected_len = self.width * self.height * C::BYTES_PER_PIXEL;
        &mut self.buffer.as_mut_u8_slice()[0..expected_len]
    }
}

impl<C, BUF> OriginDimensions for RawFrameBuf<C, BUF>
where
    C: IntoRawBytes,
    BUF: RawBufferBackendMut,
{
    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }
}

impl<C, BUF> DrawTarget for RawFrameBuf<C, BUF>
where
    C: IntoRawBytes,
    BUF: RawBufferBackendMut,
{
    type Color = C;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let bounding_box = self.bounding_box();
        let current_width = self.width;

        let buffer_slice = self.buffer.as_mut_u8_slice();
        let active_buffer_len = self.width * self.height * C::BYTES_PER_PIXEL;

        for Pixel(coord, color) in pixels.into_iter() {
            if bounding_box.contains(coord) {
                let byte_index =
                    (coord.y as usize * current_width + coord.x as usize) * C::BYTES_PER_PIXEL;

                let color_bytes = color.into_raw_bytes();

                if byte_index + C::BYTES_PER_PIXEL <= active_buffer_len {
                    buffer_slice[byte_index..byte_index + C::BYTES_PER_PIXEL]
                        .copy_from_slice(color_bytes.as_ref());
                }
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let color_bytes_array = color.into_raw_bytes();
        let color_bytes = color_bytes_array.as_ref();

        let buffer_slice = self.buffer.as_mut_u8_slice();
        let active_buffer_len = self.width * self.height * C::BYTES_PER_PIXEL;
        let active_slice = &mut buffer_slice[0..active_buffer_len];

        let all_bytes_same = if let Some(first) = color_bytes.first() {
            color_bytes.iter().all(|&b| b == *first)
        } else {
            true
        };

        if all_bytes_same && !color_bytes.is_empty() {
            active_slice.fill(color_bytes[0]);
        } else if C::BYTES_PER_PIXEL > 0 {
            for chunk in active_slice.chunks_exact_mut(C::BYTES_PER_PIXEL) {
                chunk.copy_from_slice(color_bytes);
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let drawable_area = area.intersection(&self.bounding_box());
        if drawable_area.is_zero_sized() {
            return Ok(());
        }

        let color_bytes_array = color.into_raw_bytes();
        let color_bytes = color_bytes_array.as_ref();

        let current_width = self.width;
        let buffer_slice = self.buffer.as_mut_u8_slice();

        for p in drawable_area.points() {
            let byte_index = (p.y as usize * current_width + p.x as usize) * C::BYTES_PER_PIXEL;

            if byte_index + C::BYTES_PER_PIXEL <= buffer_slice.len() {
                buffer_slice[byte_index..byte_index + C::BYTES_PER_PIXEL]
                    .copy_from_slice(color_bytes);
            }
        }
        Ok(())
    }
}
