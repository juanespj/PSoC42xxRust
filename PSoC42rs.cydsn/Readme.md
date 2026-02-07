# PSOC Export to IDE
export to makefile

# platform_debug.mk
```
# Macros required for the generic driver for toolchain and/or operating system.

DESIGN= PSoC4Embassy
CONFIG= debug
TOOLCHAIN_DIR =C:/Program Files (x86)/Cypress/PSoC Creator/4.4/PSoC Creator/import/gnu/arm/5.4.1

# ---------- FORCE TOOLCHAIN (disable autodetect) ----------
CC = $(TOOLCHAIN_DIR)/bin/arm-none-eabi-gcc
LD = $(TOOLCHAIN_DIR)/bin/arm-none-eabi-gcc
AS = $(TOOLCHAIN_DIR)/bin/arm-none-eabi-as
AR = $(TOOLCHAIN_DIR)/bin/arm-none-eabi-ar
# ----------------------------------------------------------


OUT_DIR_CortexM0= output/$(CONFIG)/CortexM0
CFLAGS_CortexM0= -mcpu=cortex-m0 -mthumb -I. -IGenerated_Source/PSoC4 -Wa,-alh=$(OUT_DIR_CortexM0)/$(basename $(<F)).lst -g -D DEBUG -Wall -ffunction-sections -ffat-lto-objects 
CDEPGEN_CortexM0= -MM $< -MF $(OUT_DIR_CortexM0)/$(<F).d -MT $(OUT_DIR_CortexM0)/$(@F) $(CFLAGS_CortexM0)

LDFLAGS_CortexM0= -mcpu=cortex-m0 -mthumb -l rust_project -L Generated_Source/PSoC4 -L ./rust_project/build/thumbv6m-none-eabi/release -Wl,-Map,$(OUT_DIR_CortexM0)/$(DESIGN).map -T Generated_Source/PSoC4/cm0gcc.ld -specs=nano.specs -Wl,--gc-sections -g -ffunction-sections -O0 -ffat-lto-objects

ASFLAGS_CortexM0= -mcpu=cortex-m0 -mthumb -I. -IGenerated_Source/PSoC4 -alh=$(OUT_DIR_CortexM0)/$(basename $(<F)).lst -g

ARFLAGS= -rs

RM= rm
RMFLAGS= -rf
```

# system architecture
cargo find 
rustc -vV | grep host

# prebuild.sh
in export/

```
#!/bin/sh
cmd //C "rustbuild.bat"
exit $?
```
# linker
Generated_Source/PSoC4/cm0gcc.ld

# elf size
arm-none-eabi-size output/debug/CortexM0/PSoC4Embassy.elf


# elf vector tables
 arm-none-eabi-objdump -h .\build\PSoC4Embassy.elf | grep -E "text|vectors"
  0 .text         00004c58  00000000  00000000  00000158  2**3
  4 .ramvectors   000000c0  20000000  20000000  00006d68  2**3

# elf
```
arm-none-eabi-objdump -d build/PSoC4rs.elf | head -n 20
arm-none-eabi-objdump -d CortexM0\ARM_GCC_541\Debug\PSoC4Embassy.elf|head -40

 arm-none-eabi-nm --defined-only build/PSoC4rs.elf | grep Reset
  arm-none-eabi-nm --defined-only CortexM0\ARM_GCC_541\Debug\PSoC4Embassy.elf| grep Reset


   arm-none-eabi-readelf -A .\build\PSoC4rs.elf
    arm-none-eabi-readelf -A .\CortexM0\ARM_GCC_541\Debug\PSoC4Embassy.elf

arm-none-eabi-readelf -h .\build\PSoC4rs.elf 
arm-none-eabi-readelf -h .\CortexM0\ARM_GCC_541\Debug\PSoC4Embassy.elf 

arm-none-eabi-nm build/PSoC4rs.elf | grep Vectors
arm-none-eabi-nm .\CortexM0\ARM_GCC_541\Debug\PSoC4Embassy.elf | grep Vectors


```

build/PSoC4Embassy.elf:     file format elf32-littlearm


Disassembly of section .text:

00000000 <RomVectors>:
       0:       00 10 00 20 11 00 00 00 95 01 00 00 95 01 00 00     ... ............

00000010 <Reset>:
      10:       b508            push    {r3, lr}
      12:       f000 f945       bl      2a0 <Start_c>
        ...

00000100 <__do_global_dtors_aux>:
     100:       b510            push    {r4, lr}
     102:       4c05            ldr     r4, [pc, #20]   @ (118 <__do_global_dtors_aux+0x18>)
     104:       7823            ldrb    r3, [r4, #0]
     106:       b933            cbnz    r3, 116 <__do_global_dtors_aux+0x16>
     108:       4b04            ldr     r3, [pc, #16]   @ (11c <__do_global_dtors_aux+0x1c>)