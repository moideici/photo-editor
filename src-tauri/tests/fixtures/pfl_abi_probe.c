/* Cross-check the public C declaration with observed PF11 x64 offsets. */
#include "../../../plugin-sdk/pfl/prisme_pfl.h"
#include <stdio.h>
_Static_assert(sizeof(void *) == 8, "PFL probe requires x64");
_Static_assert(sizeof(PflHeader) == 0x588, "header");
_Static_assert(offsetof(PflHeader, plugin_type) == 0x100, "plugin type");
_Static_assert(offsetof(PflHeader, flags) == 0x584, "menu flags");
_Static_assert(sizeof(PflLanguage) == 0x88, "language");
_Static_assert(sizeof(PflImage) == 0x38, "DBmp");
_Static_assert(offsetof(PflImage, buffer) == 0x20, "DBmp buffer");
_Static_assert(offsetof(PflImage, bitmap) == 0x28, "DBmp object");
_Static_assert(offsetof(PflImage, handle) == 0x30, "DBmp handle");
_Static_assert(offsetof(PflHost, selection_rect) == 0x10, "selection before image");
_Static_assert(offsetof(PflHost, image_rect) == 0x20, "image");
_Static_assert(offsetof(PflHost, begin_filter) == 0x38, "level 1");
_Static_assert(offsetof(PflHost, enumerate_images) == 0x108, "last level 1");
_Static_assert(offsetof(PflHost, prepare_dialog) == 0x110, "level 2");
_Static_assert(offsetof(PflHost, get_scanline) == 0x130, "last level 2");
_Static_assert(offsetof(PflHost, get_cropped_selection_mask) == 0x138, "level 3");
_Static_assert(offsetof(PflHost, get_selection_shape) == 0x168, "last level 3");
_Static_assert(offsetof(PflHost, unused_level3) == 0x170, "unused slot");
_Static_assert(offsetof(PflHost, get_layer_rect) == 0x178, "level 4");
_Static_assert(offsetof(PflHost, get_layer_position) == 0x1c0, "last level 4");
_Static_assert(offsetof(PflHost, reserved) == 0x1c8, "end written by PF11");
_Static_assert(sizeof(PflHost) == 0x218, "SDK future slots");
int main(void) {
  printf("{\"passed\":true,\"pointerBytes\":%zu,\"headerBytes\":%zu,\"languageBytes\":%zu,\"imageBytes\":%zu,\"hostBytes\":%zu,\"assertions\":22}\n",
    sizeof(void *), sizeof(PflHeader), sizeof(PflLanguage), sizeof(PflImage), sizeof(PflHost));
  return 0;
}

