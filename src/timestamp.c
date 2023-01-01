#include <time.h>
#include <string.h>
#include <stdint.h>
#include <stdio.h>

// NOTE: for some reason muslc doesn't like ':' in the fmt_str
// you can however add %T with no issue IF IT IS ALONE
void timestamp(char *ret_str, uint32_t size, char const *fmt_str) {
    struct tm *time_val;
    time_t now;
    time(&now);
    time_val = localtime(&now); // load time info into time_val    
    strftime(ret_str, size, fmt_str, time_val);
}