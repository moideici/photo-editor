//! Callbacks PFL dans le processus enfant ; aucun objet VCL étranger n'est déréférencé.
use crate::{
    document::{Content, Document, Editor, SelShape},
    pfl_abi as a,
};
use image::{Rgba, RgbaImage};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    ffi::c_void,
    path::PathBuf,
    ptr,
    sync::Arc,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::Threading::GetCurrentProcessId,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};

struct Dib {
    width: u32,
    height: u32,
    stride: usize,
    bits: *mut u8,
    handle: HBITMAP,
}
impl Dib {
    fn new(width: u32, height: u32) -> Result<Self, String> {
        crate::io::dimensions(width, height)?;
        let stride = ((width as usize * 3) + 3) & !3;
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width as i32;
        info.bmiHeader.biHeight = height as i32;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 24;
        info.bmiHeader.biCompression = BI_RGB;
        let mut bits = ptr::null_mut();
        let handle = unsafe {
            CreateDIBSection(
                ptr::null_mut(),
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                ptr::null_mut(),
                0,
            )
        };
        if handle.is_null() || bits.is_null() {
            return Err("Création du DBmp Windows impossible".into());
        }
        unsafe { ptr::write_bytes(bits.cast::<u8>(), 0, stride * height as usize) };
        Ok(Self {
            width,
            height,
            stride,
            bits: bits.cast(),
            handle,
        })
    }
    fn bytes(&self) -> usize {
        self.stride * self.height as usize
    }
    fn row(&self, y: u32) -> Result<*mut u8, String> {
        if y >= self.height || self.handle.is_null() {
            return Err("Scanline DBmp hors limites ou handle détaché".into());
        }
        Ok(unsafe { self.bits.add((self.height - 1 - y) as usize * self.stride) })
    }
    fn set(&mut self, img: &RgbaImage) -> Result<(), String> {
        if img.dimensions() != (self.width, self.height) {
            return Err("Dimensions DBmp incompatibles".into());
        }
        for y in 0..self.height {
            let row = unsafe { std::slice::from_raw_parts_mut(self.row(y)?, self.stride) };
            for x in 0..self.width {
                let p = img.get_pixel(x, y);
                let i = x as usize * 3;
                row[i..i + 3].copy_from_slice(&[p[2], p[1], p[0]]);
            }
        }
        Ok(())
    }
    fn rgba(&self) -> Result<RgbaImage, String> {
        let mut image = RgbaImage::new(self.width, self.height);
        for y in 0..self.height {
            let row = unsafe { std::slice::from_raw_parts(self.row(y)?, self.stride) };
            for x in 0..self.width {
                let i = x as usize * 3;
                image.put_pixel(x, y, Rgba([row[i + 2], row[i + 1], row[i], 255]));
            }
        }
        Ok(image)
    }
}
impl Drop for Dib {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                DeleteObject(self.handle);
            }
        }
    }
}
pub struct Context {
    pub editor: Editor,
    pub original: Option<Document>,
    pub active_id: Option<u64>,
    pub plugin_type: i32,
    pub parent: isize,
    pub filename: String,
    pub plugin_path: PathBuf,
    pub config_path: PathBuf,
    pub changed: bool,
    pub new_document: bool,
    pub touched: BTreeSet<u64>,
    pub calls: BTreeSet<String>,
    pub errors: Vec<String>,
    pub message: String,
    dibs: BTreeMap<usize, Box<Dib>>,
}
impl Context {
    pub fn new(
        editor: Editor,
        active_id: Option<u64>,
        plugin_type: i32,
        parent: isize,
        filename: String,
        plugin_path: PathBuf,
        config_path: PathBuf,
    ) -> Self {
        let original = editor.doc.clone();
        Self {
            editor,
            original,
            active_id,
            plugin_type,
            parent,
            filename,
            plugin_path,
            config_path,
            changed: false,
            new_document: false,
            touched: BTreeSet::new(),
            calls: BTreeSet::new(),
            errors: vec![],
            message: String::new(),
            dibs: BTreeMap::new(),
        }
    }
    fn editable(&self, selection: bool) -> Result<(), String> {
        if self.new_document
            || matches!(self.plugin_type, 0 | 1)
            || (selection && self.plugin_type == 4)
        {
            Ok(())
        } else {
            Err("Ce type de plugin PFL ne peut pas modifier le document courant".into())
        }
    }
    fn active(&self) -> Result<usize, String> {
        self.editor.layer_index(
            self.active_id
                .ok_or("Aucun calque actif pour le plugin PFL")?,
        )
    }
    fn layer_rect(&self) -> Result<a::Rect, String> {
        let l = &self.editor.doc()?.layers[self.active()?];
        let img = l
            .raster()
            .ok_or("Rastérisez le calque avant le plugin PFL")?;
        Ok(a::Rect {
            left: l.x,
            top: l.y,
            right: l.x.saturating_add(img.width() as i32),
            bottom: l.y.saturating_add(img.height() as i32),
        })
    }
    pub fn image_rect(&self) -> a::Rect {
        self.editor
            .doc
            .as_ref()
            .map(|d| a::Rect::image(d.width, d.height))
            .unwrap_or_default()
    }
    pub fn selection_rect(&self) -> a::Rect {
        let Some(d) = &self.editor.doc else {
            return a::Rect::default();
        };
        d.selection
            .as_ref()
            .map(|s| {
                s.bounds(d.width, d.height)
                    .map(|(x, y, w, h)| a::Rect {
                        left: x as i32,
                        top: y as i32,
                        right: (x + w) as i32,
                        bottom: (y + h) as i32,
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_else(|| self.image_rect())
    }
    fn mask(&self) -> Result<Vec<u8>, String> {
        let d = self.editor.doc()?;
        Ok(d.selection
            .as_ref()
            .map(|s| s.mask(d.width, d.height))
            .unwrap_or_else(|| vec![255; d.width as usize * d.height as usize]))
    }
    fn dib(&self, token: a::Bitmap) -> Result<&Dib, String> {
        self.dibs.get(&(token as usize)).map(|b| &**b).ok_or(
            "Objet bitmap PFL inconnu : seuls les DBmp créés par cet hôte sont compatibles".into(),
        )
    }
    fn dib_mut(&mut self, token: a::Bitmap) -> Result<&mut Dib, String> {
        self.dibs
            .get_mut(&(token as usize))
            .map(|b| &mut **b)
            .ok_or("Objet bitmap PFL inconnu : objets VCL externes non pris en charge".into())
    }
    fn read_layer(&self, rect: a::Rect, alpha: bool) -> Result<RgbaImage, String> {
        let d = self.editor.doc()?;
        let l = &d.layers[self.active()?];
        let src = l.raster().ok_or("Calque bitmap requis")?;
        let mut img = RgbaImage::from_pixel(rect.width(), rect.height(), Rgba([255; 4]));
        for y in 0..img.height() {
            for x in 0..img.width() {
                let sx = x as i64 + rect.left as i64 - l.x as i64;
                let sy = y as i64 + rect.top as i64 - l.y as i64;
                if sx >= 0 && sy >= 0 && sx < src.width() as i64 && sy < src.height() as i64 {
                    let p = *src.get_pixel(sx as u32, sy as u32);
                    img.put_pixel(
                        x,
                        y,
                        if alpha {
                            let v = 255 - p[3];
                            Rgba([v, v, v, 255])
                        } else {
                            Rgba([p[0], p[1], p[2], 255])
                        },
                    );
                }
            }
        }
        Ok(img)
    }
    fn replace_layer(&mut self, idx: usize, mut pixels: RgbaImage) -> Result<(), String> {
        if let Some(indexed) = &self.editor.doc()?.indexed {
            let before = self.editor.doc()?.layers[idx]
                .raster()
                .ok_or("Bitmap indexé requis")?;
            let next = indexed.map_edit(before, &pixels)?;
            pixels = next.expand(pixels.width(), pixels.height());
            self.editor.doc_mut()?.indexed = Some(next);
        }
        let id = self.editor.doc()?.layers[idx].id;
        self.editor.doc_mut()?.layers[idx].content = Content::Raster(Arc::new(pixels));
        self.editor.touch(idx);
        self.touched.insert(id);
        self.changed = true;
        Ok(())
    }
    fn write_layer(
        &mut self,
        rect: a::Rect,
        src: &RgbaImage,
        alpha: bool,
        masked: bool,
    ) -> Result<(), String> {
        self.editable(false)?;
        if (rect.width(), rect.height()) != src.dimensions() {
            return Err("Dimensions du bitmap incompatibles avec la région PFL".into());
        }
        let idx = self.active()?;
        let d = self.editor.doc()?;
        let l = &d.layers[idx];
        let (lx, ly) = (l.x, l.y);
        let mut dest = (**l.raster().ok_or("Calque bitmap requis")?).clone();
        let mask = if masked { Some(self.mask()?) } else { None };
        let dw = d.width as i64;
        let dh = d.height as i64;
        for y in 0..src.height() {
            for x in 0..src.width() {
                let dx = x as i64 + rect.left as i64;
                let dy = y as i64 + rect.top as i64;
                let tx = dx - lx as i64;
                let ty = dy - ly as i64;
                if tx < 0 || ty < 0 || tx >= dest.width() as i64 || ty >= dest.height() as i64 {
                    continue;
                }
                let weight = mask
                    .as_ref()
                    .map(|m| {
                        if dx >= 0 && dy >= 0 && dx < dw && dy < dh {
                            m[(dy * dw + dx) as usize] as u32
                        } else {
                            0
                        }
                    })
                    .unwrap_or(255);
                let p = src.get_pixel(x, y);
                let q = dest.get_pixel_mut(tx as u32, ty as u32);
                if alpha {
                    let a = 255 - p[0] as u32;
                    q[3] = ((q[3] as u32 * (255 - weight) + a * weight + 127) / 255) as u8;
                } else {
                    for c in 0..3 {
                        q[c] = ((q[c] as u32 * (255 - weight) + p[c] as u32 * weight + 127) / 255)
                            as u8;
                    }
                }
            }
        }
        self.replace_layer(idx, dest)
    }
    fn read_mask(&self, rect: a::Rect) -> Result<RgbaImage, String> {
        let d = self.editor.doc()?;
        let mask = self.mask()?;
        let mut image = RgbaImage::new(rect.width(), rect.height());
        for y in 0..image.height() {
            for x in 0..image.width() {
                let dx = x as i64 + rect.left as i64;
                let dy = y as i64 + rect.top as i64;
                let v = if dx >= 0 && dy >= 0 && dx < d.width as i64 && dy < d.height as i64 {
                    mask[(dy * d.width as i64 + dx) as usize]
                } else {
                    0
                };
                image.put_pixel(x, y, Rgba([v, v, v, 255]));
            }
        }
        Ok(image)
    }
    fn restore(&mut self) -> Result<(), String> {
        self.editable(false)?;
        self.editor.doc = self.original.clone();
        self.changed = false;
        self.touched.clear();
        Ok(())
    }
    fn color_count(&self) -> i32 {
        self.editor
            .doc
            .as_ref()
            .map(|d| {
                d.indexed
                    .as_ref()
                    .map(|i| i.palette.len() as i32)
                    .unwrap_or(16777216)
            })
            .unwrap_or(-1)
    }
    fn rgba_mode(&self) -> bool {
        self.editor.doc.as_ref().is_some_and(|d| {
            d.layers
                .iter()
                .any(|l| l.raster().is_some_and(|i| i.pixels().any(|p| p[3] != 255)))
        })
    }
}
thread_local! {static CONTEXT:RefCell<Option<Context>>=const{RefCell::new(None)};}
pub fn install(context: Context) -> Result<(), String> {
    CONTEXT.with(|c| {
        let mut c = c.borrow_mut();
        if c.is_some() {
            return Err("Hôte PFL déjà actif".into());
        }
        *c = Some(context);
        Ok(())
    })
}
pub fn take() -> Result<Context, String> {
    CONTEXT.with(|c| c.borrow_mut().take().ok_or("Contexte PFL absent".into()))
}
fn record_error(error: String) {
    CONTEXT.with(|c| {
        if let Ok(mut c) = c.try_borrow_mut() {
            if let Some(c) = c.as_mut() {
                if c.errors.len() < 8 {
                    c.errors.push(error)
                }
            }
        }
    })
}
fn call<T>(name: &str, fallback: T, f: impl FnOnce(&mut Context) -> Result<T, String>) -> T {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        CONTEXT.with(|slot| {
            let mut slot = slot
                .try_borrow_mut()
                .map_err(|_| "Callback PFL réentrant non pris en charge".to_string())?;
            let c = slot.as_mut().ok_or("Contexte PFL absent")?;
            c.calls.insert(name.into());
            f(c)
        })
    }));
    match result {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            record_error(format!("{name}: {e}"));
            fallback
        }
        Err(_) => {
            record_error(format!("{name}: erreur interne du callback"));
            fallback
        }
    }
}
unsafe fn wide_input(p: *const u16, max: usize) -> Result<String, String> {
    if p.is_null() {
        return Ok(String::new());
    }
    let mut data = Vec::new();
    for i in 0..max {
        let v = ptr::read_unaligned(p.add(i));
        if v == 0 {
            return String::from_utf16(&data).map_err(|_| "UTF-16 invalide".into());
        }
        data.push(v)
    }
    Err("Chaîne PFL sans terminaison ou trop longue".into())
}
unsafe fn write_string(p: *mut u16, max: i32, value: &str) -> Result<*mut u16, String> {
    if p.is_null() || !(1..=32768).contains(&max) {
        return Err("Tampon de chaîne PFL invalide".into());
    }
    let wide = a::wide(value);
    let len = (wide.len() - 1).min(max as usize - 1);
    ptr::copy_nonoverlapping(wide.as_ptr(), p, len);
    *p.add(len) = 0;
    Ok(p)
}
unsafe extern "system" fn begin_filter(message: *const u16, _cursor: a::Bool) {
    call("BeginFilter", (), |c| {
        c.message = wide_input(message, 2048)?;
        Ok(())
    });
}
unsafe extern "system" fn end_filter() {
    call("EndFilter", (), |_| Ok(()));
}
unsafe extern "system" fn create_bitmap(img: *mut a::Image) -> i32 {
    call("CreateDBmp", 0, |c| {
        let img = img.as_mut().ok_or("Structure DBmp nulle")?;
        let (w, h) = (
            u32::try_from(img.width).map_err(|_| "Largeur DBmp invalide")?,
            u32::try_from(img.height).map_err(|_| "Hauteur DBmp invalide")?,
        );
        if !img.bitmap.is_null() && c.dibs.contains_key(&(img.bitmap as usize)) {
            return Err("DBmp déjà créé : libérez-le avant de le remplacer".into());
        }
        let bytes = (((w as u64 * 3) + 3) & !3) * h as u64;
        if c.dibs.len() >= 64
            || c.dibs.values().map(|b| b.bytes() as u64).sum::<u64>() + bytes > 256 * 1024 * 1024
        {
            return Err("Budget DBmp dépassé (64 bitmaps / 256 Mio)".into());
        }
        let mut dib = Box::new(Dib::new(w, h)?);
        let token = (&mut *dib as *mut Dib).cast::<c_void>();
        img.size = std::mem::size_of::<a::Image>() as i32;
        img.width_bytes = (w * 3) as i32;
        img.scanline_bytes = dib.stride as i32;
        img.bytes_per_pixel = 3;
        img.data_bytes = dib.bytes() as i32;
        img.buffer = dib.row(0)?;
        img.bitmap = token;
        img.handle = dib.handle;
        c.dibs.insert(token as usize, dib);
        Ok(1)
    })
}
unsafe extern "system" fn free_bitmap(img: *mut a::Image) -> i32 {
    call("FreeDBmp", 0, |c| {
        let img = img.as_mut().ok_or("Structure DBmp nulle")?;
        if c.dibs.remove(&(img.bitmap as usize)).is_none() {
            return Err("DBmp inconnu ou déjà libéré".into());
        }
        img.bitmap = ptr::null_mut();
        img.handle = ptr::null_mut();
        img.buffer = ptr::null_mut();
        Ok(1)
    })
}
unsafe extern "system" fn release_handle(img: *mut a::Image) -> a::Bitmap {
    call("ReleaseHandleDBmp", ptr::null_mut(), |c| {
        let img = img.as_mut().ok_or("Structure DBmp nulle")?;
        let dib = c.dib_mut(img.bitmap)?;
        let handle = dib.handle;
        if handle.is_null() {
            return Err("Handle DBmp déjà détaché".into());
        }
        dib.handle = ptr::null_mut();
        Ok(handle)
    })
}
unsafe extern "system" fn get_scanline(img: *mut a::Image, row: i32) -> *mut u8 {
    call("GetScanlineDBmp", ptr::null_mut(), |c| {
        let img = img.as_mut().ok_or("Structure DBmp nulle")?;
        let dib = c.dib(img.bitmap)?;
        if img.size != 56
            || img.width != dib.width as i32
            || img.height != dib.height as i32
            || img.scanline_bytes != dib.stride as i32
            || img.width_bytes != dib.width as i32 * 3
            || img.bytes_per_pixel != 3
            || img.data_bytes != dib.bytes() as i32
        {
            return Err("Le plugin a altéré les dimensions ou le contrat DBmp".into());
        }
        let row = u32::try_from(row).map_err(|_| "Ligne DBmp négative")?;
        img.buffer = dib.row(row)?;
        Ok(img.buffer)
    })
}
unsafe extern "system" fn get_bitmap(bitmap: a::Bitmap) -> a::Bool {
    call("GetBitmap", 0, |c| {
        let mut image = crate::vector::flatten(c.editor.doc()?)?;
        for p in image.pixels_mut() {
            for ch in 0..3 {
                p[ch] = ((p[ch] as u32 * p[3] as u32 + 255 * (255 - p[3] as u32) + 127) / 255) as u8
            }
            p[3] = 255
        }
        c.dib_mut(bitmap)?.set(&image)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn set_bitmap(bitmap: a::Bitmap) -> a::Bool {
    call("SetBitmap", 0, |c| {
        if bitmap.is_null() {
            c.restore()?;
            return Ok(a::TRUE);
        }
        c.editable(false)?;
        let image = c.dib(bitmap)?.rgba()?;
        let d = c.editor.doc()?;
        let same = image.dimensions() == (d.width, d.height);
        if !same && (c.plugin_type != 1 || d.layers.len() != 1 || c.rgba_mode()) {
            return Err("Redimensionnement PFL réservé au type Image, fond RVB unique".into());
        }
        let (w, h) = image.dimensions();
        let old = d.layers[0]
            .raster()
            .ok_or("Le fond doit être un calque bitmap")?;
        let mut changed = image;
        if same && old.dimensions() == (w, h) {
            for (p, o) in changed.pixels_mut().zip(old.pixels()) {
                p[3] = o[3]
            }
        }
        if !same {
            let d = c.editor.doc_mut()?;
            d.width = w;
            d.height = h;
            d.selection = None;
            d.selections.clear();
            d.indexed = None
        }
        c.editor.doc_mut()?.layers[0].x = 0;
        c.editor.doc_mut()?.layers[0].y = 0;
        c.replace_layer(0, changed)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn get_selection(bitmap: a::Bitmap) -> a::Bool {
    call("GetSelBitmap", 0, |c| {
        let image = c.read_layer(c.selection_rect(), false)?;
        c.dib_mut(bitmap)?.set(&image)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn set_selection(bitmap: a::Bitmap, flag: i32) -> a::Bool {
    call("SetSelBitmap", 0, |c| {
        if !matches!(flag, 0 | 1) {
            return Err("Option de masque PFL inconnue".into());
        }
        let image = c.dib(bitmap)?.rgba()?;
        c.write_layer(c.selection_rect(), &image, false, flag == 0)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn get_mask(bitmap: a::Bitmap) -> a::Bool {
    call("GetSelMask", 0, |c| {
        let image = c.read_mask(c.image_rect())?;
        c.dib_mut(bitmap)?.set(&image)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn get_cropped_mask(bitmap: a::Bitmap) -> a::Bool {
    call("GetSelSelMask", 0, |c| {
        let image = c.read_mask(c.selection_rect())?;
        c.dib_mut(bitmap)?.set(&image)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn get_layer_rect() -> a::Rect {
    call("GetLayerRect", a::Rect::default(), |c| c.layer_rect())
}
unsafe extern "system" fn get_layer_position() -> a::Point {
    call("GetLayerPos", a::Point::default(), |c| {
        let r = c.layer_rect()?;
        Ok(a::Point {
            x: r.left,
            y: r.top,
        })
    })
}
unsafe extern "system" fn get_layer(bitmap: a::Bitmap) -> a::Bool {
    call("GetLayerBitmap", 0, |c| {
        let image = c.read_layer(c.layer_rect()?, false)?;
        c.dib_mut(bitmap)?.set(&image)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn set_layer(bitmap: a::Bitmap) -> a::Bool {
    call("SetLayerBitmap", 0, |c| {
        let image = c.dib(bitmap)?.rgba()?;
        c.write_layer(c.layer_rect()?, &image, false, false)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn get_alpha(bitmap: a::Bitmap) -> a::Bool {
    call("GetLayerAlpha", 0, |c| {
        let image = c.read_layer(c.layer_rect()?, true)?;
        c.dib_mut(bitmap)?.set(&image)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn set_alpha(bitmap: a::Bitmap) -> a::Bool {
    call("SetLayerAlpha", 0, |c| {
        let image = c.dib(bitmap)?.rgba()?;
        c.write_layer(c.layer_rect()?, &image, true, false)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn get_selection_alpha(bitmap: a::Bitmap) -> a::Bool {
    call("GetSelAlpha", 0, |c| {
        let image = c.read_layer(c.selection_rect(), true)?;
        c.dib_mut(bitmap)?.set(&image)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn set_selection_alpha(bitmap: a::Bitmap) -> a::Bool {
    call("SetSelAlpha", 0, |c| {
        let image = c.dib(bitmap)?.rgba()?;
        c.write_layer(c.selection_rect(), &image, true, true)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn get_color_count() -> i32 {
    call("GetColorCount", -1, |c| Ok(c.color_count()))
}
unsafe extern "system" fn set_color_count(count: i32) -> i32 {
    call("SetColorCount", 0, |c| {
        c.editable(false)?;
        if count == 16777216 {
            if c.editor.doc_mut()?.indexed.take().is_some() {
                c.changed = true
            }
            return Ok(count);
        }
        if !(2..=256).contains(&count) {
            return Err("Nombre de couleurs PFL non pris en charge".into());
        }
        let d = c.editor.doc()?;
        if d.layers.len() != 1 {
            return Err("Conversion indexée PFL : une seule couche de fond requise".into());
        }
        let image = crate::vector::flatten(d)?;
        let indexed = crate::indexed::quantize(&image, count as usize, false)?;
        let pixels = indexed.expand(d.width, d.height);
        c.editor.doc_mut()?.layers[0].content = Content::Raster(Arc::new(pixels));
        c.editor.doc_mut()?.indexed = Some(indexed);
        c.editor.touch(0);
        c.touched.insert(c.editor.doc()?.layers[0].id);
        c.changed = true;
        Ok(count)
    })
}
unsafe extern "system" fn get_transparent() -> i32 {
    call("GetTransparentColor", a::CL_NONE, |c| {
        Ok(c.editor
            .doc
            .as_ref()
            .and_then(|d| d.indexed.as_ref())
            .and_then(|i| i.palette.iter().find(|p| p[3] == 0))
            .map(|p| p[0] as i32 | (p[1] as i32) << 8 | (p[2] as i32) << 16)
            .unwrap_or(a::CL_NONE))
    })
}
unsafe extern "system" fn set_transparent(color: i32) -> a::Bool {
    call("SetTransparentColor", 0, |c| {
        c.editable(false)?;
        let d = c.editor.doc_mut()?;
        let Some(indexed) = d.indexed.as_mut() else {
            return Ok(a::FALSE);
        };
        for p in &mut indexed.palette {
            p[3] = if color != a::CL_NONE
                && (p[0] as i32 | (p[1] as i32) << 8 | (p[2] as i32) << 16) == color
            {
                0
            } else {
                255
            }
        }
        let pixels = indexed.expand(d.width, d.height);
        d.layers[0].content = Content::Raster(Arc::new(pixels));
        let id = d.layers[0].id;
        c.editor.touch(0);
        c.touched.insert(id);
        c.changed = true;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn alpha_blend(alpha: i32) -> a::Bool {
    call("SetAlphaBlend", 0, |c| {
        c.editable(false)?;
        if !(0..=100).contains(&alpha) {
            return Err("AlphaBlend hors de 0–100".into());
        }
        let idx = c.active()?;
        let id = c.editor.doc()?.layers[idx].id;
        let old = c
            .original
            .as_ref()
            .and_then(|d| d.layers.iter().find(|l| l.id == id))
            .and_then(|l| l.raster())
            .ok_or("Bitmap original absent")?;
        let mut image = (**c.editor.doc()?.layers[idx]
            .raster()
            .ok_or("Bitmap requis")?)
        .clone();
        if old.dimensions() != image.dimensions() {
            return Err("Dimensions modifiées avant AlphaBlend".into());
        }
        for (p, o) in image.pixels_mut().zip(old.pixels()) {
            for k in 0..4 {
                p[k] = ((p[k] as i32 * (100 - alpha) + o[k] as i32 * alpha + 50) / 100) as u8
            }
        }
        c.replace_layer(idx, image)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn reset_save() {
    call("ResetSaveMenu", (), |_| Ok(()));
}
unsafe extern "system" fn image_filename(p: *mut u16, n: i32) -> *mut u16 {
    call("GetImgFileName", ptr::null_mut(), |c| {
        write_string(p, n, &c.filename)
    })
}
unsafe extern "system" fn exe_path(p: *mut u16, n: i32) -> *mut u16 {
    call("GetExePath", ptr::null_mut(), |_| {
        let path = std::env::current_exe().map_err(|e| e.to_string())?;
        write_string(
            p,
            n,
            &path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_string_lossy(),
        )
    })
}
unsafe extern "system" fn plugin_path(p: *mut u16, n: i32) -> *mut u16 {
    call("GetPluginPath", ptr::null_mut(), |c| {
        write_string(p, n, &c.plugin_path.to_string_lossy())
    })
}
unsafe extern "system" fn ini_file(p: *mut u16, n: i32) -> *mut u16 {
    call("GetIniFile", ptr::null_mut(), |c| {
        write_string(p, n, &c.config_path.join("pfl.ini").to_string_lossy())
    })
}
unsafe extern "system" fn open_path(p: *mut u16, n: i32) -> *mut u16 {
    call("GetOpenPath", ptr::null_mut(), |c| {
        let path = std::path::Path::new(&c.filename)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(&c.plugin_path);
        write_string(p, n, &path.to_string_lossy())
    })
}
unsafe extern "system" fn save_path(p: *mut u16, n: i32) -> *mut u16 {
    call("GetSavePath", ptr::null_mut(), |c| {
        let path = std::path::Path::new(&c.filename)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(&c.plugin_path);
        write_string(p, n, &path.to_string_lossy())
    })
}
unsafe extern "system" fn set_filename(p: *const u16) -> a::Bool {
    call("SetImgFileName", 0, |c| {
        if c.plugin_type != 3 {
            return Err("SetImgFileName est réservé aux exports PFL".into());
        }
        c.filename = wide_input(p, 32768)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn bitmap_by_name(bitmap: a::Bitmap, name: *const u16) -> a::Bool {
    let matches = call("GetBitmapByName", false, |c| {
        Ok(c.filename == wide_input(name, 32768)?)
    });
    if matches {
        get_bitmap(bitmap)
    } else {
        a::FALSE
    }
}
unsafe extern "system" fn get_dpi() -> i32 {
    call("GetImageDPI", 0, |c| {
        Ok(if c.editor.doc.is_some() { 72000 } else { 0 })
    })
}
unsafe extern "system" fn set_dpi(_dpi: i32) -> i32 {
    call("SetImageDPI", 0, |_| {
        Err("La résolution du document n’est pas encore éditable dans Prisme".into())
    })
}
unsafe extern "system" fn reset_undo(rect: a::Rect) -> a::Rect {
    call("ResetUndoArea", a::Rect::default(), |c| {
        Ok(rect.intersect(c.image_rect()))
    })
}
unsafe extern "system" fn disable_layers(_disable: a::Bool) {
    call("DisableLayersPainting", (), |_| Ok(()));
}
unsafe extern "system" fn color_mode() -> i32 {
    call("GetImageColorMode", 0, |c| {
        Ok(match &c.editor.doc {
            None => 0,
            Some(d) if d.indexed.is_some() => 1,
            Some(_) if c.rgba_mode() => 4,
            Some(d) if d.layers.len() > 1 => 3,
            _ => 2,
        })
    })
}
unsafe extern "system" fn show_form(
    _form: a::Bitmap,
    _mode: i32,
    _center: a::Bool,
    _wait: a::Bool,
) {
    call("ShowForm", (), |_| {
        Err("Les fiches VCL ne sont pas compatibles ; utilisez PrepareDialog avec une fenêtre Win32".into())
    });
}
unsafe extern "system" fn application_state(_state: i32) -> i32 {
    call("SetApplicationState", -1, |_| {
        Err(
            "Le changement d’état global de l’application par un PFL n’est pas pris en charge"
                .into(),
        )
    })
}
unsafe extern "system" fn set_mask(bitmap: a::Bitmap, flag: i32) -> a::Bool {
    call("SetSelMask", 0, |c| {
        c.editable(true)?;
        let image = c.dib(bitmap)?.rgba()?;
        let d = c.editor.doc()?;
        let mut data = vec![0; d.width as usize * d.height as usize];
        let (x, y) = if flag == 0 {
            if image.dimensions() != (d.width, d.height) {
                return Err("Le masque PFL doit avoir la taille du document".into());
            }
            (0, 0)
        } else {
            let r = c.layer_rect()?;
            if image.dimensions() != (r.width(), r.height()) {
                return Err("Le masque PFL doit avoir la taille du calque".into());
            }
            (r.left, r.top)
        };
        for sy in 0..image.height() {
            for sx in 0..image.width() {
                let dx = sx as i64 + x as i64;
                let dy = sy as i64 + y as i64;
                if dx >= 0 && dy >= 0 && dx < d.width as i64 && dy < d.height as i64 {
                    let p = image.get_pixel(sx, sy);
                    data[(dy * d.width as i64 + dx) as usize] =
                        ((p[0] as u32 + p[1] as u32 + p[2] as u32 + 1) / 3) as u8
                }
            }
        }
        let (w, h) = (d.width, d.height);
        c.editor.doc_mut()?.selection = Some(crate::selection::from_mask(w, h, data));
        c.changed = true;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn selection_shape(points: *mut a::Point) -> i32 {
    call("GetSelShape", 0, |c| {
        let d = c.editor.doc()?;
        let Some(s) = &d.selection else { return Ok(0) };
        match &s.shape {
            SelShape::Rect { .. } => Ok(-1),
            SelShape::Ellipse { .. } => Ok(-2),
            SelShape::Mask { .. } => Ok(0),
            SelShape::Polygon { points: poly } => {
                if s.inverted {
                    return Ok(0);
                }
                let rect = c.selection_rect();
                if !points.is_null() {
                    for (i, p) in poly.iter().enumerate() {
                        ptr::write_unaligned(
                            points.add(i),
                            a::Point {
                                x: p[0].round() as i32 - rect.left,
                                y: p[1].round() as i32 - rect.top,
                            },
                        )
                    }
                }
                Ok(poly.len() as i32)
            }
        }
    })
}
unsafe extern "system" fn draw_border(width: i32, color: i32, antialias: a::Bool) -> a::Bool {
    call("DrawSelBorder", 0, |c| {
        c.editable(false)?;
        if !(1..=1000).contains(&width) || color < 0 {
            return Err("Contour PFL invalide".into());
        }
        let d = c.editor.doc()?;
        let selection = d
            .selection
            .clone()
            .ok_or("Une sélection est nécessaire au contour")?;
        let border =
            crate::selection::modify(&selection, d.width, d.height, "border", width as f32)?;
        let mut mask = border.mask(d.width, d.height);
        if antialias == 0 {
            for m in &mut mask {
                *m = if *m >= 128 { 255 } else { 0 }
            }
        }
        let idx = c.active()?;
        let l = &d.layers[idx];
        let mut image = (**l.raster().ok_or("Calque bitmap requis")?).clone();
        for y in 0..image.height() {
            for x in 0..image.width() {
                let dx = x as i64 + l.x as i64;
                let dy = y as i64 + l.y as i64;
                if dx < 0 || dy < 0 || dx >= d.width as i64 || dy >= d.height as i64 {
                    continue;
                }
                let m = mask[(dy * d.width as i64 + dx) as usize] as u32;
                let p = image.get_pixel_mut(x, y);
                for k in 0..3 {
                    let value = ((color as u32 >> (8 * k)) & 255) as u32;
                    p[k] = ((p[k] as u32 * (255 - m) + value * m + 127) / 255) as u8
                }
            }
        }
        c.replace_layer(idx, image)?;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn new_image(
    bitmap: a::Bitmap,
    name: *const u16,
    flag: i32,
    count: i32,
    transparent: i32,
) -> a::Bool {
    call("NewImage", 0, |c| {
        if !matches!(flag, 0 | 1) {
            return Err("Option de création d’image PFL inconnue".into());
        }
        if !matches!(c.plugin_type, 2 | 4) {
            return Err("NewImage est réservé aux types Import et Outil".into());
        }
        if c.original.is_some() || c.new_document {
            return Err("Ce plugin crée une nouvelle image : fermez le document courant après sauvegarde ; les onglets multiples sont prévus dans le plan".into());
        }
        let mut image = c.dib(bitmap)?.rgba()?;
        if transparent != a::CL_NONE {
            for p in image.pixels_mut() {
                if (p[0] as i32 | (p[1] as i32) << 8 | (p[2] as i32) << 16) == transparent {
                    p[3] = 0
                }
            }
        }
        let (w, h) = image.dimensions();
        let title = wide_input(name, 32768)?;
        let mut indexed = None;
        if count != 16777216 {
            if !(2..=256).contains(&count) {
                return Err("Nombre de couleurs de la nouvelle image invalide".into());
            }
            let palette = crate::indexed::quantize(&image, count as usize, false)?;
            image = palette.expand(w, h);
            indexed = Some(palette);
        }
        let layer = c.editor.new_raster_layer("Image du plugin", image, 0, 0);
        c.active_id = Some(layer.id);
        c.touched.insert(layer.id);
        c.editor.doc = Some(Document {
            text_styles: Vec::new(),
            width: w,
            height: h,
            layers: vec![layer],
            selection: None,
            selections: Default::default(),
            name: if title.is_empty() {
                "Image PFL".into()
            } else {
                title
            },
            indexed,
            animation: None,
            metadata: Default::default(),
        });
        c.new_document = true;
        c.filename = c.editor.doc()?.name.clone();
        c.changed = true;
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn enumerate_images(
    proc: Option<unsafe extern "system" fn(*const u16, i32, i32, i32, i32) -> a::Bool>,
) -> a::Bool {
    let data = call("EnumImages", None, |c| {
        Ok(c.editor.doc.as_ref().map(|d| {
            (
                a::wide(&c.filename),
                d.width as i32,
                d.height as i32,
                c.color_count(),
            )
        }))
    });
    let Some(proc) = proc else {
        record_error("EnumImages: callback nul".into());
        return 0;
    };
    if let Some((name, w, h, count)) = data {
        if proc(name.as_ptr(), w, h, count, a::CL_NONE) == 0 {
            return a::FALSE;
        }
    }
    a::TRUE
}
unsafe extern "system" fn thumbnail(
    bitmap: a::Bitmap,
    bg: i32,
    border: i32,
    shadow: i32,
    shadow_color: i32,
    rect: *mut a::Rect,
) -> a::Bool {
    call("GetThumb", 0, |c| {
        if rect.is_null() || border < 0 || border > 50 || shadow < 0 || shadow > 10 {
            return Err("Paramètres de miniature PFL invalides".into());
        }
        let source = crate::vector::flatten(c.editor.doc()?)?;
        let dest = c.dib(bitmap)?;
        let margin = border + shadow;
        let inner_w = dest.width as i64 - 2 * margin as i64;
        let inner_h = dest.height as i64 - 2 * margin as i64;
        if inner_w <= 0 || inner_h <= 0 || bg < 0 || shadow_color < 0 {
            return Err(
                "DBmp de miniature trop petit ou couleur système non prise en charge".into(),
            );
        }
        let scale =
            (inner_w as f64 / source.width() as f64).min(inner_h as f64 / source.height() as f64);
        let w = (source.width() as f64 * scale).round().max(1.0) as u32;
        let h = (source.height() as f64 * scale).round().max(1.0) as u32;
        let resized = image::imageops::resize(&source, w, h, image::imageops::FilterType::Lanczos3);
        let rgb = |v: i32| Rgba([v as u8, (v >> 8) as u8, (v >> 16) as u8, 255]);
        let mut out = RgbaImage::from_pixel(dest.width, dest.height, rgb(bg));
        let x = (dest.width - w) / 2;
        let y = (dest.height - h) / 2;
        if shadow > 0 {
            for sy in y.saturating_add(shadow as u32)..(y + h + shadow as u32).min(out.height()) {
                for sx in x.saturating_add(shadow as u32)..(x + w + shadow as u32).min(out.width())
                {
                    out.put_pixel(sx, sy, rgb(shadow_color));
                }
            }
        }
        image::imageops::overlay(&mut out, &resized, x as i64, y as i64);
        c.dib_mut(bitmap)?.set(&out)?;
        *rect = a::Rect {
            left: x as i32,
            top: y as i32,
            right: (x + w) as i32,
            bottom: (y + h) as i32,
        };
        Ok(a::TRUE)
    })
}
unsafe extern "system" fn thumbnail_by_name(
    bitmap: a::Bitmap,
    name: *const u16,
    bg: i32,
    border: i32,
    shadow: i32,
    shadow_color: i32,
    rect: *mut a::Rect,
) -> a::Bool {
    let matches = call("GetThumbByName", false, |c| {
        Ok(c.filename == wide_input(name, 32768)?)
    });
    if matches {
        thumbnail(bitmap, bg, border, shadow, shadow_color, rect)
    } else {
        a::FALSE
    }
}
unsafe extern "system" fn prepare_dialog(
    window: a::Bitmap,
    mode: i32,
    center: a::Bool,
    wait: a::Bool,
) {
    let parent = call("PrepareDialog", 0isize, |c| Ok(c.parent));
    let mut pid = 0;
    GetWindowThreadProcessId(window, &mut pid);
    if window.is_null() || pid != GetCurrentProcessId() || !matches!(mode, 0..=2) {
        record_error("PrepareDialog: fenêtre étrangère ou mode invalide".into());
        return;
    }
    if mode != 0 {
        record_error("PrepareDialog: les modes aperçu et pipette reliés à la vue de l’éditeur ne sont pas encore pris en charge ; le dialogue modal Win32 reste disponible".into());
        return;
    }
    if center != 0 && !parent.eq(&0) {
        let mut main: RECT = std::mem::zeroed();
        let mut own: RECT = std::mem::zeroed();
        if GetWindowRect(parent as HWND, &mut main) != 0 && GetWindowRect(window, &mut own) != 0 {
            SetWindowPos(
                window,
                ptr::null_mut(),
                main.left + ((main.right - main.left) - (own.right - own.left)) / 2,
                main.top + ((main.bottom - main.top) - (own.bottom - own.top)) / 2,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
    if parent != 0 {
        EnableWindow(parent as HWND, 0);
    }
    if wait != 0 {
        ShowWindow(window, SW_SHOW);
        let mut msg: MSG = std::mem::zeroed();
        while IsWindow(window) != 0 && IsWindowVisible(window) != 0 {
            let result = GetMessageW(&mut msg, ptr::null_mut(), 0, 0);
            if result <= 0 {
                break;
            }
            if IsDialogMessageW(window, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if parent != 0 {
            EnableWindow(parent as HWND, 1);
        }
    }
}
pub fn host() -> a::Host {
    let mut host = call("Host", a::Host::new(), |c| {
        let mut h = a::Host::new();
        h.main_window = c.parent as a::Bitmap;
        h.image_exists = if c.editor.doc.is_some() { a::TRUE } else { 0 };
        h.selected = if c.editor.doc.as_ref().is_some_and(|d| d.selection.is_some()) {
            a::TRUE
        } else {
            0
        };
        h.selection_rect = c.selection_rect();
        h.image_rect = c.image_rect();
        Ok(h)
    });
    host.functions = [
        begin_filter as *const () as usize,
        end_filter as *const () as usize,
        get_bitmap as *const () as usize,
        set_bitmap as *const () as usize,
        get_selection as *const () as usize,
        set_selection as *const () as usize,
        get_transparent as *const () as usize,
        set_transparent as *const () as usize,
        get_color_count as *const () as usize,
        set_color_count as *const () as usize,
        new_image as *const () as usize,
        show_form as *const () as usize,
        alpha_blend as *const () as usize,
        draw_border as *const () as usize,
        get_mask as *const () as usize,
        reset_save as *const () as usize,
        image_filename as *const () as usize,
        exe_path as *const () as usize,
        open_path as *const () as usize,
        save_path as *const () as usize,
        plugin_path as *const () as usize,
        ini_file as *const () as usize,
        set_filename as *const () as usize,
        bitmap_by_name as *const () as usize,
        thumbnail as *const () as usize,
        thumbnail_by_name as *const () as usize,
        enumerate_images as *const () as usize,
        prepare_dialog as *const () as usize,
        create_bitmap as *const () as usize,
        free_bitmap as *const () as usize,
        release_handle as *const () as usize,
        get_scanline as *const () as usize,
        get_cropped_mask as *const () as usize,
        get_dpi as *const () as usize,
        set_dpi as *const () as usize,
        set_mask as *const () as usize,
        reset_undo as *const () as usize,
        application_state as *const () as usize,
        selection_shape as *const () as usize,
        0,
        get_layer_rect as *const () as usize,
        get_layer as *const () as usize,
        set_layer as *const () as usize,
        get_alpha as *const () as usize,
        set_alpha as *const () as usize,
        get_selection_alpha as *const () as usize,
        set_selection_alpha as *const () as usize,
        disable_layers as *const () as usize,
        color_mode as *const () as usize,
        get_layer_position as *const () as usize,
    ];
    host
}
