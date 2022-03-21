#include <linux/input.h>
#include <linux/uinput.h>
#include <stdio.h>

void main(void)
{
    printf("EVIOCSABS(ABS_X) = %x\n", EVIOCSABS(ABS_X));
    printf("UI_ABS_SETUP = %x\n", UI_ABS_SETUP);
    printf("UI_DEV_SETUP = %x\n", UI_DEV_SETUP);
}