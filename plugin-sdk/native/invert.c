#include "../prisme_plugin.h"
#include <stdio.h>
_Static_assert(sizeof(PrismeImageV1) == 56, "Compiler pour Windows x64");
PRISME_EXPORT int32_t prisme_filter_v1(PrismeImageV1 *image,
                                    uint8_t *error, size_t capacity) {
    if (!image || image->abi_version != 1 || image->struct_size != sizeof(*image)) {
        if (error && capacity) snprintf((char *)error, capacity, "ABI Prisme incompatible");
        return 1;
    }
    for (size_t i = 0; i < image->pixels_len; i += 4) {
        image->pixels[i] = 255 - image->pixels[i];
        image->pixels[i+1] = 255 - image->pixels[i+1];
        image->pixels[i+2] = 255 - image->pixels[i+2];
    }
    return 0;
}
