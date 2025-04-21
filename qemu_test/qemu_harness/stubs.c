/* malloc/free are now provided by freertos_hooks.c to route to FreeRTOS heap. */
void abort(void) { while(1); }
