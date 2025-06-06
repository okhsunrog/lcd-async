//! A framebuffer that stores pixels as raw bytes, suitable for direct display transmission.
//! It implements `embedded_graphics::DrawTarget` by converting `PixelColor` to bytes on draw.

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, OriginDimensions},
    pixelcolor::raw::RawData,
    pixelcolor::PixelColor,
    pixelcolor::RgbColor,
    prelude::Size,
    primitives::Rectangle,
    Pixel,
};

// --- Helper Trait for Color to Raw Byte Conversion ---
pub trait IntoRawBytes<const N: usize>: PixelColor {
    fn into_raw_bytes(self) -> [u8; N];
    const BYTES_PER_PIXEL: usize = N;
}

impl IntoRawBytes<2> for embedded_graphics::pixelcolor::Rgb565 {
    fn into_raw_bytes(self) -> [u8; 2] {
        use embedded_graphics::pixelcolor::raw::RawU16;
        RawU16::from(self).into_inner().to_be_bytes()
    }
}

impl IntoRawBytes<3> for embedded_graphics::pixelcolor::Rgb888 {
    fn into_raw_bytes(self) -> [u8; 3] {
        [self.r(), self.g(), self.b()]
    }
}

// --- Backend Trait for Buffer Flexibility ---
pub trait RawBufferBackendMut {
    fn as_mut_u8_slice(&mut self) -> &mut [u8];
    fn as_u8_slice(&self) -> &[u8];
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

pub struct RawFrameBuf<C, BUF, const N: usize>
where
    C: IntoRawBytes<N>,
    BUF: RawBufferBackendMut,
{
    buffer: BUF,
    width: usize,
    height: usize,
    _phantom_color: core::marker::PhantomData<C>,
}

impl<C, BUF, const N: usize> RawFrameBuf<C, BUF, N>
where
    C: IntoRawBytes<N>,
    BUF: RawBufferBackendMut,
{
    pub fn new(buffer: BUF, width: usize, height: usize) -> Self {
        let expected_len = width * height * N;
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

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn as_bytes(&self) -> &[u8] {
        let expected_len = self.width * self.height * N;
        &self.buffer.as_u8_slice()[0..expected_len]
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        let expected_len = self.width * self.height * N;
        &mut self.buffer.as_mut_u8_slice()[0..expected_len]
    }

    // This method is not strictly needed if calculations are done in place,
    // but if kept, it should be `&self`.
    // fn point_to_byte_index(&self, p: Point) -> usize {
    //     (p.y as usize * self.width + p.x as usize) * N
    // }
}

impl<C, BUF, const N: usize> OriginDimensions for RawFrameBuf<C, BUF, N>
where
    C: IntoRawBytes<N>,
    BUF: RawBufferBackendMut,
{
    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }
}

impl<C, BUF, const N: usize> DrawTarget for RawFrameBuf<C, BUF, N>
where
    C: IntoRawBytes<N>,
    BUF: RawBufferBackendMut,
{
    type Color = C;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let current_width = self.width; // Capture width to avoid re-borrowing self later
        let current_height = self.height; // Capture height
        let buffer_slice = self.buffer.as_mut_u8_slice();
        let active_buffer_len = current_width * current_height * N;

        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.x < current_width as i32
                && coord.y >= 0
                && coord.y < current_height as i32
            {
                let byte_index = (coord.y as usize * current_width + coord.x as usize) * N;
                let color_bytes = color.into_raw_bytes();

                if byte_index + N <= active_buffer_len {
                    buffer_slice[byte_index..byte_index + N].copy_from_slice(&color_bytes);
                }
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let color_bytes = color.into_raw_bytes(); // [byte1, byte2, ..., byteN]
        let buffer_slice = self.buffer.as_mut_u8_slice();
        let active_buffer_len = self.width * self.height * N;
        let active_slice = &mut buffer_slice[0..active_buffer_len];

        // Special case: if all bytes in color_bytes are the same
        // This is true for Rgb565::BLACK ([0,0]), Rgb565::WHITE ([255,255]),
        // and grayscale colors where R=G=B.
        let mut all_bytes_same = true;
        if N > 0 {
            for i in 1..N {
                if color_bytes[i] != color_bytes[0] {
                    all_bytes_same = false;
                    break;
                }
            }
        } else {
            // N == 0, should not happen with IntoRawBytes<N> if N > 0
            return Ok(());
        }

        if N > 0 && all_bytes_same {
            // If all bytes of the pixel color are identical (e.g., Rgb565::BLACK is [0,0]),
            // we can use the highly optimized slice::fill.
            active_slice.fill(color_bytes[0]);
        } else {
            // Fallback for multi-byte patterns where bytes differ (e.g., Rgb565::RED is [0xF8, 0x00])
            // or if N == 0 (though N should always be > 0 here)
            for chunk in active_slice.chunks_exact_mut(N) {
                chunk.copy_from_slice(&color_bytes);
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let drawable_area = area.intersection(&self.bounding_box()); // self.bounding_box() is fine
        if drawable_area.is_zero_sized() {
            return Ok(());
        }

        let color_bytes = color.into_raw_bytes();
        let current_width = self.width; // Capture width
        let buffer_slice = self.buffer.as_mut_u8_slice();
        let active_buffer_len = current_width * self.height * N;

        for y_coord in
            drawable_area.top_left.y..(drawable_area.top_left.y + drawable_area.size.height as i32)
        {
            for x_coord in drawable_area.top_left.x
                ..(drawable_area.top_left.x + drawable_area.size.width as i32)
            {
                // Bounds check against self.width and self.height already handled by intersection
                // and loop bounds.
                let byte_index = (y_coord as usize * current_width + x_coord as usize) * N;
                if byte_index + N <= active_buffer_len {
                    buffer_slice[byte_index..byte_index + N].copy_from_slice(&color_bytes);
                }
            }
        }
        Ok(())
    }
}
