// Only compile this file if we are NOT building for ARM (i.e., we are on the PC)

use core::ffi::c_char;
use local_static::LocalStatic;
// This file only compiles when NOT building for the ARM target (i.e., your host PC).

// Define mock functions for all your missing C bindings.
// They don't need to do anything, just exist so your Rust code compiles.

// ====================================================================
// Type Aliases (PSoC Interrupt Handler)
// Define the C function pointer type used for interrupt handlers.
// The `ISR_Pulser_StartEx` and `UART_SetCustomInterruptHandler` errors
// suggest the C signature is `void func(void)`.
// ====================================================================

// Define the type for an external interrupt handler function pointer
type InterruptHandler = Option<unsafe extern "C" fn()>;
// Define the type for the UART interrupt handler (assuming the same signature as above)
type UartInterruptHandler = Option<unsafe extern "C" fn()>;

pub const Pulser_tmr__INTR_MASK_TC: u32 = 0x01;

// Motor functions (from src/motor.rs errors)
pub unsafe extern "C" fn DecL_Init() {}
pub unsafe extern "C" fn DecL_Start() {}
pub unsafe extern "C" fn DecL_WriteCounter(_value: u32) {}
pub unsafe extern "C" fn DecL_ReadCounter() -> u32 {
    0
} // Must return the expected type
pub unsafe extern "C" fn Pulser_tmr_ClearInterrupt(_mask: u32) {}
pub unsafe extern "C" fn ISR_Pulser_StartEx(_handler: Option<extern "C" fn()>) {}
pub unsafe extern "C" fn Pulser_InterruptHandler() {}
pub unsafe extern "C" fn Pulser_tmr_Start() {}
pub unsafe extern "C" fn DIR_Write(_value: u32) {}

// Serial functions (from src/serial.rs errors)
pub unsafe extern "C" fn UART_UartPutString(s: *const c_char) {
    print!("{}", core::ffi::CStr::from_ptr(s).to_str().unwrap());
}

pub unsafe extern "C" fn UART_Start() {}
pub unsafe extern "C" fn UART_SetCustomInterruptHandler(_handler: Option<extern "C" fn()>) {}
pub unsafe extern "C" fn UART_SpiUartClearRxBuffer() {}
pub unsafe extern "C" fn UART_UartGetChar() -> u32 {
    0
}
pub unsafe extern "C" fn ClearInterrutpt_RX() {}
pub unsafe extern "C" fn UART_SpiUartWriteTxData(_data: u32) {}

static stub_StepReg: LocalStatic<u8> = LocalStatic::new();

// System/GPIO functions (from src/sys.rs errors)
pub unsafe extern "C" fn StepReg_Write(value: u8) {
    *stub_StepReg.get_mut() = value;
}
pub unsafe extern "C" fn EN_Write(_value: u32) {}
// ... add all other missing functions (ui::* and others)
pub unsafe extern "C" fn CySysTickInit() {}
pub unsafe extern "C" fn CySysTickStart() {}
pub unsafe extern "C" fn CySysTickEnable() {}
pub unsafe extern "C" fn CySysTickSetReload(_value: u32) {}

static stub_LED: LocalStatic<u8> = LocalStatic::new();
// GPIO Write/Set
pub unsafe extern "C" fn LED_Write(value: u8) {
    *stub_LED.get_mut() = value & 1;
}

// GPIO Read/Get
pub unsafe extern "C" fn LED_Read() -> u8 {
    stub_LED.get().clone()
} // Assume LED_Read returns 0 or 1
static stub_SYSTICK: LocalStatic<u32> = LocalStatic::new();
// System Tick (from src/ui.rs)
pub unsafe extern "C" fn CySysTickGetValue() -> u32 {
    *stub_SYSTICK.get_mut() = stub_SYSTICK.get() + 10;
    stub_SYSTICK.get().clone()
}
