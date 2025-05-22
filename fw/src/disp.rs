use core::cell::RefCell;

use cortex_m::interrupt::Mutex;
use embedded_hal::digital::OutputPin;
use hdsplib::utils::udiv_ceil;
use rp235x_hal::gpio::PinState;
use rtt_target::rprintln;
// use stm32f4xx_hal::{
//     gpio::{Output, Pin},
//     interrupt,
//     pac::{SPI2, TIM2, TIM5},
//     prelude::*,
//     spi::{self, FrameSize, Spi},
//     timer::{CounterMs, CounterUs, Delay},
// };
use hal::gpio::PinId;
use hal::{
    fugit::RateExtU32,
    gpio::{
        bank0::{Gpio2, Gpio3, Gpio5},
        FunctionNull, Pin, PullDown,
    },
    gpio::{FunctionSio, SioOutput, ValidFunction},
    pac,
    spi::{SpiDevice, ValidSpiPinout},
    Clock,
};
use rp235x_hal as hal;

pub const VCOM_PERIOD_MS: u32 = 250;

pub struct Display<
    RstPin: PinId,
    FlPin: PinId,
    A0Pin: PinId,
    A1Pin: PinId,
    A2Pin: PinId,
    A3Pin: PinId,
    A4Pin: PinId,
    ClsPin: PinId,
    ClkPin: PinId,
    WrPin: PinId,
    CePin: PinId,
    RdPin: PinId,
    D0Pin: PinId,
    D1Pin: PinId,
    D2Pin: PinId,
    D3Pin: PinId,
    D4Pin: PinId,
    D5Pin: PinId,
    D6Pin: PinId,
    D7Pin: PinId,
> {
    rst: Pin<RstPin, FunctionSio<SioOutput>, PullDown>,
    fl: Pin<FlPin, FunctionSio<SioOutput>, PullDown>,

    address_bus: GpioBus5<
        Pin<A0Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<A1Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<A2Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<A3Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<A4Pin, FunctionSio<SioOutput>, PullDown>,
    >,
    cls: Pin<ClsPin, FunctionSio<SioOutput>, PullDown>,
    clk: Pin<ClkPin, FunctionSio<SioOutput>, PullDown>,
    wr: Pin<WrPin, FunctionSio<SioOutput>, PullDown>,
    ce: Pin<CePin, FunctionSio<SioOutput>, PullDown>,
    rd: Pin<RdPin, FunctionSio<SioOutput>, PullDown>,
    data_bus: GpioBus8<
        Pin<D0Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<D1Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<D2Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<D3Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<D4Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<D5Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<D6Pin, FunctionSio<SioOutput>, PullDown>,
        Pin<D7Pin, FunctionSio<SioOutput>, PullDown>,
    >,

    text: heapless::String<128>,
    text_scroll_pos: usize,
}

impl<
        'a,
        RstPin: PinId,
        FlPin: PinId,
        A0Pin: PinId,
        A1Pin: PinId,
        A2Pin: PinId,
        A3Pin: PinId,
        A4Pin: PinId,
        ClsPin: PinId,
        ClkPin: PinId,
        WrPin: PinId,
        CePin: PinId,
        RdPin: PinId,
        D0Pin: PinId,
        D1Pin: PinId,
        D2Pin: PinId,
        D3Pin: PinId,
        D4Pin: PinId,
        D5Pin: PinId,
        D6Pin: PinId,
        D7Pin: PinId,
    >
    Display<
        RstPin,
        FlPin,
        A0Pin,
        A1Pin,
        A2Pin,
        A3Pin,
        A4Pin,
        ClsPin,
        ClkPin,
        WrPin,
        CePin,
        RdPin,
        D0Pin,
        D1Pin,
        D2Pin,
        D3Pin,
        D4Pin,
        D5Pin,
        D6Pin,
        D7Pin,
    >
{
    pub fn new(
        rst: Pin<RstPin, FunctionSio<SioOutput>, PullDown>,
        fl: Pin<FlPin, FunctionSio<SioOutput>, PullDown>,
        a0: Pin<A0Pin, FunctionSio<SioOutput>, PullDown>,
        a1: Pin<A1Pin, FunctionSio<SioOutput>, PullDown>,
        a2: Pin<A2Pin, FunctionSio<SioOutput>, PullDown>,
        a3: Pin<A3Pin, FunctionSio<SioOutput>, PullDown>,
        a4: Pin<A4Pin, FunctionSio<SioOutput>, PullDown>,
        cls: Pin<ClsPin, FunctionSio<SioOutput>, PullDown>,
        clk: Pin<ClkPin, FunctionSio<SioOutput>, PullDown>,
        wr: Pin<WrPin, FunctionSio<SioOutput>, PullDown>,
        ce: Pin<CePin, FunctionSio<SioOutput>, PullDown>,
        rd: Pin<RdPin, FunctionSio<SioOutput>, PullDown>,
        d0: Pin<D0Pin, FunctionSio<SioOutput>, PullDown>,
        d1: Pin<D1Pin, FunctionSio<SioOutput>, PullDown>,
        d2: Pin<D2Pin, FunctionSio<SioOutput>, PullDown>,
        d3: Pin<D3Pin, FunctionSio<SioOutput>, PullDown>,
        d4: Pin<D4Pin, FunctionSio<SioOutput>, PullDown>,
        d5: Pin<D5Pin, FunctionSio<SioOutput>, PullDown>,
        d6: Pin<D6Pin, FunctionSio<SioOutput>, PullDown>,
        d7: Pin<D7Pin, FunctionSio<SioOutput>, PullDown>,
    ) -> Self {
        Self {
            rst,
            fl,

            address_bus: GpioBus5::new((a0, a1, a2, a3, a4)),

            cls,
            clk,
            wr,
            ce,
            rd,

            data_bus: GpioBus8::new((d0, d1, d2, d3, d4, d5, d6, d7)),
            text: heapless::String::new(),
            text_scroll_pos: 0,
        }
    }

    pub fn reset(&mut self, delay: &mut impl embedded_hal::delay::DelayNs) {
        self.rst.set_low().unwrap();
        self.ce.set_high().unwrap();
        delay.delay_us(1000);

        self.rst.set_high().unwrap();
        self.ce.set_low().unwrap();
        delay.delay_us(1000);
    }

    pub fn init(&mut self, delay: &mut impl embedded_hal::delay::DelayNs) {
        self.reset(delay);
        self.wr.set_high().unwrap(); // WR is low when writing
        self.fl.set_high().unwrap(); // FL is low when accessing flash
        self.cls.set_high().unwrap(); // CLS is high when using internal clock
        self.ce.set_high().unwrap(); // CE is low when reading or writing data
        self.rd.set_high().unwrap(); // RD is low when reading data
    }

    pub fn write_char(&mut self, pos: u8, data: u8, delay: &mut impl embedded_hal::delay::DelayNs) {
        // First three set the location in char ram (first digit is 000??)
        // self.a0.set_low ().unwrap();
        // self.a1.set_low().unwrap();
        // self.a2.set_low().unwrap();

        // self.a3.set_high().unwrap();
        // self.a4.set_high().unwrap();

        self.address_bus.set(0x18 | (pos & 0x07));

        // Set the data
        self.data_bus.set(data & 0x7F);

        self.latch_input(delay);
    }

    pub fn latch_input(&mut self, delay: &mut impl embedded_hal::delay::DelayNs) {
        self.wr.set_low().unwrap();
        self.ce.set_low().unwrap();
        delay.delay_us(100);
        self.ce.set_high().unwrap();
        self.wr.set_high().unwrap();
    }

    pub fn set_text<const N: usize>(&mut self, text: &heapless::String<N>) {
        self.text.clear();
        self.text.push_str(text).unwrap();
    }

    pub fn write(&mut self, delay: &mut impl embedded_hal::delay::DelayNs) {
        let current_text = self.text.clone();

        let mut text = "";
        let mut empty_beginning = 0;
        if self.text_scroll_pos >= self.text.len() {
            let p = self.text_scroll_pos - self.text.len();
            text = &current_text[0..p];
            empty_beginning = 8 - p;
        } else {
            if self.text_scroll_pos + 8 > self.text.len() {
                text = &current_text[self.text_scroll_pos..];
            } else {
                text = &current_text[self.text_scroll_pos..self.text_scroll_pos + 8];
            }
        }

        for i in 0..empty_beginning {
            self.write_char(i as u8, ' ' as u8, delay);
        }
        for (i, &c) in text.as_bytes().iter().enumerate() {
            self.write_char((i + empty_beginning) as u8, c as u8, delay);
        }
        
        for i in (text.len() + empty_beginning)..8 {
            self.write_char(i as u8, ' ' as u8, delay);
        }

        // if self.text_scroll_pos + 8 > self.text.len() {
        //     text = &self.text[self.text_scroll_pos..];
        // } else if self.text_scroll_pos < self.text.len() {

        // } else {
        //     text = &self.text[self.text_scroll_pos..self.text_scroll_pos + 8];
        // }

        // let text = self.text[self.text_scroll_pos..self.text_scroll_pos+8].as_bytes();

    }

    pub fn scroll(&mut self, delay: &mut impl embedded_hal::delay::DelayNs) {
        self.text_scroll_pos += 1;
        if self.text_scroll_pos >= self.text.len() + 8 {
            self.text_scroll_pos = 0;
        }

        // 12345678 90000000 // 0
        // 23456789 00000000 // 1
        // 34567890 00000000 // 2
        // 45678900 00000000 // 3
        // 56789000 00000000 // 4
        // 67890000 00000000 // 5
        // 78900000 00000000 // 6
        // 89000000 00000000 // 7
        // 90000000 00000000 // 8
        // 00000000 00000000 // 9


        self.write(delay);
        delay.delay_ms(100);
    }
}

pub fn bool_to_pin_state(value: bool) -> PinState {
    match value {
        false => PinState::Low,
        _ => PinState::High,
    }
}

pub struct GpioBus5<P0, P1, P2, P3, P4>
where
    P0: embedded_hal::digital::OutputPin,
    P1: embedded_hal::digital::OutputPin,
    P2: embedded_hal::digital::OutputPin,
    P3: embedded_hal::digital::OutputPin,
    P4: embedded_hal::digital::OutputPin,
{
    pins: (P0, P1, P2, P3, P4),
}

impl<P0, P1, P2, P3, P4> GpioBus5<P0, P1, P2, P3, P4>
where
    P0: embedded_hal::digital::OutputPin,
    P1: embedded_hal::digital::OutputPin,
    P2: embedded_hal::digital::OutputPin,
    P3: embedded_hal::digital::OutputPin,
    P4: embedded_hal::digital::OutputPin,
{
    pub fn new(pins: (P0, P1, P2, P3, P4)) -> Self {
        Self { pins }
    }

    pub fn set(&mut self, value: u8) {
        self.pins
            .0
            .set_state(bool_to_pin_state(value & 0x01 != 0x00))
            .unwrap();
        self.pins
            .1
            .set_state(bool_to_pin_state(value & 0x02 != 0x00))
            .unwrap();
        self.pins
            .2
            .set_state(bool_to_pin_state(value & 0x04 != 0x00))
            .unwrap();
        self.pins
            .3
            .set_state(bool_to_pin_state(value & 0x08 != 0x00))
            .unwrap();
        self.pins
            .4
            .set_state(bool_to_pin_state(value & 0x10 != 0x00))
            .unwrap();
    }
}

pub struct GpioBus8<P0, P1, P2, P3, P4, P5, P6, P7>
where
    P0: embedded_hal::digital::OutputPin,
    P1: embedded_hal::digital::OutputPin,
    P2: embedded_hal::digital::OutputPin,
    P3: embedded_hal::digital::OutputPin,
    P4: embedded_hal::digital::OutputPin,
    P5: embedded_hal::digital::OutputPin,
    P6: embedded_hal::digital::OutputPin,
    P7: embedded_hal::digital::OutputPin,
{
    pins: (P0, P1, P2, P3, P4, P5, P6, P7),
}

impl<P0, P1, P2, P3, P4, P5, P6, P7> GpioBus8<P0, P1, P2, P3, P4, P5, P6, P7>
where
    P0: embedded_hal::digital::OutputPin,
    P1: embedded_hal::digital::OutputPin,
    P2: embedded_hal::digital::OutputPin,
    P3: embedded_hal::digital::OutputPin,
    P4: embedded_hal::digital::OutputPin,
    P5: embedded_hal::digital::OutputPin,
    P6: embedded_hal::digital::OutputPin,
    P7: embedded_hal::digital::OutputPin,
{
    pub fn new(pins: (P0, P1, P2, P3, P4, P5, P6, P7)) -> Self {
        Self { pins }
    }

    pub fn set(&mut self, value: u8) {
        self.pins
            .0
            .set_state(bool_to_pin_state(value & 0x01 != 0x00))
            .unwrap();
        self.pins
            .1
            .set_state(bool_to_pin_state(value & 0x02 != 0x00))
            .unwrap();
        self.pins
            .2
            .set_state(bool_to_pin_state(value & 0x04 != 0x00))
            .unwrap();
        self.pins
            .3
            .set_state(bool_to_pin_state(value & 0x08 != 0x00))
            .unwrap();
        self.pins
            .4
            .set_state(bool_to_pin_state(value & 0x10 != 0x00))
            .unwrap();
        self.pins
            .5
            .set_state(bool_to_pin_state(value & 0x20 != 0x00))
            .unwrap();
        self.pins
            .6
            .set_state(bool_to_pin_state(value & 0x40 != 0x00))
            .unwrap();
        self.pins
            .7
            .set_state(bool_to_pin_state(value & 0x80 != 0x00))
            .unwrap();
    }
}
