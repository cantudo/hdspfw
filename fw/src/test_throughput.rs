//! CDC-ACM serial port example using polling in a busy loop.
//! Target board: any STM32F4 with a OTG FS peripheral and a 25MHz HSE crystal
#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;
use stm32f4xx_hal::dwt::DwtExt;
use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use stm32f4xx_hal::pac::dma1::st;
use stm32f4xx_hal::timer::SysCounter;
use stm32f4xx_hal::{pac, prelude::*};
use usb_device::prelude::*;

use rtt_target::{rtt_init_print, rprintln};
// use panic_rtt_target as _;

static mut EP_MEMORY: [u32; 1024] = [0; 1024];

#[entry]
fn main() -> ! {
    // Initialise our debug printer.
    rtt_init_print!();
    // Send a message back via the debugger.
    rprintln!("Initializing mon device!");

    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();


    let rcc = dp.RCC.constrain();

    let clocks = rcc
        .cfgr
        .use_hse(25.MHz())
        .sysclk(48.MHz())
        .require_pll48clk()
        .freeze();

    let dwt = cp.DWT.constrain(cp.DCB, &clocks);
    
    let mut times = [0; 10];
    let mut stopwatch = dwt.stopwatch(&mut times);

    stopwatch.reset();

    let mut delay = dwt.delay();
    
    
    delay.delay_ms(1000);
    
    
    stopwatch.lap();
    
    let duration = stopwatch.lap_time(1).unwrap();

    rprintln!("Delay: {:?} ms", duration.as_millis());



    let gpioa = dp.GPIOA.split();

    let usb = USB::new(
        (dp.OTG_FS_GLOBAL, dp.OTG_FS_DEVICE, dp.OTG_FS_PWRCLK),
        (gpioa.pa11, gpioa.pa12),
        &clocks,
    );

    let usb_bus = UsbBus::new(usb, unsafe { &mut EP_MEMORY });

    let mut serial = usbd_serial::SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .device_class(usbd_serial::USB_CLASS_CDC)
        .max_packet_size_0(64).unwrap()
        .strings(&[StringDescriptors::default()
            .manufacturer("Fake Company")
            .product("Product")
            .serial_number("TEST")])
        .unwrap()
        .build();

    let mut n = 0;
    const num_bytes: usize = 10_000;
    let data = [0u8; num_bytes];
    let n_to_send = 100_000;
    stopwatch.reset();
    loop {
        if !usb_dev.poll(&mut [&mut serial]) {
            continue;
        }


        match serial.write(&data) {
            Ok(n_written) => {
                n += n_written;
                if n > n_to_send {
                    stopwatch.lap();
                    let duration = stopwatch.lap_time(1).unwrap();
                    let duration_s = duration.as_secs_f32();
                    rprintln!("Throughput: {} Mbps", (n as f32 / duration_s / 1_000_000.0) * 8.0);
                    n = 0;
                    stopwatch.reset();
                }
                // rprintln!("Wrote data {}", n);
            }
            Err(e) => {}//rprintln!("Error writing data: {:?}", e),
            
        }

        


        // rprintln!("data\n");
        // match serial.write(b"Hello World\n") {
        //     Ok(_) => rprintln!("Wrote data {}", n),
        //     Err(e) => rprintln!("Error writing data: {:?}", e),
        // }

        // let mut buf = [0u8; 64];

        // match serial.read(&mut buf) {
        //     Ok(count) if count > 0 => {
        //         match serial.write(b"Hello World\n") {
        //             Ok(_) => rprintln!("Wrote data"),
        //             Err(e) => rprintln!("Error writing data: {:?}", e),
        //         }
        //         // // Echo back in upper case
        //         // for c in buf[0..count].iter_mut() {
        //         //     if 0x61 <= *c && *c <= 0x7a {
        //         //         *c &= !0x20;
        //         //     }
        //         // }

        //         // let mut write_offset = 0;
        //         // while write_offset < count {
        //         //     match serial.write(&buf[write_offset..count]) {
        //         //         Ok(len) if len > 0 => {
        //         //             write_offset += len;
        //         //         }
        //         //         _ => {}
        //         //     }
        //         // }
        //     }
        //     _ => {}
        // }
    }
}
