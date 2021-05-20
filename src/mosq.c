#include <stdlib.h>
#include <stdio.h>
#include <signal.h>
#include "mosquitto.h"

int on_message(struct mosquitto *mosq, void *userdata, const struct mosquitto_message *msg) {
    printf("%s %s (%d)\n", msg->topic, (const char *)msg->payload, msg->payloadlen);
    return 0;
}

void quit(int signum) {
    mosquitto_lib_cleanup();
    exit(0);
}

int main(int argc, char *argv[]) {
    int rc;
    signal(SIGINT, quit);
    mosquitto_lib_init();
/**
 * callback	        a callback function in the following form: int callback(struct mosquitto *mosq, void *obj, const struct mosquitto_message *message) Note that this is the same as the normal on_message callback, except that it returns an int.
 * userdata	        user provided pointer that will be passed to the callback.
 * topic	        the subscription topic to use (wildcards are allowed).
 * qos	            the qos to use for the subscription.
 * host	            the broker to connect to.
 * port	            the network port the broker is listening on.
 * client_id        the client id to use, or NULL if a random client id should be generated.
 * keepalive        the MQTT keepalive value.
 * clean_session    the MQTT clean session flag.
 * username	        the username string, or NULL for no username authentication.
 * password	        the password string, or NULL for an empty password.
 * will	            a libmosquitto_will struct containing will information, or NULL for no will.
 * tls	            a libmosquitto_tls struct containing TLS related parameters, or NULL for no use of TLS.
*/
    rc = mosquitto_subscribe_callback(
        on_message, NULL,
        "sinkd/#", 0,
        "localhost", 1883,
        NULL, 60, true,
        NULL, NULL,
        NULL, NULL);

    if (rc != MOSQ_ERR_SUCCESS) {
        printf("Error: %s\n", mosquitto_strerror(rc));
        quit(SIGINT);
    }

    return rc;
}
