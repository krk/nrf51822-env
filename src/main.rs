#![no_std]
#![deny(warnings)]
#![feature(const_fn)]
extern crate cortex_m;
extern crate cortex_m_semihosting;

#[macro_use(interrupt)]
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

static NRFP: Mutex<RefCell<Option<nrf51822::Peripherals>>> = Mutex::new(RefCell::new(None));

fn main() {
    let global_p = cortex_m::Peripherals::take().unwrap();
    let nrf_p = nrf51822::Peripherals::take().unwrap();

    interrupt::free(|cs| {
        let hstdout = HSTDOUT.borrow(cs);
        if let Ok(fd) = hio::hstdout() {
            *hstdout.borrow_mut() = Some(fd);
        }

        {
            let timer1: &nrf51822::timer0::RegisterBlock = &*nrf_p.TIMER1;

            timer1.mode.write(|w| w.mode().timer());
            timer1.tasks_clear.write(|w| unsafe { w.bits(1) });
            timer1.prescaler.write(|w| unsafe { w.bits(15) });

            // TODO TIMER doesn'T seem to be affected/initialized correctly.
            timer1.cc[0].write(|w| unsafe { w.bits(1000) });
            timer1.intenset.write(|w| w.compare0().set());
            timer1.tasks_start.write(|w| unsafe { w.bits(1) });

            let gpio = &*nrf_p.GPIO;
            let pin19_cnf = &gpio.pin_cnf[19];

            pin19_cnf.write(|w| {
                w.dir()
                    .output()
                    .drive()
                    .s0d1()
                    .pull()
                    .disabled()
                    .sense()
                    .disabled()
                    .input()
                    .disconnect()
            });
        }

        *NRFP.borrow(cs).borrow_mut() = Some(nrf_p);

        let mut nvic = global_p.NVIC;
        nvic.enable(Interrupt::TIMER1);
        *NVIC.borrow(cs).borrow_mut() = Some(nvic);

        let mut syst = global_p.SYST;
        syst.set_clock_source(SystClkSource::Core);
        syst.set_reload(16_000_000); // 1s
        syst.enable_counter();
        syst.enable_interrupt();
    });
}

interrupt!(TIMER1, tock, locals: {
     tocks: u32 = 0;
     led_state: bool = false;
 });

fn tock(l: &mut TIMER1::Locals) {
    l.tocks += 1;

    interrupt::free(|cs| {
        let hstdout = HSTDOUT.borrow(cs);

        let nrfp = NRFP.borrow(cs);
        if let Some(nrf_p) = nrfp.borrow_mut().as_mut() {
            let gpio = &*nrf_p.GPIO;

            let pin19 = gpio.out.read().pin19().bit();

            if let Some(hstdout) = hstdout.borrow_mut().as_mut() {
                writeln!(*hstdout, "{}", pin19).ok();
            }

            if l.led_state {
                gpio.out.write(|w| w.pin19().clear_bit());
                l.led_state = false;
            } else {
                gpio.out.write(|w| w.pin19().set_bit());
                l.led_state = true;
            }
        }
    });
}
