#include "../../../plugin-sdk/prisme_plugin.h"
#include <windows.h>
#include <string.h>
#include <stdlib.h>
/* Fixture indépendante de l'hôte : succès, retour d'erreur, crash, délai,
   et contrat corrompu. Ne fait partie d'aucun paquet de distribution. */
PRISME_EXPORT int32_t prisme_filter_v1(PrismeImageV1 *image,
                                    uint8_t *error, size_t capacity) {
    char args[129] = {0};
    size_t n = image->params_json_len < 128 ? image->params_json_len : 128;
    memcpy(args, image->params_json, n);
    char *value = strstr(args, "\"mode\":");
    int mode = value ? (int)strtol(value + 7, NULL, 10) : 0;
    if (mode == 1) {
        if (capacity >= 8) memcpy(error, "failure", 8);
        return 7;
    }
    if (mode == 2) *(volatile unsigned char *)0 = 0;
    if (mode == 3) Sleep(10000);
    if (mode == 4) {image->width++; return 0;}
    for (size_t i = 0; i < image->pixels_len; i += 4) {
        image->pixels[i] = 255 - image->pixels[i];
        image->pixels[i+1] = 255 - image->pixels[i+1];
        image->pixels[i+2] = 255 - image->pixels[i+2];
    }
    return 0;
}
