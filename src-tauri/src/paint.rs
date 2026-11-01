//! Dégradés SVG éditables, partagés par le rendu et les exports.
use crate::groups::{self, Matrix};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Geometry {
    Linear {
        start: [f32; 2],
        end: [f32; 2],
    },
    Radial {
        center: [f32; 2],
        focus: [f32; 2],
        radius: f32,
        focal_radius: f32,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Units {
    #[serde(rename = "objectBoundingBox")]
    ObjectBoundingBox,
    #[serde(rename = "userSpaceOnUse")]
    UserSpaceOnUse,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Spread {
    Pad,
    Reflect,
    Repeat,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stop {
    pub offset: f32,
    pub color: String,
    pub opacity: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gradient {
    pub geometry: Geometry,
    pub units: Units,
    pub transform: Matrix,
    pub spread: Spread,
    pub stops: Vec<Stop>,
}
/// A self-contained tile. Finite nested resources share a validation budget.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pattern {
    pub units: Units,
    pub transform: Matrix,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub content_width: f32,
    pub content_height: f32,
    pub children: Vec<crate::document::Shape>,
}
impl Pattern {
    pub fn validate(&self) -> Result<(), String> {
        let mut values = self.transform.to_vec();
        values.extend([
            self.x,
            self.y,
            self.width,
            self.height,
            self.content_width,
            self.content_height,
        ]);
        if values.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
            || (self.transform[0] * self.transform[3] - self.transform[1] * self.transform[2]).abs()
                < 1e-10
            || [
                self.width,
                self.height,
                self.content_width,
                self.content_height,
            ]
            .iter()
            .any(|v| *v < 1e-4 || *v > 16384.)
            || self.children.is_empty()
            || self.children.len() > 200
        {
            return Err(
                "Motif invalide : carreau positif, matrice inversible et 1 à 200 formes requis"
                    .into(),
            );
        }
        struct Scope;
        thread_local! {static BUDGET: std::cell::Cell<(usize,usize,usize)> = const {std::cell::Cell::new((0,0,0))};}
        impl Drop for Scope {fn drop(&mut self){BUDGET.with(|b|{let(d,c,n)=b.get();b.set((d-1,c,n));});}}
        BUDGET.with(|b|{let(d,c,n)=b.get();if d>=8{return Err("Motif : au plus huit niveaux de ressources imbriquées".to_string());}b.set((d+1,if d==0{0}else{c},if d==0{0}else{n}));Ok(())})?;
        let _scope=Scope;
        fn walk(shapes:&[crate::document::Shape],depth:usize,parent:Matrix)->Result<(),String>{
            use crate::document::ShapeKind;
            if depth>16{return Err("Motif : au plus seize niveaux de groupes".into());}
            for s in shapes {
                let nodes=if let ShapeKind::Path{contours,..}=&s.kind{contours.iter().map(|c|c.nodes.len()).sum()}else{0};
                BUDGET.with(|b|{let(d,c,n)=b.get();if c+1>2000||n+nodes>20_000{return Err("Motif : budget de 2 000 formes ou 20 000 nœuds dépassé".to_string());}b.set((d,c+1,n+nodes));Ok(())})?;
                crate::io::validate_shape_fields(s)?;
                if let ShapeKind::Group{children,intrinsic_w,intrinsic_h}=&s.kind{
                    if children.is_empty()||![*intrinsic_w,*intrinsic_h,s.w,s.h].iter().all(|v|v.is_finite()&&*v>=0.001)||s.fill.is_some()||s.stroke.is_some()||s.stroke_width!=0.{return Err("Motif : groupe vide, style ou dimensions invalides".into());}
                    let matrix=groups::multiply(parent,groups::frame(s));
                    if matrix.iter().any(|v|!v.is_finite()||v.abs()>1e9)||(matrix[0]*matrix[3]-matrix[1]*matrix[2]).abs()<1e-12{return Err("Motif : transformation cumulée de groupe hors limites".into());}
                    walk(children,depth+1,matrix)?;
                }
            }
            Ok(())
        }
        walk(&self.children,0,groups::IDENTITY)?;
        Ok(())
    }
    pub fn definition(&self, id: &str, transform: Matrix) -> String {
        let m = transform;
        format!("<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" preserveAspectRatio=\"none\" patternTransform=\"matrix({} {} {} {} {} {})\">{}</pattern>",self.x,self.y,self.width,self.height,self.content_width,self.content_height,m[0],m[1],m[2],m[3],m[4],m[5],self.children.iter().map(crate::vector::shape_svg).collect::<String>())
    }
}
impl Gradient {
    pub fn validate(&self) -> Result<(), String> {
        let mut values = self.transform.to_vec();
        match self.geometry {
            Geometry::Linear { start, end } => {
                values.extend(start);
                values.extend(end);
                if (start[0] - end[0]).hypot(start[1] - end[1]) < 1e-6 {
                    return Err("Les extrémités du dégradé doivent être distinctes".into());
                }
            }
            Geometry::Radial {
                center,
                focus,
                radius,
                focal_radius,
            } => {
                values.extend(center);
                values.extend(focus);
                values.extend([radius, focal_radius]);
                if radius <= 0. || focal_radius < 0. || focal_radius >= radius {
                    return Err("Rayons du dégradé radial invalides".into());
                }
            }
        }
        if values.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
            || (self.transform[0] * self.transform[3] - self.transform[1] * self.transform[2]).abs()
                < 1e-10
        {
            return Err("Coordonnées ou transformation du dégradé invalides".into());
        }
        if !(2..=64).contains(&self.stops.len())
            || self.stops.windows(2).any(|s| s[0].offset > s[1].offset)
            || self.stops.iter().any(|s| {
                !s.offset.is_finite()
                    || !s.opacity.is_finite()
                    || !(0.0..=1.0).contains(&s.offset)
                    || !(0.0..=1.0).contains(&s.opacity)
                    || s.color.len() != 7
                    || !s.color.starts_with('#')
                    || !s.color.as_bytes()[1..]
                        .iter()
                        .all(|b| b.is_ascii_hexdigit())
            })
        {
            return Err(
                "Dégradé : 2 à 64 couleurs #RRGGBB, positions ordonnées et opacités entre 0 et 1"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn definition(&self, id: &str) -> String {
        let units = match self.units {
            Units::ObjectBoundingBox => "objectBoundingBox",
            Units::UserSpaceOnUse => "userSpaceOnUse",
        };
        let spread = match self.spread {
            Spread::Pad => "pad",
            Spread::Reflect => "reflect",
            Spread::Repeat => "repeat",
        };
        let m = self.transform;
        let attrs=format!("id=\"{id}\" gradientUnits=\"{units}\" spreadMethod=\"{spread}\" gradientTransform=\"matrix({} {} {} {} {} {})\"",m[0],m[1],m[2],m[3],m[4],m[5]);
        let stops = self
            .stops
            .iter()
            .map(|s| {
                format!(
                    "<stop offset=\"{}\" stop-color=\"{}\" stop-opacity=\"{}\"/>",
                    s.offset,
                    crate::vector::escape(&s.color),
                    s.opacity
                )
            })
            .collect::<String>();
        match self.geometry {
            Geometry::Linear{start,end}=>format!("<linearGradient {attrs} x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">{stops}</linearGradient>",start[0],start[1],end[0],end[1]),
            Geometry::Radial{center,focus,radius,focal_radius}=>format!("<radialGradient {attrs} cx=\"{}\" cy=\"{}\" r=\"{radius}\" fx=\"{}\" fy=\"{}\" fr=\"{focal_radius}\">{stops}</radialGradient>",center[0],center[1],focus[0],focus[1]),
        }
    }
}
pub fn matrix(t: resvg::tiny_skia::Transform) -> Matrix {
    [t.sx, t.ky, t.kx, t.sy, t.tx, t.ty]
}
pub fn bounds(shape: &crate::document::Shape) -> Result<[f32; 4], String> {
    let mut shape = shape.clone();
    shape.transform = None;
    shape.rotation = 0.;
    shape.opacity = 1.;
    shape.fill_pattern = None;
    shape.stroke_pattern = None;
    shape.fill_gradient = None;
    shape.stroke_gradient = None;
    shape.fill = Some("#000000".into());
    shape.stroke = Some("#000000".into());
    let tree = crate::vector::parse(&crate::vector::svg_document(
        1,
        1,
        &crate::vector::shape_svg(&shape),
    ))?;
    let r = tree.root().abs_bounding_box();
    if r.width() < 1e-6 || r.height() < 1e-6 {
        return Err("Cette géométrie n’a pas de surface pour un repère local ; utilisez les coordonnées du parent".into());
    }
    Ok([r.x(), r.y(), r.width(), r.height()])
}
pub fn imported(
    paint: &resvg::usvg::Paint,
    outer: Matrix,
    allow_patterns: bool,
) -> Result<(String, Option<Gradient>, Option<Pattern>), String> {
    use resvg::usvg::{Paint, SpreadMethod};
    let (geometry, base) = match paint {
        Paint::Color(c) => {
            return Ok((
                format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue),
                None,
                None,
            ))
        }
        Paint::LinearGradient(g) => (
            Geometry::Linear {
                start: [g.x1(), g.y1()],
                end: [g.x2(), g.y2()],
            },
            &**g as &resvg::usvg::BaseGradient,
        ),
        Paint::RadialGradient(g) => (
            Geometry::Radial {
                center: [g.cx(), g.cy()],
                focus: [g.fx(), g.fy()],
                radius: g.r().get(),
                focal_radius: g.fr().get(),
            },
            &**g as &resvg::usvg::BaseGradient,
        ),
        Paint::Pattern(p) => {
            if !allow_patterns {
                return Err(
                    "Décomposition SVG impossible : motif imbriqué. Le bloc original est conservé."
                        .into(),
                );
            }
            let r = p.rect();
            let pattern = Pattern {
                units: Units::UserSpaceOnUse,
                transform: groups::multiply(outer, matrix(p.transform())),
                x: r.x(),
                y: r.y(),
                width: r.width(),
                height: r.height(),
                content_width: r.width(),
                content_height: r.height(),
                children: crate::svg_edit::pattern_children(p.root())?,
            };
            pattern.validate()?;
            return Ok(("#000000".into(), None, Some(pattern)));
        }
    };
    let gradient = Gradient {
        geometry,
        units: Units::UserSpaceOnUse,
        transform: groups::multiply(outer, matrix(base.transform())),
        spread: match base.spread_method() {
            SpreadMethod::Pad => Spread::Pad,
            SpreadMethod::Reflect => Spread::Reflect,
            SpreadMethod::Repeat => Spread::Repeat,
        },
        stops: base
            .stops()
            .iter()
            .map(|s| {
                let c = s.color();
                Stop {
                    offset: s.offset().get(),
                    opacity: s.opacity().get(),
                    color: format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue),
                }
            })
            .collect(),
    };
    gradient.validate()?;
    Ok((gradient.stops[0].color.clone(), Some(gradient), None))
}
pub fn next_id() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    format!(
        "prismePaint{}",
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        document::{Content, Editor, Shape, ShapeKind},
        io, operations, vector,
    };
    use serde_json::json;
    fn gradient() -> Gradient {
        serde_json::from_value(json!({"geometry":{"kind":"linear","start":[0,0],"end":[1,0]},"units":"objectBoundingBox","transform":[1,0,0,1,0,0],"spread":"pad","stops":[{"offset":0,"color":"#ff0000","opacity":1},{"offset":1,"color":"#0040ff","opacity":1}]})).unwrap()
    }
    fn shape() -> Shape {
        serde_json::from_value(json!({"id":7,"type":"rect","radius":0,"x":20,"y":25,"w":140,"h":90,"rotation":0,"fill":"#ff0000","stroke":null,"strokeWidth":0,"opacity":1})).unwrap()
    }
    fn compare(a: &image::RgbaImage, b: &image::RgbaImage) {
        let mut sum = 0.;
        let mut max = 0f64;
        for (p, q) in a.pixels().zip(b.pixels()) {
            for c in 0..4 {
                let d = if c == 3 {
                    (p[3] as f64 - q[3] as f64).abs()
                } else {
                    (p[c] as f64 * p[3] as f64 / 255. - q[c] as f64 * q[3] as f64 / 255.).abs()
                };
                sum += d;
                max = max.max(d);
            }
        }
        assert!(
            max <= 3. && sum / (a.as_raw().len() as f64) < 0.04,
            "max {max}, mean {}",
            sum / a.as_raw().len() as f64
        );
    }
    #[test]
    fn imported_gradients_keep_geometry_spread_alpha_and_nonuniform_transforms() {
        for defs in [
            r##"<linearGradient id="p" x1=".1" y1=".2" x2=".9" y2=".6" spreadMethod="reflect" gradientTransform="rotate(17 .5 .5)"><stop offset="0" stop-color="#ff0000" stop-opacity=".2"/><stop offset=".4" stop-color="#00aa44"/><stop offset="1" stop-color="#0040ff"/></linearGradient>"##,
            r##"<radialGradient id="p" gradientUnits="userSpaceOnUse" cx="90" cy="80" r="75" fx="70" fy="60" fr="4" gradientTransform="matrix(1 .1 .2 .8 3 4)" spreadMethod="repeat"><stop stop-color="#fff"/><stop offset="1" stop-color="#2255aa" stop-opacity=".3"/></radialGradient>"##,
        ] {
            let source = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="180"><defs>{defs}</defs><g transform="translate(12 9) scale(.85 .7)"><rect x="20" y="20" width="180" height="130" rx="12" fill="url(#p)"/><path fill="none" d="M30 160L190 150" stroke="url(#p)" stroke-width="9"/></g></svg>"##
            );
            let mut block = shape();
            block.x = 15.;
            block.y = 12.;
            block.w = 240.;
            block.h = 180.;
            block.rotation = 5.;
            block.kind = ShapeKind::Svg {
                source,
                intrinsic_w: 240.,
                intrinsic_h: 180.,
            };
            let decomposed = crate::svg_edit::decompose(&block).unwrap();
            assert!(decomposed.iter().all(|s| s.fill_gradient.is_some()));
            compare(
                &vector::rasterize(&[block], 300, 240).unwrap(),
                &vector::rasterize(&decomposed, 300, 240).unwrap(),
            );
        }
    }
    #[test]
    fn local_parent_units_and_text_bounds_preserve_actual_render() {
        for text in [false, true] {
            let mut s = shape();
            if text {
                s.kind = ShapeKind::Text {
                    text: "Texte WWW iii".into(),
                    font_family: "Segoe UI".into(),
                    font_size: 24.,
                    bold: false,
                    italic: false,
                };
            }
            s.fill_gradient = Some(gradient());
            let before = vector::rasterize(&[s.clone()], 240, 180).unwrap();
            let [x, y, w, h] = bounds(&s).unwrap();
            let g = s.fill_gradient.as_mut().unwrap();
            g.transform = [w, 0., 0., h, x, y];
            g.units = Units::UserSpaceOnUse;
            compare(&before, &vector::rasterize(&[s], 240, 180).unwrap());
        }
    }
    #[test]
    fn gradient_edits_roundtrip_and_failures_do_not_change_history() {
        let mut e = Editor::default();
        let run = |e: &mut Editor, v| operations::execute(e, serde_json::from_value(v).unwrap());
        run(
            &mut e,
            json!({"type":"new","width":240,"height":180,"background":null}),
        )
        .unwrap();
        run(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
        let id = e.active_hint.unwrap();
        let mut s = shape();
        s.id = 0;
        s.fill_gradient = Some(gradient());
        run(&mut e, json!({"type":"shape","id":id,"shape":s})).unwrap();
        let before = vector::flatten(e.doc().unwrap()).unwrap();
        let loaded = io::read_project(&io::project_bytes(e.doc().unwrap()).unwrap()).unwrap();
        assert_eq!(before, vector::flatten(&loaded).unwrap());
        let Content::Vector(shapes) = &e.doc().unwrap().layers[1].content else {
            panic!()
        };
        let mut bad = shapes[0].clone();
        bad.fill_gradient.as_mut().unwrap().stops[0].opacity = 2.;
        let history = e.info().history;
        assert!(run(&mut e, json!({"type":"shape","id":id,"shape":bad})).is_err());
        assert_eq!(history, e.info().history);
        assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
        run(&mut e, json!({"type":"undo"})).unwrap();
        assert_ne!(before, vector::flatten(e.doc().unwrap()).unwrap());
        run(&mut e, json!({"type":"redo"})).unwrap();
        assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
    }

    #[test]
    fn boolean_result_keeps_the_first_gradient_in_parent_coordinates() {
        let mut a = shape();
        a.fill_gradient = Some(gradient());
        a.rotation = 17.;
        a.transform = Some([1., 0.1, 0.15, 1., 8., 3.]);
        let a = crate::paths::convert(&a).unwrap();
        let mut b = shape();
        b.x = 15.;
        b.y = 5.;
        b.w = 65.;
        b.h = 150.;
        let b = crate::paths::convert(&b).unwrap();
        let result = crate::paths::boolean(&a, &b, "difference").unwrap();
        assert!(result.fill_gradient.is_some());
        assert!(result.transform.is_none());
        let before = vector::rasterize(&[a], 240, 180).unwrap();
        let after = vector::rasterize(&[result], 240, 180).unwrap();
        let mut checked = 0;
        let mut removed = 0;
        for (p, q) in before.pixels().zip(after.pixels()) {
            if p[3] == 255 && q[3] == 255 {
                checked += 1;
                for c in 0..3 {
                    assert!(p[c].abs_diff(q[c]) <= 2)
                }
            }
            if p[3] == 255 && q[3] == 0 {
                removed += 1
            }
        }
        assert!(checked > 1000 && removed > 1000);
    }
}
