// Host-side stubs when not building for ARM (cargo check on PC).

use core::ffi::c_char;
use local_static::LocalStatic;

pub const Pulser_tmr__INTR_MASK_TC: u32 = 0x01;

pub unsafe extern "C" fn DecL_Init() {}
pub unsafe extern "C" fn DecL_Start() {}
pub unsafe extern "C" fn DecL_WriteCounter(_value: u32) {}
pub static Encoder_count: LocalStatic<u32> = LocalStatic::new();
pub unsafe extern "C" fn DecL_ReadCounter() -> u32 {
    Encoder_count.get().clone()
}

pub unsafe extern "C" fn Pulser_tmr_ClearInterrupt(_mask: u32) {}
pub unsafe extern "C" fn ISR_Pulser_StartEx(_handler: Option<extern "C" fn()>) {}
pub unsafe extern "C" fn Pulser_tmr_Start() {}
pub unsafe extern "C" fn DIR_Write(_value: u8) {}

pub unsafe extern "C" fn UART_UartPutString(_s: *const c_char) {}
pub unsafe extern "C" fn UART_Start() {}
pub unsafe extern "C" fn UART_SetCustomInterruptHandler(_handler: Option<extern "C" fn()>) {}
pub unsafe extern "C" fn UART_SpiUartClearRxBuffer() {}
pub unsafe extern "C" fn UART_UartGetChar() -> u32 {
    0
}
pub unsafe extern "C" fn ClearInterrutpt_RX() {}
pub unsafe extern "C" fn UART_SpiUartWriteTxData(_data: u32) {}
pub unsafe extern "C" fn UART_UartPutCRLF(d: u32) {
    unsafe {
        UART_SpiUartWriteTxData(d);
    }
}

static stub_StepReg: LocalStatic<u8> = LocalStatic::new();
pub unsafe extern "C" fn StepReg_Write(value: u8) {
    *stub_StepReg.get_mut() = value;
}
pub unsafe extern "C" fn EN_Write(_value: u8) {}

pub unsafe extern "C" fn CyEnterCriticalSection() -> u8 {
    0
}
pub unsafe extern "C" fn CyExitCriticalSection(_saved: u8) {}

pub unsafe extern "C" fn CySysTickInit() {}
pub unsafe extern "C" fn CySysTickStart() {}
pub unsafe extern "C" fn CySysTickEnable() {}
pub unsafe extern "C" fn CySysTickSetReload(_value: u32) {}
pub unsafe extern "C" fn CySysTickSetCallback(_n: u32, _cb: Option<extern "C" fn()>) {}
pub unsafe extern "C" fn CySysTickClear() {}

static stub_LED: LocalStatic<u8> = LocalStatic::new();
pub unsafe extern "C" fn LED_Write(value: u8) {
    *stub_LED.get_mut() = value & 1;
}
pub unsafe extern "C" fn LED_Read() -> u8 {
    stub_LED.get().clone()
}
pub unsafe extern "C" fn BTN_Read() -> u8 {
    1
}

static stub_SYSTICK: LocalStatic<u32> = LocalStatic::new();
pub unsafe extern "C" fn CySysTickGetValue() -> u32 {
    *stub_SYSTICK.get_mut() = stub_SYSTICK.get() + 10;
    stub_SYSTICK.get().clone()
}

pub unsafe extern "C" fn IDAC_SetValue(_value: u32) {}
pub unsafe extern "C" fn IDAC_Start() {}
pub unsafe extern "C" fn ADC_SAR_Seq_Start() {}
pub unsafe extern "C" fn ADC_SAR_Seq_StartConvert() {}
pub unsafe extern "C" fn ADC_SAR_Seq_GetResult16(_chan: u32) -> i16 {
    0
}
pub unsafe extern "C" fn ADC_SAR_Seq_CountsTo_mVolts(_chan: u32, counts: i16) -> i32 {
    counts as i32
}
