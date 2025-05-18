//! CDC-ACM serial port example using polling in a busy loop.
//! Target board: any STM32F4 with a OTG FS peripheral and a 25MHz HSE crystal
#![no_std]
#![no_main]

extern crate alloc;

mod circ_buff;
mod disp;

use kokolib::packet::{Command, Packet};

use core::cell::RefCell;

use circ_buff::CircBuff;
use cortex_m::interrupt::Mutex;
// use panic_halt as _;

use cortex_m_rt::entry;
use embedded_alloc::LlffHeap as Heap;
use stm32f4xx_hal::{
    dwt::DwtExt,
    gpio::{PinPull, Pull, Speed},
    interrupt,
    otg_fs::{UsbBus, UsbBusType, USB},
    pac::{self, Interrupt, TIM2},
    prelude::*,
    spi::{Mode, Phase, Polarity}, timer::{self, CounterUs},
};
use usb_device::{bus::UsbBusAllocator, prelude::*};
use usbd_serial::SerialPort;

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

// Endpoint memory for USB device
// USB needs a dedicated PMA (Packet Memory Area) region for endpoint memory.
static mut EP_MEMORY: [u32; 1024] = [0; 1024];

// Make USB serial device globally available
static G_USB_SERIAL: Mutex<RefCell<Option<SerialPort<UsbBus<USB>>>>> =
    Mutex::new(RefCell::new(None));

// Make USB device globally available
static G_USB_DEVICE: Mutex<RefCell<Option<UsbDevice<UsbBus<USB>>>>> =
    Mutex::new(RefCell::new(None));

const RECV_BUFFER_MAX_SIZE: usize = 14000;

static G_RECV_BUFFER: Mutex<RefCell<Option<CircBuff<u8, RECV_BUFFER_MAX_SIZE>>>> =
    Mutex::new(RefCell::new(None));

pub const SPI_MODE: Mode = Mode {
    phase: Phase::CaptureOnFirstTransition,
    polarity: Polarity::IdleLow,
};


#[global_allocator]
static HEAP: Heap = Heap::empty();


#[entry]
fn main() -> ! {
    // Initialise our debug printer.
    static mut USB_BUS: Option<UsbBusAllocator<stm32f4xx_hal::otg_fs::UsbBusType>> = None;
    // rtt_init_print!();
    rtt_init_print!(rtt_target::ChannelMode::BlockIfFull);
    // rtt_target::ChannelMode::BlockIfFull

    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
    }

    // Send a message back via the debugger.
    rprintln!("Initializing mon device!");

    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();

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
    
    let mut display = {
        let mut sck = gpiob.pb10.into_push_pull_output();
        sck.set_speed(Speed::Low);
        sck = sck.internal_resistor(Pull::Up);
    
        // let miso = gpio.pa6.internal_resistor(Pull::Down);
        let mut mosi = gpiob.pb15.into_push_pull_output();
        mosi.set_speed(Speed::Low);
        mosi = mosi.internal_resistor(Pull::Down);
    
        let mut cs = gpiob.pb14.into_push_pull_output();
        cs.set_speed(Speed::Low);
        cs = cs.internal_resistor(Pull::Down);

        let mut exci = gpioa.pa8.into_push_pull_output().internal_resistor(Pull::Down);
        let mut excm = gpioa.pa9.into_push_pull_output().internal_resistor(Pull::Down);
        let mut disp_pin = gpiob.pb12.into_push_pull_output().internal_resistor(Pull::Up);
        
        let delay = dp.TIM5.delay_us(&clocks);


    
        let spi = dp.SPI2.spi_bidi((sck, mosi), SPI_MODE, 4.MHz(), &clocks);
    
        disp::Display::new(spi, cs, exci, excm, disp_pin, delay)
    };

    display.clear();

    stopwatch.reset();

    display.send_buffer();

    stopwatch.lap();

    rprintln!("send_buffer: {} ms", stopwatch.lap_time(1).unwrap().as_millis());
    
    
    cortex_m::interrupt::free(|cs| *disp::DISPLAY.borrow(cs).borrow_mut() = Some(display));




    // let mut delay = dwt.delay();
    // delay.delay_ms(1000);

    let usb = USB::new(
        (dp.OTG_FS_GLOBAL, dp.OTG_FS_DEVICE, dp.OTG_FS_PWRCLK),
        (gpioa.pa11, gpioa.pa12),
        &clocks,
    );

    // Set up the USB bus
    *USB_BUS = Some(UsbBusType::new(usb, unsafe { &mut EP_MEMORY }));
    let usb_bus = USB_BUS.as_ref().unwrap();

    cortex_m::interrupt::free(|cs| {
        *G_USB_SERIAL.borrow(cs).borrow_mut() = Some(SerialPort::new(usb_bus));

        *G_USB_DEVICE.borrow(cs).borrow_mut() = Some(
            UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
                .device_class(usbd_serial::USB_CLASS_CDC)
                .strings(&[StringDescriptors::default()
                    .manufacturer("KokoroCorp")
                    .product("KokoroFun!")
                    .serial_number("001")])
                .unwrap()
                .build(),
        );

        *G_RECV_BUFFER.borrow(cs).borrow_mut() = Some(CircBuff::new());
    });

    // Set up timer 2 for VCOM toggling
    let mut timer = dp.TIM2.counter(&clocks);
    timer.start(100.millis()).unwrap();
    timer.listen(timer::Event::Update);

    stopwatch.reset();

    timer.start(100.millis()).unwrap();

    stopwatch.lap();

    let duration = stopwatch.lap_time(1).unwrap();

    rprintln!("Delay: {:?} ns", duration.as_nanos());


    cortex_m::interrupt::free(|cs| *disp::G_TIM_VCOM.borrow(cs).borrow_mut() = Some(timer));

    // Enable interrupts
    unsafe {
        cortex_m::peripheral::NVIC::unmask(Interrupt::OTG_FS);
        cortex_m::peripheral::NVIC::unmask(Interrupt::TIM2);
    }

    let mut packet: Option<Packet> = None;

    let mut raw_packet = [0x00u8; 512];
    let mut raw_packet_size = 0;

    #[allow(clippy::empty_loop)]
    loop {
        // Do nothing. Everything is done in the IRQ handler
        cortex_m::interrupt::free(|cs| {
            // Move USB serial device here, leaving a None in its place

            let mut recv_buffer = G_RECV_BUFFER.borrow(cs).borrow_mut();

            if recv_buffer.is_none() {
                rprintln!("No serial device in loop!");
                return;
            }

            let recv_buffer = recv_buffer.as_mut().unwrap();
            while recv_buffer.size() > 0 && packet.is_none() {
                raw_packet[raw_packet_size] = recv_buffer.pop().unwrap();
                raw_packet_size += 1;

                if raw_packet[raw_packet_size - 1] == 0x00 {
                    if let Ok(p) = Packet::from_cobs(&raw_packet[0..raw_packet_size]) {
                        packet = Some(p);
                    } else {
                        rprintln!("Invalid packet!");
                    }
                }
            }

        });
        if let Some(p) = packet {
            // rprintln!("Packet: {:?}", p);
            packet = None;
            raw_packet_size = 0;

            // Do something with the packet
            match Command::from(p.command()) {
                Command::CMD_SCREEN_BUFFER => {
                    // rprintln!("SCREEN_BUFFER");
                    rprintln!("Packet data: {:?}", p.get_payload());
                }
                Command::CMD_ACK => {
                    rprintln!("ACK");
                }
                _ => {
                    rprintln!("Unknown command");
                }
            }
        }
    }
}

// KokoroFun! has different modes, each packet is at most 255 bytes long
// first byte is the mode, second byte is the length of the packet
// the rest is the data

#[interrupt]
fn OTG_FS() {
    static mut USB_SERIAL: Option<SerialPort<UsbBus<USB>>> = None;
    static mut USB_DEVICE: Option<UsbDevice<UsbBus<USB>>> = None;
    // static mut RECV_BUFFER: Option<CircBuff<u8, RECV_BUFFER_MAX_SIZE>> = None;

    let usb_dev = USB_DEVICE.get_or_insert_with(|| {
        cortex_m::interrupt::free(|cs| {
            // Move USB device here, leaving a None in its place
            G_USB_DEVICE.borrow(cs).replace(None).unwrap()
        })
    });

    let serial = USB_SERIAL.get_or_insert_with(|| {
        cortex_m::interrupt::free(|cs| {
            // Move USB serial device here, leaving a None in its place
            G_USB_SERIAL.borrow(cs).replace(None).unwrap()
        })
    });


    if usb_dev.poll(&mut [serial]) {
        cortex_m::interrupt::free(|cs| {

            let mut recv_buffer = G_RECV_BUFFER.borrow(cs).borrow_mut();

            if recv_buffer.is_none() {
                rprintln!("No serial device!");
                return;
            }
            let recv_buffer = recv_buffer.as_mut().unwrap();

            let mut buf = [0u8; 64];

            if recv_buffer.remaining() < 64 {
                return;
            }

            match serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    for i in 0..count {
                        recv_buffer.push(buf[i]);
                    }
                }
                _ => {}
            }
        });
    }
}
