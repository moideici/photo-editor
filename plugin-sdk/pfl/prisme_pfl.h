/* Prisme PFL interoperability declarations — MIT.
 * PhotoFiltre 11 Unicode x64 ABI, reconstructed from the public SDK and
 * observed PF11 record layouts. This is not the Prisme DLL C ABI.
 * Host support is still under development. No VCL object layout is exposed. */
#ifndef PRISME_PFL_H
#define PRISME_PFL_H
#include <stdint.h>
#include <stddef.h>
#if defined(_WIN32) && !defined(_WIN64)
#error Prisme PFL declarations target Windows x64 only
#endif
#pragma pack(push, 8)
#if defined(_WIN32)
#define PFL_CALL __stdcall
#define PFL_EXPORT __declspec(dllexport)
#else
#define PFL_CALL
#define PFL_EXPORT
#endif
typedef int32_t PflBool;
typedef int32_t PflColor; /* Windows COLORREF / Delphi TColor, special values negative */
typedef void *PflBitmap;  /* Opaque host object; only pass back to host callbacks. */
typedef void *PflWindow;
typedef void *PflHandle;
typedef uint16_t PflChar;
typedef struct PflRect { int32_t left, top, right, bottom; } PflRect;
typedef struct PflPoint { int32_t x, y; } PflPoint;
typedef struct PflHeader {
  PflChar caption[64], version[64];
  int32_t plugin_type;
  PflChar author[64], comment[256], reserved[256];
  uint32_t flags;
} PflHeader;
typedef struct PflLanguage { PflChar code[4], name[64]; } PflLanguage;
/* Default x64 alignment: the three pointers start at 0x20, NOT 0x1c. */
typedef struct PflImage {
  int32_t size, width, height, width_bytes, scanline_bytes, bytes_per_pixel, data_bytes;
  uint8_t *buffer;
  PflBitmap bitmap;
  PflHandle handle;
} PflImage;
typedef PflBool (PFL_CALL *PflEnumImage)(const PflChar *, int32_t, int32_t, int32_t, PflColor);
typedef struct PflHost {
  PflWindow main_window;             /* 0x000 */
  PflBool image_exists, selected;    /* 0x008 / 0x00c */
  PflRect selection_rect;            /* 0x010 */
  PflRect image_rect;                /* 0x020 */
  PflBool themes_enabled;            /* 0x030, natural padding to 0x038 */
  void (PFL_CALL *begin_filter)(const PflChar *, PflBool);
  void (PFL_CALL *end_filter)(void);
  PflBool (PFL_CALL *get_bitmap)(PflBitmap);
  PflBool (PFL_CALL *set_bitmap)(PflBitmap);
  PflBool (PFL_CALL *get_selection_bitmap)(PflBitmap);
  PflBool (PFL_CALL *set_selection_bitmap)(PflBitmap, int32_t);
  PflColor (PFL_CALL *get_transparent_color)(void);
  PflBool (PFL_CALL *set_transparent_color)(PflColor);
  int32_t (PFL_CALL *get_color_count)(void);
  int32_t (PFL_CALL *set_color_count)(int32_t);
  PflBool (PFL_CALL *new_image)(PflBitmap, const PflChar *, int32_t, int32_t, PflColor);
  void (PFL_CALL *show_form)(void *, int32_t, PflBool, PflBool);
  PflBool (PFL_CALL *set_alpha_blend)(int32_t);
  PflBool (PFL_CALL *draw_selection_border)(int32_t, PflColor, PflBool);
  PflBool (PFL_CALL *get_selection_mask)(PflBitmap); /* full image mask */
  void (PFL_CALL *reset_save_menu)(void);
  PflChar *(PFL_CALL *get_image_filename)(PflChar *, int32_t);
  PflChar *(PFL_CALL *get_exe_path)(PflChar *, int32_t);
  PflChar *(PFL_CALL *get_open_path)(PflChar *, int32_t);
  PflChar *(PFL_CALL *get_save_path)(PflChar *, int32_t);
  PflChar *(PFL_CALL *get_plugin_path)(PflChar *, int32_t);
  PflChar *(PFL_CALL *get_ini_file)(PflChar *, int32_t);
  PflBool (PFL_CALL *set_image_filename)(const PflChar *);
  PflBool (PFL_CALL *get_bitmap_by_name)(PflBitmap, const PflChar *);
  PflBool (PFL_CALL *get_thumbnail)(PflBitmap, PflColor, int32_t, int32_t, PflColor, PflRect *);
  PflBool (PFL_CALL *get_thumbnail_by_name)(PflBitmap, const PflChar *, PflColor, int32_t, int32_t, PflColor, PflRect *);
  PflBool (PFL_CALL *enumerate_images)(PflEnumImage);
  /* API level 2 */
  void (PFL_CALL *prepare_dialog)(PflWindow, int32_t, PflBool, PflBool);
  int32_t (PFL_CALL *create_bitmap)(PflImage *);
  int32_t (PFL_CALL *free_bitmap)(PflImage *);
  PflHandle (PFL_CALL *release_bitmap_handle)(PflImage *);
  uint8_t *(PFL_CALL *get_scanline)(PflImage *, int32_t);
  /* API level 3 */
  PflBool (PFL_CALL *get_cropped_selection_mask)(PflBitmap);
  int32_t (PFL_CALL *get_image_dpi)(void);
  int32_t (PFL_CALL *set_image_dpi)(int32_t);
  PflBool (PFL_CALL *set_selection_mask)(PflBitmap, int32_t);
  PflRect (PFL_CALL *reset_undo_area)(PflRect);
  int32_t (PFL_CALL *set_application_state)(int32_t);
  int32_t (PFL_CALL *get_selection_shape)(PflPoint *);
  void *unused_level3;              /* 0x170, not a callback */
  /* API level 4 */
  PflRect (PFL_CALL *get_layer_rect)(void);
  PflBool (PFL_CALL *get_layer_bitmap)(PflBitmap);
  PflBool (PFL_CALL *set_layer_bitmap)(PflBitmap);
  PflBool (PFL_CALL *get_layer_alpha)(PflBitmap);
  PflBool (PFL_CALL *set_layer_alpha)(PflBitmap);
  PflBool (PFL_CALL *get_selection_alpha)(PflBitmap);
  PflBool (PFL_CALL *set_selection_alpha)(PflBitmap);
  void (PFL_CALL *disable_layers_painting)(PflBool);
  int32_t (PFL_CALL *get_image_color_mode)(void);
  PflPoint (PFL_CALL *get_layer_position)(void);
  void *reserved[10];
} PflHost;
/* Return any nonzero LongBool as true; do not compare to 1 only. */
#define PFL_FALSE 0
#define PFL_TRUE (-1)
#define PFL_FILTER 0
#define PFL_IMAGE 1
#define PFL_IMPORT 2
#define PFL_EXPORT_IMAGE 3
#define PFL_TOOL 4
#define PFL_RGB_COLORS 0x00000001u
#define PFL_INDEXED_COLORS 0x00000002u
#define PFL_TRANSPARENT 0x00000004u
#define PFL_NOT_TRANSPARENT 0x00000008u
#define PFL_SELECTED 0x00000010u
#define PFL_NOT_SELECTED 0x00000020u
#define PFL_NEED_IMAGE 0x00000040u
#define PFL_NEED_MULTIPLE_IMAGES 0x00000080u
#define PFL_LAYERED 0x00000100u
#define PFL_NOT_LAYERED 0x00000200u
#define PFL_RGBA_COLORS 0x00000400u
#define PFL_NOT_RGBA_COLORS 0x00000800u
#define PFL_IMAGE_JPEG 0x00010000u
#define PFL_IMAGE_GIF 0x00020000u
#define PFL_IMAGE_PNG 0x00040000u
#define PFL_IMAGE_BMP 0x00080000u
#define PFL_IMAGE_TIFF 0x00100000u
#define PFL_IMAGE_PFS 0x00200000u
#define PFL_JPEG_EXIF 0x00400000u
/* Mandatory DLL exports (names must be exact):
 * int32_t PFL_CALL RegisterCompatibility(void);
 * PflHeader *PFL_CALL RegisterPluginHeader(void);
 * PflHost *PFL_CALL RegisterPluginStruct(void);
 * int32_t PFL_CALL Execute(void);
 * Optional: RegisterPluginLanguage, RegisterPluginGlyph, AboutPlugin.
 */
#pragma pack(pop)
#endif

