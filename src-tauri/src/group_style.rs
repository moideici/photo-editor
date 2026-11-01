use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupStyle {
    pub blend: String,
    pub isolate: bool,
}
pub const BLENDS: [&str; 17] = [
    "normal",
    "multiply",
    "screen",
    "overlay",
    "darken",
    "lighten",
    "color-dodge",
    "color-burn",
    "hard-light",
    "soft-light",
    "difference",
    "exclusion",
    "hue",
    "saturation",
    "color",
    "luminosity",
    "plus-lighter",
];
impl GroupStyle {
    pub fn validate(&self) -> Result<(), String> {
        if !BLENDS.contains(&self.blend.as_str()) {
            return Err("Mode de fusion du groupe SVG invalide".into());
        }
        Ok(())
    }
    pub fn neutral(&self) -> bool {
        self.blend == "normal" && !self.isolate
    }
    pub fn svg(&self) -> String {
        format!(
            "mix-blend-mode:{};isolation:{}",
            self.blend,
            if self.isolate { "isolate" } else { "auto" }
        )
    }
    pub fn imported(group: &resvg::usvg::Group) -> Self {
        use resvg::usvg::BlendMode::*;
        let blend = match group.blend_mode() {
            Normal => "normal",
            Multiply => "multiply",
            Screen => "screen",
            Overlay => "overlay",
            Darken => "darken",
            Lighten => "lighten",
            ColorDodge => "color-dodge",
            ColorBurn => "color-burn",
            HardLight => "hard-light",
            SoftLight => "soft-light",
            Difference => "difference",
            Exclusion => "exclusion",
            Hue => "hue",
            Saturation => "saturation",
            Color => "color",
            Luminosity => "luminosity",
            PlusLighter => "plus-lighter",
        };
        Self {
            blend: blend.into(),
            isolate: group.isolate(),
        }
    }
}
