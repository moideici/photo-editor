//! Images intégrées : données autonomes, validation bornée et diagnostics SVG.
use crate::{
    document::{Shape, ShapeKind},
    groups::{self, Matrix},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
pub const MAX_BYTES: usize = 12 * 1024 * 1024;
pub const RENDERING: [&str; 6] = [
    "optimizeQuality",
    "optimizeSpeed",
    "smooth",
    "high-quality",
    "crisp-edges",
    "pixelated",
];
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAsset {
    pub mime: String,
    pub data: String,
    pub width: u32,
    pub height: u32,
    pub rendering: String,
}
impl ImageAsset {
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        if self.data.len() > MAX_BYTES * 4 / 3 + 8 {
            return Err("Image intégrée supérieure à 12 Mio".into());
        }
        let bytes = STANDARD
            .decode(&self.data)
            .map_err(|_| "Données de l’image intégrée invalides")?;
        if bytes.len() > MAX_BYTES {
            return Err("Image intégrée supérieure à 12 Mio".into());
        }
        Ok(bytes)
    }
    pub fn url(&self) -> String {
        format!("data:{};base64,{}", self.mime, self.data)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !RENDERING.contains(&self.rendering.as_str()) {
            return Err("Interpolation de l’image intégrée invalide".into());
        }
        let size = payload_size(&self.mime, &self.bytes()?, 0)?;
        if size != [self.width, self.height] {
            return Err("Dimensions de l’image intégrée incohérentes avec ses données".into());
        }
        Ok(())
    }
    pub fn from_bytes(mime: &str, bytes: &[u8], rendering: &str) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("Image intégrée supérieure à 12 Mio".into());
        }
        let [width, height] = payload_size(mime, bytes, 0)?;
        let asset = Self {
            mime: mime.into(),
            data: STANDARD.encode(bytes),
            width,
            height,
            rendering: rendering.into(),
        };
        asset.validate()?;
        Ok(asset)
    }
}
fn payload_size(mime: &str, bytes: &[u8], depth: usize) -> Result<[u32; 2], String> {
    if depth > 8 {
        return Err("Plus de huit niveaux d’images SVG intégrées".into());
    }
    if bytes.len() > MAX_BYTES {
        return Err("Image intégrée supérieure à 12 Mio".into());
    }
    let (w, h) = if mime == "image/svg+xml" {
        let source = std::str::from_utf8(bytes).map_err(|_| "Image SVG non UTF-8")?;
        let notices = diagnostics_inner(source, depth + 1)?;
        if !notices.is_empty() {
            return Err(notices.join(" ; "));
        }
        let tree = crate::vector::parse(source)?;
        (
            tree.size().width().ceil() as u32,
            tree.size().height().ceil() as u32,
        )
    } else {
        let expected = match mime {
            "image/png" => image::ImageFormat::Png,
            "image/jpeg" => image::ImageFormat::Jpeg,
            "image/gif" => image::ImageFormat::Gif,
            "image/webp" => image::ImageFormat::WebP,
            _ => {
                return Err(format!(
                    "Format d’image intégrée non pris en charge : {mime}"
                ))
            }
        };
        if image::guess_format(bytes).map_err(|e| e.to_string())? != expected {
            return Err("Format déclaré de l’image intégrée incohérent".into());
        }
        let image = crate::io::decode(bytes)?;
        image.dimensions()
    };
    crate::io::dimensions(w, h)?;
    Ok([w, h])
}
pub fn file(path: &str) -> Result<ImageAsset, String> {
    let p = std::path::Path::new(path);
    if std::fs::metadata(p).map_err(|e| e.to_string())?.len() > MAX_BYTES as u64 {
        return Err("Image intégrée supérieure à 12 Mio".into());
    }
    let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
    let mime = if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
        "image/svg+xml"
    } else {
        match image::guess_format(&bytes).map_err(|e| e.to_string())? {
            image::ImageFormat::Png => "image/png",
            image::ImageFormat::Jpeg => "image/jpeg",
            image::ImageFormat::Gif => "image/gif",
            image::ImageFormat::WebP => "image/webp",
            _ => return Err("Choisissez une image PNG, JPEG, GIF, WebP ou SVG".into()),
        }
    };
    ImageAsset::from_bytes(mime, &bytes, "optimizeQuality")
}
pub fn imported(image: &resvg::usvg::Image, matrix: Matrix) -> Result<Shape, String> {
    use resvg::usvg::{ImageKind, ImageRendering};
    let (mime, bytes) = match image.kind() {
        ImageKind::PNG(b) => ("image/png", (**b).clone()),
        ImageKind::JPEG(b) => ("image/jpeg", (**b).clone()),
        ImageKind::GIF(b) => ("image/gif", (**b).clone()),
        ImageKind::WEBP(b) => ("image/webp", (**b).clone()),
        ImageKind::SVG(tree) => (
            "image/svg+xml",
            tree.to_string(&resvg::usvg::WriteOptions {
                preserve_text: false,
                ..Default::default()
            })
            .into_bytes(),
        ),
    };
    let rendering = match image.rendering_mode() {
        ImageRendering::OptimizeQuality => "optimizeQuality",
        ImageRendering::OptimizeSpeed => "optimizeSpeed",
        ImageRendering::Smooth => "smooth",
        ImageRendering::HighQuality => "high-quality",
        ImageRendering::CrispEdges => "crisp-edges",
        ImageRendering::Pixelated => "pixelated",
    };
    let asset = ImageAsset::from_bytes(mime, &bytes, rendering)?;
    let sx = matrix[0].hypot(matrix[1]);
    let sy = matrix[2].hypot(matrix[3]);
    if sx < 1e-6 || sy < 1e-6 {
        return Err("Transformation de l’image intégrée singulière".into());
    }
    let [a, b, c, d] = [
        matrix[0] / sx,
        matrix[1] / sx,
        matrix[2] / sy,
        matrix[3] / sy,
    ];
    let (x, y) = (matrix[4], matrix[5]);
    let m = [a, b, c, d, x - a * x - c * y, y - b * x - d * y];
    let transform = if m == groups::IDENTITY { None } else { Some(m) };
    Ok(Shape {
        font_options: None,
        text_path: None,
        svg_unclipped: false,
        svg_viewport: false,
        filters: None,
        mask: None,
        clip: None,
        group_style: None,
        fill_pattern: None,
        stroke_pattern: None,
        fill_gradient: None,
        stroke_gradient: None,
        transform,
        id: 0,
        x,
        y,
        w: image.size().width() * sx,
        h: image.size().height() * sy,
        rotation: 0.,
        fill: None,
        stroke: None,
        stroke_width: 0.,
        opacity: 1.,
        kind: ShapeKind::Image { asset },
    })
}
pub fn diagnostics(source: &str) -> Result<Vec<String>, String> {
    diagnostics_inner(source, 0)
}
fn diagnostics_inner(source: &str, depth: usize) -> Result<Vec<String>, String> {
    if source.len() > 16 * 1024 * 1024 {
        return Err("SVG supérieur à 16 Mio".into());
    }
    if depth > 8 {
        return Err("Plus de huit niveaux d’images SVG intégrées".into());
    }
    let doc = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|e| format!("SVG invalide : {e}"))?;
    let mut notices = Vec::new();
    let filter_ids = crate::svg_filter::referenced_filter_templates(&doc);
    for node in doc.descendants().filter(|n| {
        n.has_tag_name("image")
            || n.has_tag_name("feImage")
                && n.ancestors().any(|a| {
                    a.has_tag_name("filter")
                        && a.attribute("id").is_some_and(|id| filter_ids.contains(id))
                })
    }) {
        let href = node
            .attribute("href")
            .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")))
            .unwrap_or("");
        let label = node.attribute("id").unwrap_or("sans identifiant");
        if node.has_tag_name("feImage") && href.starts_with('#') {
            if href.len() > 1
                && doc
                    .descendants()
                    .any(|n| n.attribute("id") == Some(&href[1..]))
            {
                continue;
            }
            notices.push(format!(
                "Source feImage « {label} » : référence interne absente : {href}"
            ));
            continue;
        }
        let reason = if !href.starts_with("data:") {
            Some(if href.is_empty() {
                "référence absente".into()
            } else {
                format!(
                    "référence externe non chargée : {}",
                    href.chars().take(180).collect::<String>()
                )
            })
        } else {
            let result = (|| -> Result<(), String> {
                if href.len() > 16 * 1024 * 1024 {
                    return Err("données supérieures à 16 Mio".into());
                }
                let data = data_url::DataUrl::process(href).map_err(|e| e.to_string())?;
                let (body, _) = data.decode_to_vec().map_err(|e| e.to_string())?;
                let mime = data.mime_type().to_string();
                payload_size(mime.split(';').next().unwrap_or(&mime), &body, depth)?;
                Ok(())
            })();
            result.err()
        };
        if let Some(reason) = reason {
            notices.push(format!("Image SVG « {label} » : {reason}"));
        }
        if notices.len() >= 32 {
            notices.push("Autres images non chargées : diagnostics limités à 32 entrées".into());
            break;
        }
    }
    Ok(notices)
}
