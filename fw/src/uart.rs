use core::fmt::Write;
use hal::{
    clocks::{Clock, ClocksManager},
    fugit::RateExtU32,
    pac,
    uart::{DataBits, StopBits, UartConfig},
};
use panic_halt as _;
use rp235x_hal as hal;

// rp235x_hal::gpio::Pin<rp235x_hal::gpio::bank0::Gpio3, rp235x_hal::gpio::FunctionNull, rp235x_hal::gpio::PullDown>

pub fn init(
    gpio32: hal::gpio::Pin<hal::gpio::bank0::Gpio32, hal::gpio::FunctionNull, hal::gpio::PullDown>,
    gpio33: hal::gpio::Pin<hal::gpio::bank0::Gpio33, hal::gpio::FunctionNull, hal::gpio::PullDown>,
    uart0: pac::UART0,
    resets: &mut pac::RESETS,
    clocks: &ClocksManager,
) {
    let uart0_pins = (
        // UART TX (characters sent from rp235x) on pin 4 (GPIO2) in Aux mode
        gpio32.into_function(),
        // UART RX (characters received by rp235x) on pin 5 (GPIO3) in Aux mode
        gpio33.into_function(),
    );

    let mut uart0 = hal::uart::UartPeripheral::new(uart0, uart0_pins, resets)
        .enable(
            UartConfig::new(115200u32.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    writeln!(uart0, "Hello!");
}
