#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(not(target_arch = "arm"), allow(unused_variables))]

pub mod ffi;
mod motor;
mod serial;
mod sys;
mod ui;
use ffi::*;

use crate::motor::*;
use local_static::LocalStatic;
use serial::*;
use sys::*;
use ui::*;

static SYS: LocalStatic<System_T> = LocalStatic::new();
static Xaxis: LocalStatic<Stepper<XEncoder>> = LocalStatic::new();
static Yaxis: LocalStatic<Stepper<YEncoder>> = LocalStatic::new();
static Zaxis: LocalStatic<Stepper<ZEncoder>> = LocalStatic::new();

// ... SysTick() and systick_handler ...
#[no_mangle]
pub extern "C" fn tick_callback() {
    // *MS_TICK.get_mut() = MS_TICK.get().wrapping_add(1);
}
#[cfg(target_arch = "arm")]
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
#[cfg(target_arch = "arm")]
#[no_mangle]
pub extern "C" fn main() -> () {
    *SYS.get_mut() = System_T::new();
    // *Yaxis.get_mut() = Stepper::new(Box::new(RightEncoder));
    // *Zaxis.get_mut() = Stepper::new(Box::new(LeftEncoder));
    *Xaxis.get_mut() = Stepper::new(XEncoder, 0);
    UI_init();

    unsafe {
        CySysTickInit();
        CySysTickStart();
        CySysTickEnable();
        CySysTickSetReload(CYDEV_BCLK__SYSCLK__HZ / 10);

        CySysTickSetCallback(0, Some(tick_callback));
        CySysTickClear();
        core::arch::asm!("cpsie i"); //same as CyGlobalIntEnable();
        IDAC_SetValue(306);
        IDAC_Start();
        ADC_SAR_Seq_Start();
        ADC_SAR_Seq_StartConvert();
    }
    pulser_init();

    let gpio_pin = || unsafe { BTN_Read() == 0 }; //change polarity if needed
    let mut btn = DebouncedButton::new(gpio_pin);
    // UART_SetCustomInterruptHandler(Some(UARTRX));
    uart_put_str("Initialized PSoCcmake.");

    let mut last_upd: u32 = 0;
    let mut old_count: i16 = 0;
    let mut spd_ref: u16;

    loop {
        let now = unsafe { CySysTickGetValue() };
        // duration in ticks between blinks
        if last_upd.wrapping_sub(now) >= 50 {
            last_upd = now;
            led_task();
            //  btn.update();
            SYS.get_mut().sys_task();
        }
        //wfi or sleep?

        unsafe {
            let mut count = ADC_SAR_Seq_GetResult16(0);
            if count <= 0 {
                count = 1;
            }
            if (count - old_count).saturating_abs() > 10 {
                spd_ref = ADC_SAR_Seq_CountsTo_mVolts(0, count).saturating_abs() as u16;
                // spd_ref = count as u32;
                Xaxis.get_mut().set_speed(spd_ref as u32);
                uart_printf(format_args!("SPD:{}\n\r", spd_ref));

                old_count = count;
            }
        }

        // enc.read_counter();
        // uart_printf(format_args!("Encoder: {}\n\r", enc.curr));

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
