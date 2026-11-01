//! Lecture des GIF coalescés (offsets et méthodes de disposition compris).
use image::{AnimationDecoder, ImageDecoder, RgbaImage};
use serde::Serialize;
use std::{io::Cursor, sync::Arc};
#[derive(Clone)]
pub struct AnimationFrame {
    pub image: Arc<RgbaImage>,
    pub delay_ms: u32,
}
#[derive(Clone)]
pub struct Animation {
    pub id: u64,
    pub frames: Arc<Vec<AnimationFrame>>,
    pub current: usize,
    pub loops: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationInfo {
    pub id: u64,
    pub frame_count: usize,
    pub delays: Vec<u32>,
    pub current: usize,
    pub loops: u32,
}
impl Animation {
    pub fn info(&self) -> AnimationInfo {
        AnimationInfo {
            id: self.id,
            frame_count: self.frames.len(),
            delays: self.frames.iter().map(|f| f.delay_ms).collect(),
            current: self.current,
            loops: self.loops,
        }
    }
    pub fn validate(&self, w: u32, h: u32) -> Result<(), String> {
        if !(2..=1000).contains(&self.frames.len()) || self.current >= self.frames.len() {
            return Err("Animation incohérente".into());
        }
        let mut bytes = 0u64;
        for frame in self.frames.iter() {
            if frame.image.dimensions() != (w, h) || frame.delay_ms == 0 {
                return Err("Image ou durée d'animation incohérente".into());
            }
            bytes += frame.image.as_raw().len() as u64;
        }
        if bytes > 256 * 1024 * 1024 {
            return Err("Les images de l'animation dépassent 256 Mio".into());
        }
        Ok(())
    }
}
pub fn decode_gif(bytes: &[u8]) -> Result<(Arc<RgbaImage>, Option<Animation>), String> {
    let mut decoder =
        image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    crate::io::dimensions(w, h)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(512 * 1024 * 1024);
    decoder.set_limits(limits).map_err(|e| e.to_string())?;
    // gif::Repeat compte les répétitions APRÈS le premier passage. Son défaut
    // Finite(0) signifie une seule lecture sans extension Netscape.
    let header = gif::DecodeOptions::new()
        .read_info(Cursor::new(bytes))
        .map_err(|e| e.to_string())?;
    let loops = match header.repeat() {
        gif::Repeat::Infinite => 0,
        gif::Repeat::Finite(n) => u32::from(n) + 1,
    };
    let mut frames = Vec::new();
    let mut total = 0u64;
    for frame in decoder.into_frames() {
        let frame = frame.map_err(|e| e.to_string())?;
        let (n, d) = frame.delay().numer_denom_ms();
        let delay = if n == 0 {
            100
        } else {
            ((n as u64 + d.max(1) as u64 / 2) / d.max(1) as u64).clamp(10, u32::MAX as u64) as u32
        };
        let image = Arc::new(frame.into_buffer());
        total += image.as_raw().len() as u64;
        if total > 256 * 1024 * 1024 || frames.len() >= 1000 {
            return Err("Animation trop grande (256 Mio ou 1 000 images maximum)".into());
        }
        frames.push(AnimationFrame {
            image,
            delay_ms: delay,
        });
    }
    let first = frames.first().ok_or("GIF sans image")?.image.clone();
    let animation = if frames.len() > 1 {
        let a = Animation {
            id: 0,
            frames: Arc::new(frames),
            current: 0,
            loops,
        };
        a.validate(w, h)?;
        Some(a)
    } else {
        None
    };
    Ok((first, animation))
}

/// GIF : palette locale par image, alpha binaire et durées en centièmes.
pub fn encode_gif(doc: &crate::document::Document) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut out, doc.width as u16, doc.height as u16, &[])
            .map_err(|e| e.to_string())?;
        let frames = if let Some(a) = &doc.animation {
            a.validate(doc.width, doc.height)?;
            if a.loops == 0 {
                encoder
                    .set_repeat(gif::Repeat::Infinite)
                    .map_err(|e| e.to_string())?;
            } else if a.loops > 1 {
                if a.loops > 65536 {
                    return Err("GIF limité à 65 536 passages ou une boucle infinie".into());
                }
                encoder
                    .set_repeat(gif::Repeat::Finite((a.loops - 1) as u16))
                    .map_err(|e| e.to_string())?;
            }
            a.frames.as_ref().clone()
        } else {
            vec![AnimationFrame {
                image: Arc::new(crate::vector::flatten(doc)?),
                delay_ms: 100,
            }]
        };
        for frame in frames {
            if frame.delay_ms > 655_350 {
                return Err("Une image GIF ne peut durer plus de 655,35 secondes".into());
            }
            let mut rgba = frame.image.as_raw().clone();
            for pixel in rgba.chunks_exact_mut(4) {
                pixel[3] = if pixel[3] < 128 { 0 } else { 255 };
            }
            let mut encoded =
                gif::Frame::from_rgba_speed(doc.width as u16, doc.height as u16, &mut rgba, 10);
            encoded.delay = ((frame.delay_ms + 5) / 10).clamp(1, 65535) as u16;
            encoded.dispose = gif::DisposalMethod::Background;
            encoder.write_frame(&encoded).map_err(|e| e.to_string())?;
        }
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::borrow::Cow;
    pub fn fixture() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder =
                gif::Encoder::new(&mut out, 4, 3, &[255, 0, 0, 0, 255, 0, 0, 0, 255]).unwrap();
            encoder.set_repeat(gif::Repeat::Finite(2)).unwrap();
            let first = gif::Frame {
                width: 4,
                height: 3,
                buffer: Cow::Owned(vec![0; 12]),
                delay: 5,
                dispose: gif::DisposalMethod::Keep,
                ..gif::Frame::default()
            };
            encoder.write_frame(&first).unwrap();
            let second = gif::Frame {
                left: 1,
                top: 1,
                width: 1,
                height: 1,
                buffer: Cow::Owned(vec![1]),
                delay: 8,
                dispose: gif::DisposalMethod::Background,
                ..gif::Frame::default()
            };
            encoder.write_frame(&second).unwrap();
            let third = gif::Frame {
                left: 2,
                top: 1,
                width: 1,
                height: 1,
                buffer: Cow::Owned(vec![2]),
                delay: 12,
                dispose: gif::DisposalMethod::Keep,
                ..gif::Frame::default()
            };
            encoder.write_frame(&third).unwrap();
        }
        out
    }
    #[test]
    fn absent_loop_extension_plays_once_and_infinite_is_distinct() {
        let bytes = fixture();
        let start = bytes
            .windows(3)
            .position(|w| w == [0x21, 0xff, 11])
            .unwrap();
        let mut once = bytes.clone();
        once.drain(start..start + 19);
        assert_eq!(decode_gif(&once).unwrap().1.unwrap().loops, 1);
        let mut infinite = bytes;
        infinite[start + 16] = 0;
        infinite[start + 17] = 0;
        assert_eq!(decode_gif(&infinite).unwrap().1.unwrap().loops, 0);
    }
    #[test]
    fn decoding_coalesces_offsets_disposal_durations_and_loop_count() {
        let work = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("fixture-animation.gif"), fixture()).unwrap();
        let (first, animation) = decode_gif(&fixture()).unwrap();
        let animation = animation.unwrap();
        assert_eq!(first.dimensions(), (4, 3));
        assert_eq!(animation.loops, 3);
        assert_eq!(animation.info().delays, vec![50, 80, 120]);
        assert_eq!(
            animation.frames[1].image.get_pixel(1, 1).0,
            [0, 255, 0, 255]
        );
        assert_eq!(
            animation.frames[1].image.get_pixel(0, 0).0,
            [255, 0, 0, 255]
        );
        assert_ne!(
            animation.frames[2].image.get_pixel(1, 1).0,
            [0, 255, 0, 255]
        );
        assert_eq!(
            animation.frames[2].image.get_pixel(2, 1).0,
            [0, 0, 255, 255]
        );
    }
}
