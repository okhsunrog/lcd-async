# lcd-async

This crate is an `async`-first driver for TFT displays that implement the [MIPI Display Command Set](https://www.mipi.org/specifications/display-command-set).

This project is a fork of the excellent [mipidsi](https://github.com/almindor/mipidsi) crate, but with a fundamentally different, `async`-native architecture. It is heavily inspired by the designs of [st7735-embassy](https://github.com/kalkyl/st7735-embassy) and [embedded-graphics-framebuf](https://github.com/bernii/embedded-graphics-framebuf).

The key architectural changes are:

1.  **Fully Asynchronous:** The entire communication interface (`interface::Interface`) has been redesigned with `async` traits, making it directly compatible with `async` runtimes like [embassy](https://embassy.dev/).
2.  **Framebuffer-Based Drawing:** Instead of drawing primitives directly to the display, this crate uses an "off-screen rendering" workflow. You draw a complete frame into an in-memory buffer (`RawFrameBuf`) and then send the entire buffer to the display in one efficient, asynchronous operation.

## Performance and Architecture Benefits

The design of `lcd-async` offers significant advantages over traditional direct-drawing drivers:

*   **Improved Performance:** The `RawFrameBuf` stores pixel data directly in the display's native byte format. Color conversion from `embedded-graphics` types (e.g., `Rgb565`) to raw bytes only happens for the pixels that are actually drawn. In contrast, drivers that draw directly to the display often need to convert every pixel of a shape or fill area, even those that are ultimately overwritten.
*   **Decoupled Drawing and Sending:** Drawing operations are entirely synchronous and CPU-bound, while sending the framebuffer to the display is an asynchronous, I/O-bound operation. This clean separation allows for advanced patterns like **double buffering**: you can begin rendering the next frame into a second buffer while the hardware is still busy sending the previous frame via DMA.
*   **Async-Native Integration:** By being `async` from the ground up, the driver integrates seamlessly into modern embedded `async` ecosystems without blocking the executor.

## Workflow: Draw, then Show

Drawing is performed in a two-step process:

1.  **Draw:** You create a buffer (e.g., a static array) and wrap it in a `RawFrameBuf`. This framebuffer implements the `embedded-graphics` `DrawTarget` trait. All your drawing operations—clearing, drawing text, shapes, and images—are performed on this in-memory framebuffer.
2.  **Show:** Once your scene is fully rendered in the buffer, you pass a slice of the buffer to the `display.show_raw_data()` method. This `async` method handles sending the complete, raw pixel data to the display controller.

This workflow is demonstrated in the example below.

## Example

This example demonstrates the typical usage pattern with a static buffer and an `async` runtime like `embassy`.

```rust
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Delay;
use static_cell::StaticCell;

use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::primitives::{Circle, PrimitiveStyle};

// This crate's framebuffer and async display interface
use lcd_async::prelude::*;
use lcd_async::framebuffer::RawFrameBuf;

// In a real application, these would come from your HAL and BSP
use your_hal::{Spi, Output, Pin};

const WIDTH: usize = 240;
const HEIGHT: usize = 240;
// Rgb565 uses 2 bytes per pixel
const FRAME_BUFFER_SIZE: usize = WIDTH * HEIGHT * 2;

// Use StaticCell to create a static, zero-initialized buffer.
static FRAME_BUFFER: StaticCell<[u8; FRAME_BUFFER_SIZE]> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // 1. Initialize your hardware (SPI, CS, DC, RST pins)
    // ...
    let spi_bus = ...; // Your async SPI bus
    let cs = ...;      // Your CS OutputPin
    let dc = ...;      // Your DC OutputPin
    let mut rst = ...; // Your RST OutputPin
    let mut delay = Delay;

    // 2. Create the asynchronous display interface
    let spi_device = SpiDevice::new(spi_bus, cs);
    let di = SpiInterface::new(spi_device, dc);

    // 3. Initialize the display driver
    let mut display = Builder::new(ST7789, di) // Using ST7789 as an example model
        .reset_pin(rst)
        .display_size(WIDTH as u16, HEIGHT as u16)
        .init(&mut delay)
        .await
        .unwrap();

    // 4. Initialize the static framebuffer and get a mutable slice to it.
    let frame_buffer = FRAME_BUFFER.init([0; FRAME_BUFFER_SIZE]);

    // 5. Create a framebuffer in a new scope to draw the scene.
    {
        let mut fbuf = RawFrameBuf::<Rgb565, _>::new(frame_buffer, WIDTH, HEIGHT);

        // Draw anything from `embedded-graphics` into the in-memory buffer.
        fbuf.clear(Rgb565::BLACK).unwrap();
        Circle::new(Point::new(120, 120), 80)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
            .draw(&mut fbuf)
            .unwrap();
    } // `fbuf` is dropped here, releasing the mutable borrow.

    // 6. Send the entire rendered frame to the display.
    display
        .show_raw_data(0, 0, WIDTH as u16, HEIGHT as u16, frame_buffer)
        .await
        .unwrap();
}
```

## Supported Models

This fork inherits the excellent model support from `mipidsi`. The following models are supported:

-   GC9107
-   GC9A01
-   ILI9341
-   ILI9342C
-   ILI9486
-   ILI9488
-   RM67162
-   ST7735
-   ST7789
-   ST7796

## Relationship to `mipidsi`

This is a friendly fork of `mipidsi`, created to explore a fully `async` and framebuffer-centric design. All credit for the original models, command sequences, and architecture goes to the `mipidsi` authors and contributors.

## License

Licensed under the MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT), same as the original `mipidsi` crate.