#include <stdint.h>
 #include <project.h>
//#include "rust_project\rust_api.h"

// Newer versions of LD get upset when there's nothing in this region, so insert a dummy variable there.
// static volatile uint8_t dummy __attribute__ ((section(".cyeeprom"),unused));

// int main(){
//     while(1){
// LED_Write(!LED_Read());
// CyDelay(500);
//     }
// }
CY_ISR(ISR_TEMP){
    
}
/*

int main(){
    UART_Start();
    UART_UartPutString("\n\rPSoCStarted");

  CyGlobalIntEnable;
  CySysTickSetReload(24000);
  CySysTickSetCallback(0, tick_callback);
 //UART_SetCustomInterruptHandler(UARTRX);
    rust_main();
    
}*/
    
// void test(){
//  LED_Read();
// }