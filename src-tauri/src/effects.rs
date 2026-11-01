//! Styles non destructifs, exprimés en primitives SVG pour partager le même
//! rendu entre l'atelier et l'export vectoriel.
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LayerEffects {
    pub shadow: Option<Shadow>,
    pub outline: Option<Outline>,
    pub bevel: Option<Bevel>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Shadow {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: String,
    pub opacity: f32,
}
impl Default for Shadow {
    fn default() -> Self {
        Self {
            x: 8.0,
            y: 8.0,
            blur: 4.0,
            spread: 0.0,
            color: "#000000".into(),
            opacity: 0.6,
        }
    }
}
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Position {
    #[default]
    Outside,
    Inside,
    Center,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Outline {
    pub width: f32,
    pub color: String,
    pub opacity: f32,
    pub position: Position,
}
impl Default for Outline {
    fn default() -> Self {
        Self {
            width: 3.0,
            color: "#000000".into(),
            opacity: 1.0,
            position: Position::Outside,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Bevel {
    pub size: f32,
    pub depth: f32,
    pub angle: f32,
    pub opacity: f32,
    pub highlight: String,
    pub shade: String,
}
impl Default for Bevel {
    fn default() -> Self {
        Self {
            size: 4.0,
            depth: 4.0,
            angle: 225.0,
            opacity: 0.75,
            highlight: "#ffffff".into(),
            shade: "#000000".into(),
        }
    }
}
fn valid(value: f32, min: f32, max: f32) -> bool {
    value.is_finite() && (min..=max).contains(&value)
}
fn color_valid(color: &str) -> bool {
    crate::raster::parse_color(color).is_some()
}

impl LayerEffects {
    pub fn is_empty(&self) -> bool {
        self.shadow.is_none() && self.outline.is_none() && self.bevel.is_none()
    }
    pub fn validate(&self) -> Result<(), String> {
        if let Some(s) = &self.shadow {
            if !s.x.is_finite()
                || !s.y.is_finite()
                || s.x.hypot(s.y) > 1024.001
                || !valid(s.blur, 0.0, 128.0)
                || !valid(s.spread, 0.0, 128.0)
                || !valid(s.opacity, 0.0, 1.0)
                || !color_valid(&s.color)
            {
                return Err("Paramètres d'ombre invalides".into());
            }
        }
        if let Some(s) = &self.outline {
            if !valid(s.width, 0.1, 128.0) || !valid(s.opacity, 0.0, 1.0) || !color_valid(&s.color)
            {
                return Err("Paramètres de contour invalides".into());
            }
        }
        if let Some(s) = &self.bevel {
            if !valid(s.size, 0.5, 128.0)
                || !valid(s.depth, 0.0, 32.0)
                || !valid(s.angle, -360.0, 360.0)
                || !valid(s.opacity, 0.0, 1.0)
                || !color_valid(&s.highlight)
                || !color_valid(&s.shade)
            {
                return Err("Paramètres de biseau invalides".into());
            }
        }
        Ok(())
    }
    pub fn padding(&self) -> u32 {
        let shadow = self.shadow.as_ref().map_or(0.0, |s| {
            s.x.abs().max(s.y.abs()) + s.spread + s.blur * 3.0 + 2.0
        });
        let outline = self.outline.as_ref().map_or(0.0, |s| s.width + 2.0);
        shadow.max(outline).ceil() as u32
    }
    pub fn rotate(&mut self, degrees: f32) {
        let (s, c) = degrees.to_radians().sin_cos();
        if let Some(shadow) = &mut self.shadow {
            let (x, y) = (shadow.x, shadow.y);
            shadow.x = c * x - s * y;
            shadow.y = s * x + c * y;
        }
        if let Some(bevel) = &mut self.bevel {
            bevel.angle = (bevel.angle + degrees).rem_euclid(360.0);
        }
    }
    pub fn mirror(&mut self, horizontal: bool) {
        if let Some(s) = &mut self.shadow {
            if horizontal {
                s.x = -s.x;
            } else {
                s.y = -s.y;
            }
        }
        if let Some(b) = &mut self.bevel {
            b.angle = (if horizontal {
                180.0 - b.angle
            } else {
                -b.angle
            })
            .rem_euclid(360.0);
        }
    }
    /// Région en coordonnées du contenu du calque, avant son déplacement.
    pub fn svg(&self, id: &str, region: [f32; 4]) -> String {
        let mut parts = String::new();
        let mut below = Vec::new();
        let mut above = Vec::new();
        if let Some(shadow) = &self.shadow {
            parts.push_str(&format!("<feMorphology in=\"SourceAlpha\" operator=\"dilate\" radius=\"{}\" result=\"shadowSpread\"/><feGaussianBlur in=\"shadowSpread\" stdDeviation=\"{}\" result=\"shadowBlur\"/><feOffset in=\"shadowBlur\" dx=\"{}\" dy=\"{}\" result=\"shadowAlpha\"/>",shadow.spread,shadow.blur,shadow.x,shadow.y));
            parts.push_str(&paint(
                "shadowPaint",
                "shadowAlpha",
                &shadow.color,
                shadow.opacity,
            ));
            below.push("shadowPaint");
        }
        if let Some(outline) = &self.outline {
            match outline.position {
                Position::Outside=>parts.push_str(&format!("<feMorphology in=\"SourceAlpha\" operator=\"dilate\" radius=\"{}\" result=\"outlineAlpha\"/>",outline.width)),
                Position::Inside=>parts.push_str(&format!("<feMorphology in=\"SourceAlpha\" operator=\"erode\" radius=\"{}\" result=\"outlineInner\"/><feComposite in=\"SourceAlpha\" in2=\"outlineInner\" operator=\"out\" result=\"outlineAlpha\"/>",outline.width)),
                Position::Center=>parts.push_str(&format!("<feMorphology in=\"SourceAlpha\" operator=\"dilate\" radius=\"{}\" result=\"outlineOuter\"/><feMorphology in=\"SourceAlpha\" operator=\"erode\" radius=\"{}\" result=\"outlineInner\"/><feComposite in=\"outlineOuter\" in2=\"outlineInner\" operator=\"out\" result=\"outlineAlpha\"/>",outline.width/2.0,outline.width/2.0)),
            }
            parts.push_str(&paint(
                "outlinePaint",
                "outlineAlpha",
                &outline.color,
                outline.opacity,
            ));
            if matches!(outline.position, Position::Outside) {
                below.push("outlinePaint");
            } else {
                above.push("outlinePaint");
            }
        }
        if let Some(bevel) = &self.bevel {
            parts.push_str(&format!("<feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{}\" result=\"bevelHeight\"/><feMorphology in=\"SourceAlpha\" operator=\"erode\" radius=\"{}\" result=\"bevelInner\"/><feComposite in=\"SourceAlpha\" in2=\"bevelInner\" operator=\"out\" result=\"bevelEdge\"/><feDiffuseLighting in=\"bevelHeight\" surfaceScale=\"{}\" diffuseConstant=\"1\" lighting-color=\"#ffffff\" result=\"bevelLight\"><feDistantLight azimuth=\"{}\" elevation=\"45\"/></feDiffuseLighting>",bevel.size/2.0,bevel.size,bevel.depth,bevel.angle));
            for (name, slope, intercept, color) in [
                ("bevelHighlight", 4.0, -2.828427, &bevel.highlight),
                ("bevelShade", -4.0, 2.828427, &bevel.shade),
            ] {
                parts.push_str(&format!("<feComponentTransfer in=\"bevelLight\" result=\"{name}Luma\"><feFuncR type=\"linear\" slope=\"{slope}\" intercept=\"{intercept}\"/><feFuncG type=\"linear\" slope=\"{slope}\" intercept=\"{intercept}\"/><feFuncB type=\"linear\" slope=\"{slope}\" intercept=\"{intercept}\"/></feComponentTransfer><feColorMatrix in=\"{name}Luma\" type=\"matrix\" values=\"0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  1 0 0 0 0\" result=\"{name}Alpha\"/><feComposite in=\"{name}Alpha\" in2=\"bevelEdge\" operator=\"in\" result=\"{name}Edge\"/>"));
                parts.push_str(&paint(name, &format!("{name}Edge"), color, bevel.opacity));
            }
            above.extend(["bevelShade", "bevelHighlight"]);
        }
        let nodes = below
            .into_iter()
            .chain(std::iter::once("SourceGraphic"))
            .chain(above)
            .map(|name| format!("<feMergeNode in=\"{name}\"/>"))
            .collect::<String>();
        format!("<filter id=\"{id}\" filterUnits=\"userSpaceOnUse\" primitiveUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" color-interpolation-filters=\"sRGB\">{parts}<feMerge>{nodes}</feMerge></filter>",region[0],region[1],region[2],region[3])
    }
}
fn paint(name: &str, mask: &str, color: &str, opacity: f32) -> String {
    let p = crate::raster::parse_color(color).unwrap_or([0, 0, 0, 255]);
    format!("<feFlood flood-color=\"#{:02x}{:02x}{:02x}\" flood-opacity=\"{}\" result=\"{name}Color\"/><feComposite in=\"{name}Color\" in2=\"{mask}\" operator=\"in\" result=\"{name}\"/>",p[0],p[1],p[2],opacity*p[3] as f32/255.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        document::{Content, Editor},
        operations::{self, Operation},
        vector,
    };
    fn command(e: &mut Editor, v: serde_json::Value) {
        operations::execute(e, serde_json::from_value(v).unwrap()).unwrap();
    }
    fn scene() -> (Editor, u64) {
        let mut e = Editor::default();
        command(
            &mut e,
            serde_json::json!({"type":"new","width":64,"height":48,"background":null}),
        );
        command(
            &mut e,
            serde_json::json!({"type":"addLayer","kind":"vector"}),
        );
        let id = e.active_hint.unwrap();
        command(
            &mut e,
            serde_json::json!({"type":"shape","id":id,"shape":{"id":0,"type":"rect","radius":0,"x":20,"y":16,"w":16,"h":16,"fill":"#808080","stroke":null,"strokeWidth":0,"opacity":1,"rotation":0}}),
        );
        (e, id)
    }
    fn style(e: &mut Editor, id: u64, effects: LayerEffects) {
        operations::execute(e, Operation::Effects { id, effects }).unwrap();
    }
    #[test]
    fn shadow_is_rendered_outside_content_and_is_undoable_without_rasterization() {
        let (mut e, id) = scene();
        let before = vector::flatten(e.doc().unwrap()).unwrap();
        style(
            &mut e,
            id,
            LayerEffects {
                shadow: Some(Shadow {
                    x: 12.0,
                    y: 0.0,
                    blur: 0.0,
                    color: "#0000ff".into(),
                    opacity: 1.0,
                    ..Shadow::default()
                }),
                ..Default::default()
            },
        );
        let image = vector::flatten(e.doc().unwrap()).unwrap();
        assert_eq!(image.get_pixel(22, 20).0, [128, 128, 128, 255]);
        assert_eq!(image.get_pixel(40, 20).0, [0, 0, 255, 255]);
        assert!(matches!(
            e.doc().unwrap().layers[1].content,
            Content::Vector(_)
        ));
        e.undo().unwrap();
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before);
        e.redo().unwrap();
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), image);
    }
    #[test]
    fn outline_positions_and_bevel_produce_real_pixels() {
        let (mut e, id) = scene();
        style(
            &mut e,
            id,
            LayerEffects {
                outline: Some(Outline {
                    width: 3.0,
                    color: "#ff0000".into(),
                    ..Outline::default()
                }),
                ..Default::default()
            },
        );
        let image = vector::flatten(e.doc().unwrap()).unwrap();
        assert_eq!(image.get_pixel(18, 20).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(22, 20).0, [128, 128, 128, 255]);
        style(
            &mut e,
            id,
            LayerEffects {
                outline: Some(Outline {
                    width: 3.0,
                    color: "#ff0000".into(),
                    position: Position::Inside,
                    ..Outline::default()
                }),
                ..Default::default()
            },
        );
        let image = vector::flatten(e.doc().unwrap()).unwrap();
        assert_eq!(image.get_pixel(18, 20)[3], 0);
        assert_eq!(image.get_pixel(20, 20).0, [255, 0, 0, 255]);
        let plain = vector::render_svg(
            &vector::svg_document(
                64,
                48,
                "<rect x=\"20\" y=\"16\" width=\"16\" height=\"16\" fill=\"#808080\"/>",
            ),
            64,
            48,
        )
        .unwrap();
        style(
            &mut e,
            id,
            LayerEffects {
                bevel: Some(Bevel::default()),
                ..Default::default()
            },
        );
        let bevel = vector::flatten(e.doc().unwrap()).unwrap();
        assert_ne!(bevel, plain);
        assert_eq!(bevel.get_pixel(28, 24).0, [128, 128, 128, 255]);
    }
    #[test]
    fn styled_text_and_svg_export_share_rendering_and_project_round_trip() {
        let (mut e, id) = scene();
        command(
            &mut e,
            serde_json::json!({"type":"shape","id":id,"shape":{"id":0,"type":"text","text":"P","fontFamily":"Segoe UI","fontSize":24,"bold":true,"italic":false,"x":4,"y":4,"w":16,"h":32,"fill":"#e08020","stroke":null,"strokeWidth":0,"opacity":1,"rotation":0}}),
        );
        style(
            &mut e,
            id,
            LayerEffects {
                shadow: Some(Shadow::default()),
                outline: Some(Outline::default()),
                bevel: Some(Bevel::default()),
            },
        );
        let doc = e.doc().unwrap();
        let before = vector::flatten(doc).unwrap();
        let svg = vector::export_svg(doc).unwrap();
        assert!(svg.contains("<text") && svg.contains("<feDiffuseLighting"));
        let exported = vector::render_svg(&svg, 64, 48).unwrap();
        assert_eq!(before, exported);
        let restored = crate::io::read_project(&crate::io::project_bytes(doc).unwrap()).unwrap();
        assert_eq!(vector::flatten(&restored).unwrap(), before);
        assert!(matches!(restored.layers[1].content, Content::Vector(_)));
        operations::execute(&mut e, Operation::Rasterize { id }).unwrap();
        assert!(!e.doc().unwrap().layers[1].effects.is_empty());
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before);
    }
    #[test]
    fn off_canvas_content_can_cast_a_shadow_back_onto_canvas_and_export() {
        let (mut e, id) = scene();
        let Content::Vector(shapes) = &mut e.doc.as_mut().unwrap().layers[1].content else {
            panic!()
        };
        shapes[0].x = 66.0;
        style(
            &mut e,
            id,
            LayerEffects {
                shadow: Some(Shadow {
                    x: -12.0,
                    y: 0.0,
                    blur: 0.0,
                    color: "#0000ff".into(),
                    opacity: 1.0,
                    ..Shadow::default()
                }),
                ..Default::default()
            },
        );
        let doc = e.doc().unwrap();
        let rendered = vector::flatten(doc).unwrap();
        assert_eq!(rendered.get_pixel(55, 20).0, [0, 0, 255, 255]);
        assert_eq!(
            rendered,
            vector::render_svg(&vector::export_svg(doc).unwrap(), 64, 48).unwrap()
        );
    }
    #[test]
    fn bounds_and_transformations_follow_shadow_geometry() {
        let mut fx = LayerEffects {
            shadow: Some(Shadow {
                x: 12.0,
                y: 5.0,
                blur: 2.0,
                ..Shadow::default()
            }),
            ..Default::default()
        };
        assert_eq!(fx.padding(), 20);
        fx.rotate(90.0);
        let s = fx.shadow.as_ref().unwrap();
        assert!((s.x + 5.0).abs() < 0.001 && (s.y - 12.0).abs() < 0.001);
        fx.mirror(true);
        assert!((fx.shadow.unwrap().x - 5.0).abs() < 0.001);
    }
    #[test]
    fn invalid_effects_are_rejected_before_rendering() {
        let mut fx = LayerEffects::default();
        assert!(fx.validate().is_ok());
        fx.shadow = Some(Shadow {
            blur: f32::NAN,
            ..Shadow::default()
        });
        assert!(fx.validate().is_err());
        fx.shadow = Some(Shadow {
            color: "red\" onload=\"x".into(),
            ..Shadow::default()
        });
        assert!(fx.validate().is_err());
    }
}
