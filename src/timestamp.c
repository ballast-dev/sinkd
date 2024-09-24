#include <time.h>
#include <string.h>
#include <stdint.h>
#include <stdio.h>

// NOTE: for some reason muslc doesn't like ':' in the fmt_str
// you can however add %T with no issue IF IT IS ALONE
void timestamp(char *ret_str, uint32_t size, char const *fmt_str) {
    time_t now = time(NULL);
    if (now == (time_t)-1) {
        fprintf(stderr, "Error: time() failed\n");
        ret_str[0] = '\0';
        return;
    }

    struct tm *time_val = localtime(&now);
    if (time_val == NULL) {
        fprintf(stderr, "Error: localtime() returned NULL\n");
        ret_str[0] = '\0';
        return;
    }

    size_t bytes_written = strftime(ret_str, size, fmt_str, time_val);
    if (bytes_written == 0) {
        fprintf(stderr, "Error: strftime() failed or output was truncated\n");
        ret_str[0] = '\0';
    }
}
