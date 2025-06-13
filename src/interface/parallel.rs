use embedded_hal::digital::OutputPin;

use super::{Interface, InterfaceKind};

/// This trait represents the data pins of a parallel bus.
///
/// See [Generic8BitBus] and [Generic16BitBus] for generic implementations.
pub trait OutputBus {
    /// [u8] for 8-bit buses, [u16] for 16-bit buses, etc.
    type Word: Copy;

    /// Interface kind.
    const KIND: InterfaceKind;

    /// Error type
    type Error: core::fmt::Debug;

    /// Set the output bus to a specific value
    fn set_value(&mut self, value: Self::Word) -> Result<(), Self::Error>;
}

macro_rules! generic_bus {
    ($GenericxBitBus:ident { type Word = $Word:ident; const KIND: InterfaceKind = $InterfaceKind:expr; Pins {$($PX:ident => $x:tt,)*}}) => {
        /// A generic implementation of [OutputBus] using [OutputPin]s
        pub struct $GenericxBitBus<$($PX, )*> {
            pins: ($($PX, )*),
            last: Option<$Word>,
        }

        impl<$($PX, )*> $GenericxBitBus<$($PX, )*>
        where
            $($PX: OutputPin, )*
        {
            /// Creates a new bus. This does not change the state of the pins.
            ///
            /// The first pin in the tuple is the least significant bit.
            pub fn new(pins: ($($PX, )*)) -> Self {
                Self { pins, last: None }
            }

            /// Consumes the bus and returns the pins. This does not change the state of the pins.
            pub fn release(self) -> ($($PX, )*) {
                self.pins
            }
        }

        impl<$($PX, )* E> OutputBus
            for $GenericxBitBus<$($PX, )*>
        where
            $($PX: OutputPin<Error = E>, )*
            E: core::fmt::Debug,
        {
            type Word = $Word;
            type Error = E;

            const KIND: InterfaceKind = $InterfaceKind;

            fn set_value(&mut self, value: Self::Word) -> Result<(), Self::Error> {
                // It's quite common for multiple consecutive values to be identical, e.g. when filling or
                // clearing the screen, so let's optimize for that case.
                // The `Eq` bound for this is on the `ParallelInterface` impl.
                if self.last == Some(value) {
                    return Ok(())
                }

                // Sets self.last to None.
                // We will update it to Some(value) *after* all the pins are succesfully set.
                let last = self.last.take();

                let changed = match last {
                    Some(old_value) => value ^ old_value,
                    None => !0, // all ones, this ensures that we will update all the pins
                };

                $(
                    let mask = 1 << $x;
                    if changed & mask != 0 {
                        if value & mask != 0 {
                            self.pins.$x.set_high()
                        } else {
                            self.pins.$x.set_low()
                        }
                        ?;
                    }
                )*

                self.last = Some(value);
                Ok(())
            }
        }

        impl<$($PX, )*> From<($($PX, )*)>
            for $GenericxBitBus<$($PX, )*>
        where
            $($PX: OutputPin, )*
        {
            fn from(pins: ($($PX, )*)) -> Self {
                Self::new(pins)
            }
        }
    };
}

generic_bus! {
    Generic8BitBus {
        type Word = u8;
        const KIND: InterfaceKind = InterfaceKind::Parallel8Bit;
        Pins {
            P0 => 0,
            P1 => 1,
            P2 => 2,
            P3 => 3,
            P4 => 4,
            P5 => 5,
            P6 => 6,
            P7 => 7,
        }
    }
}

generic_bus! {
    Generic16BitBus {
        type Word = u16;
        const KIND: InterfaceKind = InterfaceKind::Parallel16Bit;
        Pins {
            P0 => 0,
            P1 => 1,
            P2 => 2,
            P3 => 3,
            P4 => 4,
            P5 => 5,
            P6 => 6,
            P7 => 7,
            P8 => 8,
            P9 => 9,
            P10 => 10,
            P11 => 11,
            P12 => 12,
            P13 => 13,
            P14 => 14,
            P15 => 15,
        }
    }
}

/// Parallel interface error
#[derive(Clone, Copy, Debug)]
pub enum ParallelError<BUS, DC, WR> {
    /// Bus error
    Bus(BUS),
    /// Data/command pin error
    Dc(DC),
    /// Write pin error
    Wr(WR),
}

/// Parallel communication interface
///
/// This interface implements a "8080" style write-only display interface using any
/// [`OutputBus`] implementation as well as one
/// [`OutputPin`] for the data/command selection and one [`OutputPin`] for the write-enable flag.
///
/// All pins in the data bus are supposed to be high-active. High for the D/C pin meaning "data" and the
/// write-enable being pulled low before the setting of the bits and supposed to be sampled at a
/// low to high edge.
pub struct ParallelInterface<BUS, DC, WR> {
    bus: BUS,
    dc: DC,
    wr: WR,
}

impl<BUS, DC, WR> ParallelInterface<BUS, DC, WR>
where
    BUS: OutputBus,
    // The Eq bound is used by the `set_value` optimization in the generic bus
    BUS::Word: From<u8> + Eq + core::ops::BitXor<Output = BUS::Word>,
    DC: OutputPin,
    WR: OutputPin,
{
    /// Create new parallel GPIO interface for communication with a display driver
    pub fn new(bus: BUS, dc: DC, wr: WR) -> Self {
        Self { bus, dc, wr }
    }

    /// Consume the display interface and return
    /// the bus and GPIO pins used by it
    pub fn release(self) -> (BUS, DC, WR) {
        (self.bus, self.dc, self.wr)
    }

    /// Sends a single word to the display.
    fn send_word(
        &mut self,
        word: BUS::Word,
    ) -> Result<(), ParallelError<BUS::Error, DC::Error, WR::Error>> {
        self.wr.set_low().map_err(ParallelError::Wr)?;
        self.bus.set_value(word).map_err(ParallelError::Bus)?;
        self.wr.set_high().map_err(ParallelError::Wr)
    }
}

impl<BUS, DC, WR> Interface for ParallelInterface<BUS, DC, WR>
where
    BUS: OutputBus,
    // The Eq bound is used by the `set_value` optimization in the generic bus.
    // The BitXor is also needed for that optimization.
    BUS::Word: From<u8> + Eq + core::ops::BitXor<Output = BUS::Word>,
    DC: OutputPin,
    WR: OutputPin,
{
    type Word = BUS::Word;
    type Error = ParallelError<BUS::Error, DC::Error, WR::Error>;

    const KIND: InterfaceKind = BUS::KIND;

    async fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        // Set DC pin low for command
        self.dc.set_low().map_err(ParallelError::Dc)?;
        self.send_word(BUS::Word::from(command))?;

        // Set DC pin high for data
        self.dc.set_high().map_err(ParallelError::Dc)?;
        for &arg in args {
            self.send_word(BUS::Word::from(arg))?;
        }

        Ok(())
    }

    async fn send_data_slice(&mut self, data: &[Self::Word]) -> Result<(), Self::Error> {
        // DC pin is expected to be high (data mode) from a previous command.
        // We just need to send the words.
        for &word in data {
            self.send_word(word)?;
        }
        Ok(())
    }
}
