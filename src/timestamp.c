#include <time.h>
#include <string.h>
#include <stdint.h>
// #include <stdio.h>

char const * get_timestamp(char * ret_str, uint32_t size, char const * fmt_str) {
    struct tm *time_val;
    time_t now;
    time(&now);
    time_val = localtime(&now); // load time info into time_val    
    strftime(ret_str, size, fmt_str, time_val); 
    return ret_str;
}

// int main() {
//     char timestamp[25] = {'\0'};
//     get_timestamp(timestamp, sizeof(timestamp), "%Y%m%d-%T");
//     printf("%s\n", timestamp);
// }