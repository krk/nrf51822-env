#![no_std]
#![deny(warnings)]
#![feature(const_fn)]
extern crate cortex_m;
// extern crate cortex_m_rt; // included in the device crate
extern crate cortex_m_semihosting;

#[macro_use(exception, interrupt)]
extern crate nrf51822;

extern crate panic_abort; // panicking behavior

use core::cell::RefCell;
use core::fmt::Write;

use cortex_m::interrupt::{self, Mutex};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_semihosting::hio::{self, HStdout};
use nrf51822::Interrupt;

static HSTDOUT: Mutex<RefCell<Option<HStdout>>> = Mutex::new(RefCell::new(None));

static NVIC: Mutex<RefCell<Option<cortex_m::peripheral::NVIC>>> = Mutex::new(RefCell::new(None));

fn main() {
    // DO NOT try to flash this program, it only compiles.
    let global_p = cortex_m::Peripherals::take().unwrap();
    interrupt::free(|cs| {
        let hstdout = HSTDOUT.borrow(cs);
        if let Ok(fd) = hio::hstdout() {
            *hstdout.borrow_mut() = Some(fd);
        }

        let mut nvic = global_p.NVIC;
        nvic.enable(Interrupt::TIMER0);
        *NVIC.borrow(cs).borrow_mut() = Some(nvic);

        let mut syst = global_p.SYST;
        syst.set_clock_source(SystClkSource::Core);
        syst.set_reload(8_000_000); // 1s
        syst.enable_counter();
        syst.enable_interrupt();
    });
}

exception!(SYS_TICK, tick);

fn tick() {
    interrupt::free(|cs| {
        let hstdout = HSTDOUT.borrow(cs);
        if let Some(hstdout) = hstdout.borrow_mut().as_mut() {
            writeln!(*hstdout, "Tick").ok();
        }

        if let Some(nvic) = NVIC.borrow(cs).borrow_mut().as_mut() {
            nvic.set_pending(Interrupt::TIMER1);
        }
    });
}

interrupt!(TIMER1, tock, locals: {
     tocks: u32 = 0;
 });

fn tock(l: &mut TIMER1::Locals) {
    l.tocks += 1;

    interrupt::free(|cs| {
        let hstdout = HSTDOUT.borrow(cs);
        if let Some(hstdout) = hstdout.borrow_mut().as_mut() {
            writeln!(*hstdout, "Tock ({})", l.tocks).ok();
        }
    });
}
