/* 
  Interrupt enable, was a function-like macro (not translated by bindgen)
*/
#include "project.h"

void ClearInterrutpt_RX(void){
   UART_ClearRxInterruptSource(UART_GetRxInterruptSource());
}
void Uart_Put(void){
  UART_UartPutChar('\r');
  UART_UartPutChar('\n');

}
CY_ISR_PROTO(IntDefaultHandler);
