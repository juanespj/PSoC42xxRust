#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![no_std]
mod motor;
mod serial;
mod sys;
mod ui;
use cortex_m_rt::entry;
pub mod ffi;
use crate::motor::{read_hw_counter, Encoder};
use ffi::*;
use serial::*;
use sys::*;
use ui::*;

use cortex_m::Peripherals;
use embassy_executor::Executor;
use embassy_executor::Spawner;
use embassy_time::Timer;
use static_cell::StaticCell;
use systick_timer::SystickDriver;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();
// ... SysTick() and systick_handler ...
embassy_time_driver::time_driver_impl!(
    static DRIVER: SystickDriver<4> = SystickDriver::new(24_000_000, 24)
);

#[no_mangle]
pub extern "C" fn tick_callback() {
    // DRIVER.systick_interrupt();
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Optionally log the panic location to a simple UART print if you have one,
    // otherwise, just halt.
    // Halt the CPU forever
    unsafe {
        UART_UartPutString("\n\rPanic!".as_ptr());
    }

    loop {
        cortex_m::asm::bkpt(); // Use a breakpoint instruction to halt
    }
}

#[entry]
fn main() -> ! {
    UI_init();
    unsafe {
        let mut cp = Peripherals::steal();
        DRIVER.start(&mut cp.SYST);
        CySysTickInit();
        CySysTickStart();
        CySysTickEnable();
        CySysTickSetReload(CYDEV_BCLK__SYSCLK__HZ / 10);

        CySysTickSetCallback(0, Some(tick_callback));
        CySysTickClear();
        core::arch::asm!("cpsie i"); //same as CyGlobalIntEnable();
        uart_printf(format_args!("Initialized Embassy PSoC."));
    }
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner: Spawner| {
        // spawn tasks inside the closure
        // spawner.spawn(blink()).unwrap();
        spawner.spawn(main_task()).unwrap();
        // UART_PutString("\n\r--- Blinkyr Started ---".as_ptr());

        //    spawner.spawn(uartrx(psoc5uart)).unwrap();
    });
}

#[embassy_executor::task]
async fn main_task() {
    let mut enc = Encoder::default();
    enc.init();
    let gpio_pin = || unsafe { BTN_Read() == 0 }; //change polarity if needed
    let mut btn = DebouncedButton::new(gpio_pin);
    // UART_SetCustomInterruptHandler(Some(UARTRX));

    let mut last_upd: u32 = 0;
    let mut sys = System_T::new();
    loop {
        led_task();
        let now = unsafe { CySysTickGetValue() };
        // duration in ticks between blinks
        if last_upd.wrapping_sub(now) >= 1_000 {
            last_upd = now;

            btn.update();
        }
        sys.sys_task();
        if enc.update(read_hw_counter) {
            let pos = enc.position();
            uart_printf(format_args!("Encoder: {}\n\r", pos));
        }
        if btn.is_pressed() {
            uart_printf(format_args!("Button Pressed!\n\r"));
        }
        if btn.is_held() {
            uart_printf(format_args!("Button Held!\n\r"));
        }
    }
}

//**  --- EMBASSY ---
//
//
// use core::future::poll_fn;
// use core::panic::PanicInfo;
// use cortex_m::peripheral::SCB;
// use core::fmt::Write; // Required for the write! macro
// Define a static signal for the event.
// The type '()' means the signal is just a notification, no data is passed.
// static RX_WAKER: Signal<RefCell, ()> = Signal::new();
// Define a capacity constant for your string buffer (e.g., 128 bytes)

// use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering}; //AtomicU32,

// use cortex_m::Peripherals;
// use embassy_executor::Executor;
// use embassy_executor::Spawner;
// use embassy_sync::signal::Signal;
// use embassy_time::Timer;
// use static_cell::StaticCell;
// use systick_timer::SystickDriver;
// static EXECUTOR: StaticCell<Executor> = StaticCell::new();
// In your interrupt handler file (e.g., in src/interrupts.rs)
// embassy_time_driver::time_driver_impl!(
// static DRIVER: SystickDriver<4> = SystickDriver::new(24_000_000, 1000)
// );
//
// let mut cp = Peripherals::steal();
// This call will configure the reload value (24000 based on your DRIVER definition),
// enable the timer, and set the priority correctly for the Embassy runtime.
// DRIVER.start(&mut cp.SYST);
// let executor = EXECUTOR.init(Executor::new());
// executor.run(|spawner: Spawner| {
//     spawn tasks inside the closure
//     spawner.spawn(blink()).unwrap();
// spawner.spawn(uart_rx_task()).unwrap();
// });
// #[embassy_executor::task]
// async fn blink() {
//     loop {
//         unsafe {
//             LED_Write(0);
//         }
//         Timer::after_millis(150).await;
//         unsafe {
//             LED_Write(1);
//         }
//         Timer::after_millis(300).await;
//     }
// }
// static mut COUNT: u8 = 0; */
