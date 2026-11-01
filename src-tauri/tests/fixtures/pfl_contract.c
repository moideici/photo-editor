/* Prisme PFL regression plugin. MIT. Real PhotoFiltre exports, no Prisme filter ABI. */
#include <windows.h>
#include <stdint.h>
#include <string.h>
#include "../../../plugin-sdk/pfl/prisme_pfl.h"
#ifndef MODE
#define MODE 0
#endif
#ifndef LEVEL
#define LEVEL 4
#endif
static PflHost host;
static PflHeader header;
static PflLanguage language;
PFL_EXPORT int32_t PFL_CALL RegisterCompatibility(void) { return LEVEL; }
PFL_EXPORT PflLanguage *PFL_CALL RegisterPluginLanguage(void) { return &language; }
PFL_EXPORT PflHeader *PFL_CALL RegisterPluginHeader(void) {
#if MODE == 15
  *(volatile int *)0 = 1;
#endif
  memset(&header,0,sizeof(header));
  memcpy(header.caption,L"Fixture PFL réelle",sizeof(L"Fixture PFL réelle"));
  memcpy(header.version,L"1.0",sizeof(L"1.0"));
  memcpy(header.author,L"Prisme",sizeof(L"Prisme"));
  memcpy(header.comment,L"Contrôle ABI Unicode x64",sizeof(L"Contrôle ABI Unicode x64"));
#if MODE == 11
  header.plugin_type=PFL_IMPORT;
#elif MODE == 12 || MODE == 16 || MODE == 20 || MODE == 24
  header.plugin_type=PFL_TOOL;
#elif MODE == 14 || MODE == 17
  header.plugin_type=PFL_EXPORT_IMAGE;
#elif MODE == 18 || MODE == 21 || MODE == 23
  header.plugin_type=PFL_IMAGE;
#else
  header.plugin_type=PFL_FILTER;
#endif
  header.flags=MODE==11?0:PFL_NEED_IMAGE;
  if(language.code[0]!='F')header.reserved[0]='X';
  return &header;
}
PFL_EXPORT PflHost *PFL_CALL RegisterPluginStruct(void) {
  if(LEVEL==2)*(uint64_t *)((uint8_t *)&host+0x138)=UINT64_C(0xf1f2f3f4f5f6f7f8);
  if(LEVEL==3)*(uint64_t *)((uint8_t *)&host+0x170)=UINT64_C(0xf1f2f3f4f5f6f7f8);
  return &host;
}
PFL_EXPORT HBITMAP PFL_CALL RegisterPluginGlyph(void) {
  BITMAPINFO info={0};void *data=NULL;
  info.bmiHeader.biSize=sizeof(BITMAPINFOHEADER);info.bmiHeader.biWidth=19;info.bmiHeader.biHeight=-19;
  info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=32;
  HBITMAP bitmap=CreateDIBSection(NULL,&info,DIB_RGB_COLORS,&data,NULL,0);
  if(bitmap){uint32_t *p=(uint32_t *)data;for(int y=0;y<19;y++)for(int x=0;x<19;x++)p[y*19+x]=(x>3&&x<15&&y>3&&y<15)?0x004080c0:0x00ffffff;}
  return bitmap;
}
static INT_PTR CALLBACK dialog_proc(HWND dialog,UINT msg,WPARAM w,LPARAM l) {
  (void)w;(void)l;
  if(msg==WM_INITDIALOG){host.prepare_dialog(dialog,MODE==25?1:0,PFL_TRUE,PFL_FALSE);SetTimer(dialog,1,200,NULL);return TRUE;}
  if(msg==WM_TIMER){KillTimer(dialog,1);EndDialog(dialog,IDOK);return TRUE;}
  if(msg==WM_CLOSE){EndDialog(dialog,IDCANCEL);return TRUE;}
  return FALSE;
}
static void dialog(void) {
  unsigned char buffer[512]={0};DLGTEMPLATE *d=(DLGTEMPLATE *)buffer;
  d->style=WS_POPUP|WS_CAPTION|WS_SYSMENU|DS_MODALFRAME;d->cx=180;d->cy=55;
  WORD *text=(WORD *)(d+1);*text++=0;*text++=0;
  const wchar_t *title=L"Prisme PFL fixture";while((*text++=*title++)!=0){}
  DialogBoxIndirectParamW(GetModuleHandleW(NULL),d,(HWND)host.main_window,dialog_proc,0);
}
PFL_EXPORT void PFL_CALL AboutPlugin(void) { dialog(); }
static int enumeration_count=0;
static PflBool PFL_CALL enum_image(const PflChar *name,int32_t w,int32_t h,int32_t count,PflColor transparent) {
 (void)transparent;if(!name||w<=0||h<=0||count<=0)return 0;enumeration_count++;return PFL_TRUE;
}
static PflBool PFL_CALL stop_enumeration(const PflChar *name,int32_t w,int32_t h,int32_t count,PflColor transparent) {
 (void)name;(void)w;(void)h;(void)count;(void)transparent;return PFL_FALSE;
}
static int read_api(void) {
 PflChar filename[512]={0},buffer[512]={0},tiny[3]={0,0,0x1234};
 if(host.get_image_filename(filename,512)!=filename||!filename[0])return 0;
 if(host.get_exe_path(buffer,512)!=buffer||!buffer[0])return 0;
 if(host.get_plugin_path(buffer,512)!=buffer||!buffer[0])return 0;
 if(host.get_ini_file(buffer,512)!=buffer||!buffer[0])return 0;
 if(host.get_open_path(buffer,512)!=buffer||!buffer[0])return 0;
 if(host.get_save_path(buffer,512)!=buffer||!buffer[0])return 0;
 host.get_plugin_path(tiny,2);if(tiny[1]!=0||tiny[2]!=0x1234)return 0;
 if(host.get_color_count()!=16777216||host.get_image_color_mode()!=2||host.get_transparent_color()!=0x1fffffff||host.get_image_dpi()!=72000)return 0;
 enumeration_count=0;if(!host.enumerate_images(enum_image)||enumeration_count!=1)return 0;
 if(host.enumerate_images(stop_enumeration))return 0;
 PflRect r={-5,-5,host.image_rect.right+5,host.image_rect.bottom+5};r=host.reset_undo_area(r);
 if(memcmp(&r,&host.image_rect,sizeof(r))!=0)return 0;
 if(host.get_selection_shape(NULL)!=0)return 0;
 host.disable_layers_painting(PFL_TRUE);host.disable_layers_painting(PFL_FALSE);
 PflImage image={0};image.width=host.image_rect.right;image.height=host.image_rect.bottom;
 if(!host.create_bitmap(&image))return 0;
 if(!host.get_bitmap(image.bitmap)||!host.get_bitmap_by_name(image.bitmap,filename)||!host.get_layer_bitmap(image.bitmap))return 0;
 if(!host.free_bitmap(&image))return 0;
 image.width=32;image.height=32;
 if(!host.create_bitmap(&image))return 0;
 if(!host.get_thumbnail(image.bitmap,0xffffff,2,2,0x777777,&r))return 0;
 if(r.left<0||r.top<0||r.right>32||r.bottom>32||r.right<=r.left||r.bottom<=r.top)return 0;
 if(!host.get_thumbnail_by_name(image.bitmap,filename,0xffffff,2,2,0x777777,&r))return 0;
 return host.free_bitmap(&image)!=0;
}
PFL_EXPORT int32_t PFL_CALL Execute(void) {
  if(LEVEL==2&&*(uint64_t *)((uint8_t *)&host+0x138)!=UINT64_C(0xf1f2f3f4f5f6f7f8))return 0;
  if(LEVEL==3&&*(uint64_t *)((uint8_t *)&host+0x170)!=UINT64_C(0xf1f2f3f4f5f6f7f8))return 0;
#if MODE == 2
  *(volatile int *)0=1;
#elif MODE == 3
  Sleep(10000);return 1;
#elif MODE == 5
  host.get_selection_bitmap((PflBitmap)(uintptr_t)0x1234);return 1;
#elif MODE == 10
  host.set_image_dpi(72000);return 1;
#elif MODE == 16
  return read_api();
#elif MODE == 17
  host.reset_save_menu();return host.set_image_filename((const PflChar *)L"export-test.png")!=0;
#elif MODE == 20
  {int32_t count=host.get_selection_shape(NULL);PflPoint points[1024]={0};
   if(count<=0||count>1024||host.get_selection_shape(points)!=count)return 0;
   return points[0].x==0&&points[0].y==0;}
#elif MODE == 21
  if(host.set_color_count(2)!=2||host.get_color_count()>2||host.get_image_color_mode()!=1)return 0;
  if(!host.set_transparent_color(0x604020)||host.get_transparent_color()!=0x604020)return 0;
  return 1;
#elif MODE == 24
  {PflImage image={0};image.width=13;image.height=7;
   if(!host.create_bitmap(&image))return 0;
   HBITMAP bitmap=(HBITMAP)host.release_bitmap_handle(&image);if(!bitmap)return 0;
   if(!host.free_bitmap(&image))return 0;
   BITMAP native={0};int ok=GetObjectW(bitmap,sizeof(native),&native)!=0&&native.bmWidth==13;
   return DeleteObject(bitmap)!=0&&ok;}
#elif MODE == 26
  host.show_form((void *)(uintptr_t)0x1234,0,PFL_TRUE,PFL_TRUE);return 1;
#elif MODE == 28
  return host.draw_selection_border(1,0x0000ff,PFL_TRUE)!=0;
#elif MODE == 29
  return host.set_application_state(2)==-1;
#endif
  host.begin_filter((const PflChar *)L"PFL en cours",PFL_TRUE);
  PflImage img={0};
#if MODE == 11 || MODE == 23
  img.width=33;img.height=17;
#elif MODE == 8 || MODE == 12 || MODE == 18
  img.width=host.image_rect.right-host.image_rect.left;img.height=host.image_rect.bottom-host.image_rect.top;
#elif MODE == 7
  PflRect bounds=host.get_layer_rect();PflPoint point=host.get_layer_position();
  if(bounds.left!=point.x||bounds.top!=point.y)return 0;
  img.width=bounds.right-bounds.left;img.height=bounds.bottom-bounds.top;
#else
  img.width=host.selection_rect.right-host.selection_rect.left;img.height=host.selection_rect.bottom-host.selection_rect.top;
#endif
  if(!host.create_bitmap(&img))return 0;
#if MODE == 4
  img.scanline_bytes++;
  host.get_scanline(&img,0);host.free_bitmap(&img);return 1;
#endif
  BITMAP native={0};
  if(GetObjectW((HBITMAP)img.handle,sizeof(native),&native)==0||native.bmWidth!=img.width||native.bmHeight!=img.height||native.bmBitsPixel!=24){host.free_bitmap(&img);return 0;}
#if MODE == 11 || MODE == 8 || MODE == 12 || MODE == 23
  for(int y=0;y<img.height;y++){uint8_t *row=host.get_scanline(&img,y);if(!row)return 0;
    for(int x=0;x<img.width;x++){
#if MODE == 11 || MODE == 23
      row[x*3]=17;row[x*3+1]=(uint8_t)(y*13);row[x*3+2]=(uint8_t)(x*7);
#else
      row[x*3]=row[x*3+1]=row[x*3+2]=x<img.width/2?255:0;
#endif
    }
  }
#if MODE == 11
  int result=host.new_image(img.bitmap,(const PflChar *)L"Image PFL importée",0,16777216,0x1fffffff);
#elif MODE == 23
  int result=host.set_bitmap(img.bitmap);
#else
  int result=host.set_selection_mask(img.bitmap,0);
#endif
#elif MODE == 18
  int result=host.get_selection_mask(img.bitmap)&&host.set_layer_bitmap(img.bitmap);
#elif MODE == 19
  int result=host.get_cropped_selection_mask(img.bitmap)&&host.set_selection_bitmap(img.bitmap,1);
#elif MODE == 7 || MODE == 30
#if MODE == 30
  if(!host.get_selection_alpha(img.bitmap))return 0;
#else
  if(!host.get_layer_alpha(img.bitmap))return 0;
#endif
  for(int y=0;y<img.height;y++){uint8_t *row=host.get_scanline(&img,y);for(int x=0;x<img.width*3;x++)row[x]=255-row[x];}
#if MODE == 30
  int result=host.set_selection_alpha(img.bitmap);
#else
  int result=host.set_layer_alpha(img.bitmap);
#endif
#else
  if(!host.get_selection_bitmap(img.bitmap))return 0;
  uint8_t *top=host.get_scanline(&img,0);
  if(img.height>1&&host.get_scanline(&img,1)!=top-img.scanline_bytes){host.free_bitmap(&img);return 0;}
  for(int y=0;y<img.height;y++){uint8_t *row=host.get_scanline(&img,y);for(int x=0;x<img.width*3;x++)row[x]=255-row[x];}
#if MODE == 9 || MODE == 25
  dialog();
#endif
  int result=host.set_selection_bitmap(img.bitmap,0);
#if MODE == 22
  result=result&&host.set_alpha_blend(50);
#endif
#endif
  host.free_bitmap(&img);host.end_filter();
#if MODE == 1
  return 0;
#else
  return result!=0;
#endif
}

