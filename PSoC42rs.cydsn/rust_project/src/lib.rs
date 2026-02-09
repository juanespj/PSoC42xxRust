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
// use core::sync::atomic::AtomicU8;
use local_static::LocalStatic;
use serial::*;

use sys::*;
use ui::*;
static SYS: LocalStatic<System_T> = LocalStatic::new();
// static UART: LocalStatic<Uart> = LocalStatic::new();

static Xaxis: LocalStatic<Stepper<XEncoder>> = LocalStatic::new();

// static MS_TICK: LocalStatic<u32> = LocalStatic::new();
// ... SysTick() and systick_handler ...
#[unsafe(no_mangle)]
pub extern "C" fn tick_callback() {}

#[cfg(not(target_os = "windows"))]
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
const RELOAD: u32 = 2_400_000;
#[unsafe(no_mangle)]
pub extern "C" fn main() -> () {
    *SYS.get_mut() = System_T::new();
    let mut led = LED_CTRL::new();
    *Xaxis.get_mut() = Stepper::new(XEncoder, 0);
    // *UART.get_mut() = Uart::new();
    // UART.get_mut().
    UI_init();
    unsafe {
        CySysTickInit();
        CySysTickStart();
        CySysTickEnable();
        CySysTickSetReload(RELOAD);

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
    // if btn.is_pressed() {
    //     uart_printf(format_args!("Pressed!\n\r"));
    // }
    // if btn.is_held() {
    //     uart_printf(format_args!("Held!\n\r"));
    // }
    uart_put_str("Initialized PSoCcmake.");
    let mut print_cnt: u32 = 0;
    let mut upd_task: u32 = 0;
    let mut enc_last_upd: u32 = 0;

    let mut old_count: i16 = -1;
    let mut spd_ref: u16;

    loop {
        let now = unsafe { CySysTickGetValue() };
        // duration in ticks between blinks
        let dt = if now <= enc_last_upd {
            // Normal case: counts down from 100 to 80 (dt = 20)
            enc_last_upd - now
        } else {
            // Wrap case: last was 10, now is 23,990
            // distance is (10 - 0) + (reload - 23,990)
            enc_last_upd + (RELOAD - now)
        };
        if dt > 400 {
            //highest priority
            enc_last_upd = now;
            // unsafe { LED_Write(1) }
            Xaxis.get_mut().encoder.read_counter();
            Xaxis.get_mut().encoder.update(dt);
            // unsafe { LED_Write(0) }
            if print_cnt > 3 {
                print_cnt = 0;
                if Xaxis.get().state != MotorState::IDLE {
                    // uart_printf(format_args!("{},", Xaxis.get().encoder.omega));
                    // uart_send_i32f32_scaled(Xaxis.get().encoder.omega);
                    uart_send_u32_decimal(dt);
                    uart_put_tx(b',' as u32)
                }
            }
            print_cnt += 1;
        }
        if dt > 100 && upd_task == 0 {
            led.led_task();
            btn.update();
            upd_task += 1;
        }
        if dt > 110 && upd_task == 1 {
            SYS.get_mut().sys_task();
            upd_task += 1;
        }
        if dt > 120 && upd_task == 2 {
            unsafe {
                let mut count = ADC_SAR_Seq_GetResult16(0);
                if count <= 0 {
                    count = 1;
                }
                if (count - old_count).saturating_abs() > 10 {
                    spd_ref = ADC_SAR_Seq_CountsTo_mVolts(0, count).saturating_abs() as u16;
                    // spd_ref = count as u32;
                    Xaxis.get_mut().set_speed(spd_ref as u32 * 4);
                    // uart_printf(format_args!("SPD:{}\n\r", spd_ref));

                    old_count = count;
                }
            }
            upd_task = 0;
        }
    }
}
