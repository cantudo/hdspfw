use core::cell::RefCell;

use panic_halt as _;
use cortex_m::interrupt::Mutex;
use rp235x_hal as hal;
use rp235x_hal::clocks::UsbClock;
use rp235x_hal::{pac, pac::interrupt, usb::UsbBus};
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

use hdsplib::circ_buff::CircBuff;

type MutRefOption<T> = Mutex<RefCell<Option<T>>>;

static G_USB_SERIAL: MutRefOption<SerialPort<UsbBus>> = Mutex::new(RefCell::new(None));

static G_USB_DEVICE: MutRefOption<UsbDevice<UsbBus>> = Mutex::new(RefCell::new(None));

const RECV_BUFFER_MAX_SIZE: usize = 14000;

static G_RECV_BUFFER: MutRefOption<CircBuff<u8, RECV_BUFFER_MAX_SIZE>> =
    Mutex::new(RefCell::new(None));

pub fn init(
    pac_usb: pac::USB,
    pac_usb_dpram: pac::USB_DPRAM,
    usb_clock: UsbClock,
    pac_resets: &mut pac::RESETS,
) {
    static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

    unsafe {
        USB_BUS = Some(UsbBusAllocator::new(hal::usb::UsbBus::new(
            pac_usb,
            pac_usb_dpram,
            usb_clock,
            true,
            pac_resets,
        )));
    }

    let usb_bus = unsafe { USB_BUS.as_ref().unwrap() };

    let serial = SerialPort::new(&usb_bus);

    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("KokoroKorp")
            .product("Kokoro FUN!")
            .serial_number("001")])
        .unwrap()
        .device_class(2) // from: https://www.usb.org/defined-class-codes
        .build();

    cortex_m::interrupt::free(|cs| {
        *G_USB_SERIAL.borrow(cs).borrow_mut() = Some(serial);

        *G_USB_DEVICE.borrow(cs).borrow_mut() = Some(usb_dev);

        *G_RECV_BUFFER.borrow(cs).borrow_mut() = Some(CircBuff::new());
    });
}

#[interrupt]
fn USBCTRL_IRQ() {
    static mut USB_SERIAL: Option<SerialPort<UsbBus>> = None;
    static mut USB_DEVICE: Option<UsbDevice<UsbBus>> = None;

    // On the first execution of the function, USB_DEVICE is none, and gets populated
    // with the value from the global variable. On subsequent runs it is populated.
    let usb_dev = unsafe {
        USB_DEVICE.get_or_insert_with(|| {
            cortex_m::interrupt::free(|cs| G_USB_DEVICE.borrow(cs).replace(None).unwrap())
        })
    };

    let serial = unsafe {
        USB_SERIAL.get_or_insert_with(|| {
            cortex_m::interrupt::free(|cs| G_USB_SERIAL.borrow(cs).replace(None).unwrap())
        })
    };

    // println

    if usb_dev.poll(&mut [serial]) {
        cortex_m::interrupt::free(|cs| {
            let mut recv_buffer = G_RECV_BUFFER.borrow(cs).borrow_mut();

            if recv_buffer.is_none() {
                // println
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
