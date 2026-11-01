/* Prisme — original negative filter for the PhotoFiltre Unicode x64 ABI. MIT. */
#include <stdint.h>
#include <string.h>
#include "../prisme_pfl.h"
static PflHost host;
static PflHeader header;
PFL_EXPORT int32_t PFL_CALL RegisterCompatibility(void) { return 2; }
PFL_EXPORT PflHeader *PFL_CALL RegisterPluginHeader(void) {
  memset(&header,0,sizeof(header));
  memcpy(header.caption,L"Négatif PFL",sizeof(L"Négatif PFL"));
  memcpy(header.version,L"1.0",sizeof(L"1.0"));
  memcpy(header.author,L"Prisme",sizeof(L"Prisme"));
  memcpy(header.comment,L"Inverse les couleurs dans la sélection.",sizeof(L"Inverse les couleurs dans la sélection."));
  header.plugin_type=PFL_FILTER;header.flags=PFL_RGB_COLORS|PFL_NEED_IMAGE;
  return &header;
}
PFL_EXPORT PflHost *PFL_CALL RegisterPluginStruct(void) { return &host; }
PFL_EXPORT int32_t PFL_CALL Execute(void) {
  if(!host.image_exists)return 0;
  PflImage image={0};
  image.width=host.selection_rect.right-host.selection_rect.left;
  image.height=host.selection_rect.bottom-host.selection_rect.top;
  if(!host.create_bitmap(&image))return 0;
  int32_t result=0;
  host.begin_filter((const PflChar *)L"Négatif PFL",PFL_TRUE);
  if(host.get_selection_bitmap(image.bitmap)) {
    result=1;
    for(int32_t y=0;y<image.height;y++){
      uint8_t *row=host.get_scanline(&image,y);
      if(!row){result=0;break;}
      for(int32_t x=0;x<image.width*3;x++)row[x]=(uint8_t)(255-row[x]);
    }
    if(result)result=host.set_selection_bitmap(image.bitmap,0)!=0;
  }
  host.free_bitmap(&image);host.end_filter();return result;
}

