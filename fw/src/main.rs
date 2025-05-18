#![no_std]
#![no_main]

use embedded_hal::{delay::DelayNs, digital::{OutputPin, StatefulOutputPin}};
use hal::fugit::*;
use panic_halt as _;
use rp235x_hal::{self as hal, pio::PIOExt, Clock};
use rtt_target::{rprintln, rtt_init_print};
// use rp235x_hal::pac::Interrupt;
// use hal::fugit::RateExtU32;

mod disp;
mod uart;
mod usb;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[hal::entry]
fn main() -> ! {

    rtt_init_print!(rtt_target::ChannelMode::BlockIfFull);                                                                         
    rprintln!("Initializing mon device!");       
    

    let mut pac = hal::pac::Peripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);
    // Set the pins to their default state
    let pins: rp235x_hal::gpio::Pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut delay = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    uart::init(
        pins.gpio32,
        pins.gpio33,
        pac.UART0,
        &mut pac.RESETS,
        &clocks,
    );
    usb::init(pac.USB, pac.USB_DPRAM, clocks.usb_clock, &mut pac.RESETS);

    // RST: 16
    // FL: 17
    // A0: 18
    // A1: 19
    // A2: 20
    // A3: 21
    // A4: 22
    // CLS: 26
    // CLK: 27
    // WR: 28
    // CE: 15

    // RD: 13
    // D0: 12
    // D1: 11
    // D2: 10
    // D3: 9
    // D4: 8
    // D5: 7
    // D6: 6
    // D7: 5

    let rst = pins.gpio16.into_push_pull_output();
    let fl = pins.gpio17.into_push_pull_output();
    let a0 = pins.gpio18.into_push_pull_output();
    let a1 = pins.gpio19.into_push_pull_output();
    let a2 = pins.gpio20.into_push_pull_output();
    let a3 = pins.gpio21.into_push_pull_output();
    let a4 = pins.gpio22.into_push_pull_output();
    let cls = pins.gpio26.into_push_pull_output();
    let clk = pins.gpio27.into_push_pull_output();
    let wr = pins.gpio28.into_push_pull_output();
    let ce = pins.gpio15.into_push_pull_output();
    let rd = pins.gpio13.into_push_pull_output();
    let d0 = pins.gpio12.into_push_pull_output();
    let d1 = pins.gpio11.into_push_pull_output();
    let d2 = pins.gpio10.into_push_pull_output();
    let d3 = pins.gpio9.into_push_pull_output();
    let d4 = pins.gpio8.into_push_pull_output();
    let d5 = pins.gpio7.into_push_pull_output();
    let d6 = pins.gpio6.into_push_pull_output();
    let d7 = pins.gpio5.into_push_pull_output();

    let mut display = disp::Display::new(rst, fl, a0, a1, a2, a3, a4, cls, clk, wr, ce, rd, d0, d1, d2, d3, d4, d5, d6, d7);
    display.init(&mut delay);
    // display.write_char(2, 0x0, &mut delay);
    // display.write_text("hola?", &mut delay);
    // display.set_text("Hello, my name is Uru!");
    // display.set_text("123456789");
    // display.set_text("Bohemia Jazz Cafe, Plaza de los Lobos 11");
    display.set_text("I can see clearly now, the rain is gone.");

    display.write(&mut delay);
    delay.delay_ms(150);


    loop {
        display.scroll(&mut delay);
        delay.delay_ms(50);
    }

}


/// Program metadata for `picotool info`
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"HDSP clock firmware"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];

// End of file
