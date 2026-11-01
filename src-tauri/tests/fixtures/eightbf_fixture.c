/* Prisme regression fixture; upstream portion and license are in adjacent files.
 * No Adobe SDK. Not included in the application bundle. */
#include <windows.h>
#include "eightbf_upstream.c"

EXPORT void CrashMain(int16_t selector, FilterRecord *r, intptr_t *data, int16_t *result) {
  (void)r; (void)data; *result = noErr;
  if (selector == selStart) *(volatile int *)0 = 1;
}
EXPORT void TimeoutMain(int16_t selector, FilterRecord *r, intptr_t *data, int16_t *result) {
  (void)r; (void)data; *result = noErr;
  if (selector == selStart) Sleep(10000);
}
static INT_PTR CALLBACK fixture_dialog_proc(HWND dialog, UINT msg, WPARAM w, LPARAM l) {
  (void)w; (void)l;
  if (msg == WM_INITDIALOG) { SetTimer(dialog, 1, 250, NULL); return TRUE; }
  if (msg == WM_TIMER) { KillTimer(dialog, 1); EndDialog(dialog, IDOK); return TRUE; }
  if (msg == WM_CLOSE) { EndDialog(dialog, IDCANCEL); return TRUE; }
  return FALSE;
}
static void test_dialog(void) {
  unsigned char buffer[512] = {0};
  DLGTEMPLATE *d = (DLGTEMPLATE *)buffer;
  d->style = WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_MODALFRAME;
  d->cx = 180; d->cy = 55;
  WORD *text = (WORD *)(d + 1);
  *text++ = 0; /* menu */
  *text++ = 0; /* default dialog class */
  const wchar_t *title = L"Prisme 8BF fixture";
  while ((*text++ = *title++) != 0) { }
  DialogBoxIndirectParamW(GetModuleHandleW(NULL), d, NULL, fixture_dialog_proc, 0);
}
EXPORT void DialogMain(int16_t selector, FilterRecord *r, intptr_t *data, int16_t *result) {
  if (selector == 0) { test_dialog(); *result = noErr; return; }
  InvertMain(selector, r, data, result);
  if (selector == selParameters && *result == noErr) {
    test_dialog();
    Params *p = params(r);
    if (p) { p->amount = 37; r->handleProcs->unlockProc(r->parameters); }
  }
}

