//! A basic implementation of the `micros()` function from Arduino:
//!
//!     https://www.arduino.cc/reference/en/language/functions/time/micros/
//!
#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_uno::prelude::*;
use core::cell;
use panic_halt as _;

// Possible Values:
//
// ╔═══════════╦══════════════╦═══════════════════╗
// ║ PRESCALER ║ TIMER_COUNTS ║ Overflow Interval ║
// ╠═══════════╬══════════════╬═══════════════════╣
// ║         8 ║            2 ║              1 us ║
// ║        64 ║          250 ║              1 ms ║
// ║       256 ║          125 ║              2 ms ║
// ║       256 ║          250 ║              4 ms ║
// ║      1024 ║          125 ║              8 ms ║
// ║      1024 ║          250 ║             16 ms ║
// ╚═══════════╩══════════════╩═══════════════════╝
const PRESCALER: u32 = 8;
const TIMER_COUNTS: u32 = 2;

const MICROS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16;

static MICROS_COUNTER: avr_device::interrupt::Mutex<cell::Cell<u32>> =
    avr_device::interrupt::Mutex::new(cell::Cell::new(0));

fn micros_init(tc0: arduino_uno::pac::TC0) {
    // Configure the timer for the above interval (in CTC mode)
    // and enable its interrupt.
    tc0.tccr0a.write(|w| w.wgm0().ctc());
    tc0.ocr0a.write(|w| unsafe { w.bits(TIMER_COUNTS as u8) });
    tc0.tccr0b.write(|w| match PRESCALER {
        8 => w.cs0().prescale_8(),
        64 => w.cs0().prescale_64(),
        256 => w.cs0().prescale_256(),
        1024 => w.cs0().prescale_1024(),
        _ => panic!(),
    });
    tc0.timsk0.write(|w| w.ocie0a().set_bit());

    // Reset the global millisecond counter
    avr_device::interrupt::free(|cs| {
        MICROS_COUNTER.borrow(cs).set(0);
    });
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter_cell = MICROS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MICROS_INCREMENT);
    })
}

fn micros() -> u32 {
    avr_device::interrupt::free(|cs| MICROS_COUNTER.borrow(cs).get())
}

#[arduino_uno::entry]
fn main() -> ! {
    let dp = arduino_uno::Peripherals::take().unwrap();

    let mut pins = arduino_uno::Pins::new(dp.PORTB, dp.PORTC, dp.PORTD);

    let mut serial = arduino_uno::Serial::new(
        dp.USART0,
        pins.d0,
        pins.d1.into_output(&mut pins.ddr),
        57600.into_baudrate(),
    );

    micros_init(dp.TC0);

    // Enable interrupts globally
    unsafe { avr_device::interrupt::enable() };

    // Wait for a character and print current time once it is received
    loop {
        let b = nb::block!(serial.read()).void_unwrap();

        let time = micros();
        ufmt::uwriteln!(&mut serial, "Got {} after {} us!\r", b, time).void_unwrap();
    }
}
