#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Primitive, PrimitiveStyle, Triangle},
};
use esp_hal::{
    clock::CpuClock,
    dma::{DmaRxBuf, DmaTxBuf},
    gpio::{Level, Output},
    spi::{
        master::{Config, Spi, SpiDmaBus},
        Mode,
    },
    time::Rate,
    timer::systimer::SystemTimer,
    Async,
};
use lcd_async::{
    interface,
    models::ST7789,
    options::{ColorInversion, Orientation, Rotation},
    raw_framebuf::RawFrameBuf,
    Builder,
};
use panic_rtt_target as _;
use static_cell::StaticCell;

// Display parameters
const WIDTH: u16 = 240;
const HEIGHT: u16 = 240;
const PIXEL_SIZE: usize = 2; // RGB565 = 2 bytes per pixel
const FRAME_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize) * PIXEL_SIZE;

static FRAME_BUFFER: StaticCell<[u8; FRAME_SIZE]> = StaticCell::new();

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    // Create DMA buffers for SPI
    #[allow(clippy::manual_div_ceil)]
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = esp_hal::dma_buffers!(4, 32_000);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    // SPI pins for ESP32-C3 (adjust these according to your wiring)
    let sclk = peripherals.GPIO6; // SCL
    let mosi = peripherals.GPIO7; // SDA
    let res = peripherals.GPIO10; // RES (Reset)
    let dc = peripherals.GPIO2; // DC (Data/Command)
    let cs = peripherals.GPIO3; // CS (Chip Select)

    // Create SPI with DMA
    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(20))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(mosi)
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    // Create control pins
    let res = Output::new(res, Level::Low, Default::default());
    let dc = Output::new(dc, Level::Low, Default::default());
    let cs = Output::new(cs, Level::High, Default::default());

    // Create shared SPI bus
    static SPI_BUS: StaticCell<Mutex<NoopRawMutex, SpiDmaBus<'static, Async>>> = StaticCell::new();
    let spi_bus = Mutex::new(spi);
    let spi_bus = SPI_BUS.init(spi_bus);
    let spi_device = SpiDevice::new(spi_bus, cs);

    // Create display interface
    let di = interface::SpiInterface::new(spi_device, dc);
    let mut delay = embassy_time::Delay;

    // Initialize the display
    let mut display = Builder::new(ST7789, di)
        .reset_pin(res)
        .display_size(WIDTH, HEIGHT)
        .orientation(Orientation {
            rotation: Rotation::Deg0,
            mirrored: false,
        })
        .display_offset(0, 0)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut delay)
        .await
        .unwrap();

    info!("Display initialized!");

    // Initialize frame buffer
    let frame_buffer = FRAME_BUFFER.init([0; FRAME_SIZE]);

    // Create a framebuffer for drawing
    let mut raw_fb =
        RawFrameBuf::<Rgb565, _>::new(frame_buffer.as_mut_slice(), WIDTH.into(), HEIGHT.into());

    // Clear the framebuffer to black
    raw_fb.clear(Rgb565::BLACK).unwrap();

    // Draw a simple smiley face
    draw_smiley(&mut raw_fb).unwrap();

    // Send the framebuffer data to the display
    display
        .show_raw_data(0, 0, WIDTH, HEIGHT, frame_buffer)
        .await
        .unwrap();

    info!("Smiley face drawn!");

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

fn draw_smiley<T>(display: &mut T) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    // Draw the left eye as a circle located at (80, 80), with a diameter of 30, filled with white
    Circle::new(Point::new(80, 80), 30)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

    // Draw the right eye as a circle located at (130, 80), with a diameter of 30, filled with white
    Circle::new(Point::new(130, 80), 30)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

    // Draw an upside down triangle to represent a smiling mouth
    Triangle::new(
        Point::new(80, 140),  // Left point
        Point::new(160, 140), // Right point
        Point::new(120, 180), // Bottom point
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
    .draw(display)?;

    // Cover the top part of the mouth with a black triangle so it looks like a smile
    Triangle::new(
        Point::new(90, 150),  // Left point
        Point::new(150, 150), // Right point
        Point::new(120, 170), // Bottom point
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
    .draw(display)?;

    Ok(())
}
