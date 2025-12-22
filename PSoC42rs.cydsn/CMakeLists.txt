# --- Configuration for PSoC4 Cortex-M0 Embedded Target ---

cmake_minimum_required(VERSION 3.10) # Updated to remove deprecation warning
# Ensure we specify C and ASM languages
project(PSoC4rs C ASM)

include(CheckIPOSupported)

# --- Build Options ---
option(CYELF_TOOL "Run ELF output through Cypress' post-processors to generate a HEX file to use with their programmer, bootloader, etc" OFF)
set(CYELF_TOOL ON CACHE BOOL "Run ELF output through Cypress' post-processors to generate a HEX file" FORCE)

# Path for auto-generated source/header code for compiler from IDE
set(GEN_SOURCE_DIR Generated_Source/PSoC4)

# Path for auto-generated files for everything else (linker, etc)
# This needs to be absolute, as it's going to be passed to the linker.

# --- Source File Definitions ---

# Sources compiled without the separate 'cyfitter_cfg' compilation step (most files)
set(PSOC_GENERATED_C_SRC
    # --- PSoC Generated Component Sources (from build output) ---
    ${GEN_SOURCE_DIR}/BTN.c
    ${GEN_SOURCE_DIR}/BTN_PM.c
    ${GEN_SOURCE_DIR}/LED.c
    ${GEN_SOURCE_DIR}/LED_PM.c
    ${GEN_SOURCE_DIR}/LchA.c
    ${GEN_SOURCE_DIR}/LchA_PM.c
    ${GEN_SOURCE_DIR}/StepReg.c
    ${GEN_SOURCE_DIR}/StepReg_PM.c
    ${GEN_SOURCE_DIR}/stepX.c
    ${GEN_SOURCE_DIR}/stepX_PM.c
    ${GEN_SOURCE_DIR}/LchB.c
    ${GEN_SOURCE_DIR}/LchB_PM.c
    ${GEN_SOURCE_DIR}/DIR.c
    ${GEN_SOURCE_DIR}/DIR_PM.c
    ${GEN_SOURCE_DIR}/EN.c
    ${GEN_SOURCE_DIR}/EN_PM.c
    ${GEN_SOURCE_DIR}/clkA.c
    ${GEN_SOURCE_DIR}/ISR_Pulser.c
    ${GEN_SOURCE_DIR}/UART.c
    ${GEN_SOURCE_DIR}/UART_SPI_UART.c
    ${GEN_SOURCE_DIR}/UART_SPI_UART_INT.c
    ${GEN_SOURCE_DIR}/UART_PM.c
    ${GEN_SOURCE_DIR}/UART_UART.c
    ${GEN_SOURCE_DIR}/UART_BOOT.c
    ${GEN_SOURCE_DIR}/UART_UART_BOOT.c
    ${GEN_SOURCE_DIR}/DecL.c
    ${GEN_SOURCE_DIR}/DecL_PM.c
    ${GEN_SOURCE_DIR}/EN_GND.c
    ${GEN_SOURCE_DIR}/EN_GND_PM.c
    ${GEN_SOURCE_DIR}/I2CS.c
    ${GEN_SOURCE_DIR}/I2CS_I2C_INT.c
    ${GEN_SOURCE_DIR}/I2CS_I2C.c
    ${GEN_SOURCE_DIR}/I2CS_I2C_BOOT.c
    ${GEN_SOURCE_DIR}/I2CS_I2C_SLAVE.c
    ${GEN_SOURCE_DIR}/I2CS_PM.c
    ${GEN_SOURCE_DIR}/I2CS_BOOT.c
    ${GEN_SOURCE_DIR}/ISR_QDecL.c
    ${GEN_SOURCE_DIR}/Pulser_tmr.c
    ${GEN_SOURCE_DIR}/Pulser_tmr_PM.c
    ${GEN_SOURCE_DIR}/stepY.c
    ${GEN_SOURCE_DIR}/stepY_PM.c
    ${GEN_SOURCE_DIR}/stepZ.c
    ${GEN_SOURCE_DIR}/stepZ_PM.c
    ${GEN_SOURCE_DIR}/IDAC.c
    ${GEN_SOURCE_DIR}/IDAC_PM.c
    ${GEN_SOURCE_DIR}/ADC_SAR_Seq.c
    ${GEN_SOURCE_DIR}/ADC_SAR_Seq_PM.c
    ${GEN_SOURCE_DIR}/ADC_SAR_Seq_INT.c
    ${GEN_SOURCE_DIR}/Iref.c
    ${GEN_SOURCE_DIR}/Iref_PM.c
    ${GEN_SOURCE_DIR}/SPD_REF.c
    ${GEN_SOURCE_DIR}/SPD_REF_PM.c
    ${GEN_SOURCE_DIR}/UART_SCBCLK.c
    ${GEN_SOURCE_DIR}/UART_tx.c
    ${GEN_SOURCE_DIR}/UART_tx_PM.c
    ${GEN_SOURCE_DIR}/UART_SCB_IRQ.c
    ${GEN_SOURCE_DIR}/UART_rx.c
    ${GEN_SOURCE_DIR}/UART_rx_PM.c
    ${GEN_SOURCE_DIR}/I2CS_SCBCLK.c
    ${GEN_SOURCE_DIR}/I2CS_sda.c
    ${GEN_SOURCE_DIR}/I2CS_sda_PM.c
    ${GEN_SOURCE_DIR}/I2CS_scl.c
    ${GEN_SOURCE_DIR}/I2CS_scl_PM.c
    ${GEN_SOURCE_DIR}/I2CS_SCB_IRQ.c
    ${GEN_SOURCE_DIR}/ADC_SAR_Seq_IRQ.c
    ${GEN_SOURCE_DIR}/ADC_SAR_Seq_intClock.c
    ${GEN_SOURCE_DIR}/CyFlash.c
    ${GEN_SOURCE_DIR}/CyLib.c
    ${GEN_SOURCE_DIR}/cyPm.c
    ${GEN_SOURCE_DIR}/cyutils.c
    ${GEN_SOURCE_DIR}/CyLFClk.c
    ${GEN_SOURCE_DIR}/cy_em_eeprom.c
    # --- PSoC Generated Startup/Boot Sources ---
)

# User-provided source files from the build output
set(USER_SRC
    ${PROJECT_SOURCE_DIR}/main.c
    ${PROJECT_SOURCE_DIR}/bindgen_wrappers.c
)

set(PSOC_CREATOR_INCLUDE_DIRS
    ${PROJECT_SOURCE_DIR}/Generated_Source/PSoC4
    ${PROJECT_SOURCE_DIR}/Generated_Source/PSoC4/CyLib
    ${PROJECT_SOURCE_DIR}/Generated_Source/PSoC4/CyDevice
    ${PROJECT_SOURCE_DIR}/Generated_Source/PSoC4/CyBoot
    ${PROJECT_SOURCE_DIR}/codegentemp
    ${PROJECT_SOURCE_DIR}
)
# --- 1. Define the Main Generated Static Library ---

add_library(PSOC4_STARTUP_LIB STATIC #PSOC_GENERATED_SEP_SRC 
    ${GEN_SOURCE_DIR}/Cm0Start.c
    ${GEN_SOURCE_DIR}/cyfitter_cfg.c
    ${GEN_SOURCE_DIR}/cymetadata.c
    ${GEN_SOURCE_DIR}/CyBootAsmGnu.s
)
target_include_directories(PSOC4_STARTUP_LIB PRIVATE
    ${PSOC_CREATOR_INCLUDE_DIRS}

)
add_library(PSOC4_GEN_LIB STATIC
    ${PSOC_GENERATED_C_SRC}

)
target_include_directories(PSOC4_GEN_LIB PRIVATE
    ${PSOC_CREATOR_INCLUDE_DIRS}
    
)

set(RUST_LIB_PATH 
    "${PROJECT_SOURCE_DIR}/rust_project/build/thumbv6m-none-eabi/release/librust_project.a"
)

# Build all in one project
# Could do this as separate libraries, 
add_executable(PSoC4rs
    ${USER_SRC}
    # creator_stubs.c
)
target_include_directories(PSoC4rs
    PUBLIC
    ${PSOC_CREATOR_INCLUDE_DIRS}
)

target_link_libraries(PSoC4rs PRIVATE
    -Wl,--start-group
    PSOC4_STARTUP_LIB
    ${PROJECT_SOURCE_DIR}/Export/ARM_GCC_Generic/CyComponentLibrary.a  
    ${RUST_LIB_PATH}
    PSOC4_GEN_LIB
    
    c
    m
    -Wl,--end-group
)

# arm-none-eabi-gcc.exe -Wl,--start-group -o PSoC4Embassy.elf -mcpu=cortex-m0 -mthumb -l rust_project -L Generated_Source\PSoC4 -L .\rust_project\build\thumbv6m-none-eabi\release -Wl,-Map,.\CortexM0\ARM_GCC_541\Debug/PSoC4Embassy.map -T Generated_Source\PSoC4\cm0gcc.ld -specs=nano.specs -Wl,--gc-sections -g -ffunction-sections -O0 -ffat-lto-objects -Wl,--end-group
# cyelftool.exe -C J:\Projects\PSoC-Rust\PSoC4Embassy.cydsn\CortexM0\ARM_GCC_541\Debug\PSoC4Embassy.elf --flash_row_size 128 --flash_size 32768 --flash_offset 0x00000000
target_link_options(PSoC4rs PRIVATE
    -Wl,--undefined=Reset_Handler
    -Wl,-u,main
)

# // Disable garbage collection for now (debugging only):
# target_link_options(PSoC4rs PRIVATE
#     -Wl,--no-gc-sections
# )


set_target_properties(PSoC4rs PROPERTIES
    OUTPUT_NAME "PSoC4rs.elf"
    LINK_FLAGS "-T cm0gcc.ld -L \"${PROJECT_SOURCE_DIR}\"/codegentemp"
        INTERPROCEDURAL_OPTIMIZATION TRUE

)


# This is a post-build step that runs the Cypress metadata tool (cyelftool.exe)
if(CYELF_TOOL)
    add_custom_command(TARGET PSoC4rs
        POST_BUILD
        COMMAND ${PROJECT_SOURCE_DIR}/Export/cyelftool.exe -C "PSoC4rs.elf" --flash_row_size 123 --flash_size 32768 
    )
endif()
