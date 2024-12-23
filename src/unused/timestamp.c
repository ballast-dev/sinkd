#include <time.h>
#include <string.h>
#include <stdint.h>
#include <stdio.h>

typedef struct {
    time_t timestamp;
    uint32_t cycle_number;
} LastSync;

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

/**
 * Reads the lastsync file and parses the timestamp and cycle number.
 * 
 * @param file_path Path to the lastsync file.
 * @param sync Pointer to a LastSync structure to populate.
 * @return 0 on success, -1 on failure.
 */
int read_lastsync(const char *file_path, LastSync *sync) {
    FILE *file = fopen(file_path, "r");
    if (!file) {
        perror("Error opening file");
        return -1;
    }

    char line[256];

    // Read the timestamp
    if (!fgets(line, sizeof(line), file)) {
        fprintf(stderr, "Error: Failed to read timestamp from file\n");
        fclose(file);
        return -1;
    }
    sync->timestamp = strtoll(line, NULL, 10);
    if (sync->timestamp == 0 && line[0] != '0') {
        fprintf(stderr, "Error: Invalid timestamp format\n");
        fclose(file);
        return -1;
    }

    // Read the cycle number
    if (!fgets(line, sizeof(line), file)) {
        fprintf(stderr, "Error: Failed to read cycle number from file\n");
        fclose(file);
        return -1;
    }
    sync->cycle_number = strtoul(line, NULL, 10);
    if (sync->cycle_number == 0 && line[0] != '0') {
        fprintf(stderr, "Error: Invalid cycle number format\n");
        fclose(file);
        return -1;
    }

    fclose(file);
    return 0;
}

/**
 * Formats a given timestamp using the provided timestamp function.
 * 
 * @param sync The LastSync structure containing the timestamp to format.
 * @param ret_str Buffer to store the formatted string.
 * @param size Size of the buffer.
 * @param fmt_str Format string for strftime.
 * @return 0 on success, -1 on failure.
 */
int format_lastsync(const LastSync *sync, char *ret_str, uint32_t size, const char *fmt_str) {
    struct tm *time_val = localtime(&sync->timestamp);
    if (!time_val) {
        fprintf(stderr, "Error: localtime() failed\n");
        return -1;
    }

    size_t bytes_written = strftime(ret_str, size, fmt_str, time_val);
    if (bytes_written == 0) {
        fprintf(stderr, "Error: strftime() failed or output was truncated\n");
        return -1;
    }

    return 0;
}

