#ifndef PRISME_PLUGIN_H
#define PRISME_PLUGIN_H
#include <stddef.h>
#include <stdint.h>
#if defined(_WIN32)
#define PRISME_EXPORT __declspec(dllexport)
#else
#define PRISME_EXPORT __attribute__((visibility("default")))
#endif
#ifdef __cplusplus
extern "C" {
#endif
/* ABI v1, alignement C naturel. Windows x64 : sizeof = 56. */
typedef struct PrismeImageV1 {
    uint32_t struct_size;
    uint32_t abi_version;
    uint32_t width;
    uint32_t height;
    uint32_t stride;
    uint32_t reserved;
    uint8_t *pixels;
    size_t pixels_len;
    const uint8_t *params_json;
    size_t params_json_len;
} PrismeImageV1;
/* RGBA8 non prémultiplié, lignes de haut en bas, stride=width*4.
   Seul pixels[0..pixels_len] est modifiable. Aucun pointeur n'est conservé.
   params_json est un objet UTF-8 sans terminateur NUL garanti.
   Retour : 0 succès ; autre valeur = échec. error_utf8 peut recevoir un
   message UTF-8 terminé par NUL, au plus error_capacity-1 octets.
   Ne pas allouer/libérer les buffers de l'hôte. Ne pas modifier la structure.
   Le plugin doit être sans interface : ses réglages viennent du manifeste. */
PRISME_EXPORT int32_t prisme_filter_v1(PrismeImageV1 *image,
                                    uint8_t *error_utf8, size_t error_capacity);
#ifdef __cplusplus
}
#endif
#endif
