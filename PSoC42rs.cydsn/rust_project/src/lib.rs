#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![no_std]
// #![cfg_attr(not(target_arch = "arm"), allow(unused_variables))]
//pub
pub mod ffi;
use crate::ffi::*;
pub mod Config;
pub mod encoder;
pub mod motor;
pub mod serial;
pub mod sys;
pub mod ui;
pub mod utils;
// use cortex_m_rt::entry;

use crate::encoder::*;
use crate::motor::*;
use crate::Config::*;
// use core::sync::atomic::AtomicU8;
use local_static::LocalStatic;
use serial::*;
use sys::*;
use ui::*;
static SYS: LocalStatic<System_T> = LocalStatic::new();
static Xaxis: LocalStatic<Stepper<XEncoder>> = LocalStatic::new();

// static MS_TICK: LocalStatic<u32> = LocalStatic::new();
// ... SysTick() and systick_handler ...
#[unsafe(no_mangle)]
pub extern "C" fn tick_callback() {}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Optionally log the panic location to a simple UART print if you have one,
    // otherwise, just halt.
    // Halt the CPU forever
    uart_printf(format_args!("Panic!->"));
    uart_printf(format_args!(
        "line {} file{}\n\r",
        info.location().unwrap().line(),
        info.location().unwrap().file()
    ));
    loop {
        cortex_m::asm::bkpt(); // Use a breakpoint instruction to halt
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> () {
    *SYS.get_mut() = System_T::new();
    let mut led = LED_CTRL::new();
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

    uart_put_str("Initialized PSoCcmake.");
    let mut print_cnt: u32 = 0;
    let mut last_upd: u32 = 0;
    let mut enc_last_upd: u32 = 0;
    let mut enc_last_dt: u32 = 0;

    let mut old_count: i16 = -1;
    let mut spd_ref: u16;

    loop {
        let now = unsafe { CySysTickGetValue() };
        // duration in ticks between blinks
        if last_upd.wrapping_sub(now) >= 10 {
            led.led_task();
            btn.update();
            SYS.get_mut().sys_task();
            last_upd = now;
        }
        Xaxis.get_mut().encoder.read_counter();

        // if enc_last_dt != enc_last_upd.wrapping_sub(now) {
        enc_last_dt = enc_last_upd.wrapping_sub(now);
        // uart_printf(format_args!("dt: {}\n\r", enc_last_dt));
        // }
        Xaxis.get_mut().encoder.update(enc_last_dt);
        enc_last_upd = now;
        print_cnt += 1;
        if print_cnt > 150 {
            print_cnt = 0;
            if Xaxis.get().state != MotorState::IDLE {
                uart_printf(format_args!(
                    "\n\r{},{}",
                    Xaxis.get().encoder.alpha,
                    // Xaxis.get().encoder.prev_enc_counts,
                    Xaxis.get().encoder.omega // Xaxis.get().encoder.prev_enc_counts
                ));
            } else {
            }
        }
        unsafe {
            let mut count = ADC_SAR_Seq_GetResult16(0);
            if count <= 0 {
                count = 1;
            }
            if (count - old_count).saturating_abs() > 10 {
                spd_ref = ADC_SAR_Seq_CountsTo_mVolts(0, count).saturating_abs() as u16;
                // spd_ref = count as u32;
                Xaxis.get_mut().set_speed(spd_ref as u32 * 4);
                uart_printf(format_args!("SPD:{}\n\r", spd_ref));

                old_count = count;
            }
        }
        // print_cnt += 1;
        // if print_cnt > 5000 {
        //     print_cnt = 0;
        // }
        // enc_last_upd += 1;

        // uart_printf(format_args!("Encoder: {}\n\r", enc.curr));

        if btn.is_pressed() {
            uart_printf(format_args!("Pressed!\n\r"));
        }
        if btn.is_held() {
            uart_printf(format_args!("Held!\n\r"));
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
