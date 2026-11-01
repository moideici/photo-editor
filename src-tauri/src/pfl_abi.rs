//! Layouts PFL Unicode x64. Compare plugin-sdk/pfl/prisme_pfl.h and PFL_ABI.md.
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
pub type Bool = i32;
pub type Bitmap = *mut c_void;
pub const TRUE: Bool = -1;
pub const FALSE: Bool = 0;
pub const CL_NONE: i32 = 0x1fff_ffff;
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl Rect {
    pub fn image(w: u32, h: u32) -> Self {
        Self {
            left: 0,
            top: 0,
            right: w as i32,
            bottom: h as i32,
        }
    }
    pub fn width(self) -> u32 {
        self.right.saturating_sub(self.left).max(0) as u32
    }
    pub fn height(self) -> u32 {
        self.bottom.saturating_sub(self.top).max(0) as u32
    }
    pub fn intersect(self, other: Self) -> Self {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        Self {
            left,
            top,
            right: self.right.min(other.right).max(left),
            bottom: self.bottom.min(other.bottom).max(top),
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Header {
    pub caption: [u16; 64],
    pub version: [u16; 64],
    pub plugin_type: i32,
    pub author: [u16; 64],
    pub comment: [u16; 256],
    pub reserved: [u16; 256],
    pub flags: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Language {
    pub code: [u16; 4],
    pub name: [u16; 64],
}
#[repr(C)]
pub struct Image {
    pub size: i32,
    pub width: i32,
    pub height: i32,
    pub width_bytes: i32,
    pub scanline_bytes: i32,
    pub bytes_per_pixel: i32,
    pub data_bytes: i32,
    pub buffer: *mut u8,
    pub bitmap: Bitmap,
    pub handle: Bitmap,
}
#[repr(C)]
pub struct Host {
    pub main_window: Bitmap,
    pub image_exists: Bool,
    pub selected: Bool,
    pub selection_rect: Rect,
    pub image_rect: Rect,
    pub themes_enabled: Bool,
    // 49 callbacks and the unmodified reserved slot at 0x170 (index 39).
    pub functions: [usize; 50],
}
impl Host {
    pub fn new() -> Self {
        Self {
            main_window: std::ptr::null_mut(),
            image_exists: 0,
            selected: 0,
            selection_rect: Rect::default(),
            image_rect: Rect::default(),
            themes_enabled: 0,
            functions: [0; 50],
        }
    }
    pub fn prefix_bytes(level: i32) -> usize {
        match level {
            2 => 0x138,
            3 => 0x170,
            _ => 0x1c8,
        }
    }
}
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn text(data: &[u16]) -> Result<String, String> {
    String::from_utf16(&data[..data.iter().position(|&c| c == 0).unwrap_or(data.len())])
        .map_err(|_| "Chaîne UTF-16 PFL invalide".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};
    #[test]
    fn observed_x64_layouts_match_the_independent_c_probe() {
        assert_eq!(size_of::<usize>(), 8);
        assert_eq!(size_of::<Header>(), 0x588);
        assert_eq!(offset_of!(Header, plugin_type), 0x100);
        assert_eq!(offset_of!(Header, flags), 0x584);
        assert_eq!(size_of::<Language>(), 0x88);
        assert_eq!(size_of::<Image>(), 0x38);
        assert_eq!(offset_of!(Image, buffer), 0x20);
        assert_eq!(offset_of!(Image, bitmap), 0x28);
        assert_eq!(offset_of!(Image, handle), 0x30);
        assert_eq!(offset_of!(Host, selection_rect), 0x10);
        assert_eq!(offset_of!(Host, image_rect), 0x20);
        assert_eq!(offset_of!(Host, functions), 0x38);
        assert_eq!(size_of::<Host>(), 0x1c8);
        assert_eq!(Host::prefix_bytes(2), 0x138);
        assert_eq!(Host::prefix_bytes(3), 0x170);
    }
}
