//! Bounded bitmap tracing into ordinary editable, even-odd Bézier paths.
use crate::{
    document::{Content, Editor, Shape},
    paths::{Contour, Node},
};
use image::{imageops, RgbaImage};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

const MAX_EDGES: usize = 200_000;
const MAX_NODES: usize = 20_000;
const MAX_CONTOURS: usize = 1000;
const COMPLEX: &str = "Vectorisation trop complexe : réduisez la résolution ou le nombre de couleurs, ou augmentez le filtrage des petites zones. Le bitmap est conservé.";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub mode: String,
    pub threshold: u8,
    pub invert: bool,
    pub color: String,
    pub colors: usize,
    pub alpha_threshold: u8,
    pub max_side: u32,
    pub simplify: f32,
    pub smoothing: f32,
    pub min_area: f32,
    pub use_selection: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: "mono".into(),
            threshold: 128,
            invert: false,
            color: "#000000".into(),
            colors: 8,
            alpha_threshold: 1,
            max_side: 512,
            simplify: 0.5,
            smoothing: 0.,
            min_area: 0.,
            use_selection: false,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.mode.as_str(), "mono" | "color")
            || !(2..=32).contains(&self.colors)
            || !(32..=1024).contains(&self.max_side)
            || self.alpha_threshold == 0
            || !self.simplify.is_finite()
            || !(0.0..=4.0).contains(&self.simplify)
            || !self.smoothing.is_finite()
            || !(0.0..=1.0).contains(&self.smoothing)
            || !self.min_area.is_finite()
            || !(0.0..=4096.0).contains(&self.min_area)
            || self.color.len() != 7
            || !self.color.starts_with('#')
            || crate::raster::parse_color(&self.color).is_none()
        {
            return Err("Réglages de vectorisation invalides : 2–32 couleurs, résolution 32–1024, simplification 0–4 px, lissage 0–1 et petites zones 0–4096 px²".into());
        }
        Ok(())
    }
}
pub struct Input {
    pub image: Arc<RgbaImage>,
    pub selection: Option<crate::document::Selection>,
    pub canvas: [u32; 2],
    pub offset: [i32; 2],
}
pub fn input(e: &Editor, id: u64, revision: u64) -> Result<Input, String> {
    if e.revision() != revision {
        return Err("Le document a changé. Recalculez l’aperçu avant de vectoriser.".into());
    }
    let doc = e.doc()?;
    if doc.indexed.is_some() {
        return Err("Passez en RVB avant de vectoriser en nouveau calque.".into());
    }
    if doc.animation.is_some() {
        return Err("Choisissez « Éditer cette image » avant de vectoriser ce GIF.".into());
    }
    let l = doc
        .layers
        .iter()
        .find(|l| l.id == id)
        .ok_or("Calque introuvable")?;
    let Content::Raster(image) = &l.content else {
        return Err("Choisissez un calque bitmap à vectoriser".into());
    };
    Ok(Input {
        image: image.clone(),
        selection: doc.selection.clone(),
        canvas: [doc.width, doc.height],
        offset: [l.x, l.y],
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub source_width: u32,
    pub source_height: u32,
    pub width: u32,
    pub height: u32,
    pub nodes: usize,
    pub contours: usize,
    pub paths: usize,
    pub removed_contours: usize,
    pub palette: Vec<[u8; 4]>,
    pub elapsed_ms: u128,
}
pub struct ResultData {
    pub shapes: Vec<Shape>,
    pub sampled: RgbaImage,
    pub stats: Stats,
}
#[derive(Clone, Copy)]
struct Edge {
    start: u32,
    end: u32,
    dir: u8,
}
fn distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let v = [b[0] - a[0], b[1] - a[1]];
    let len = v[0] * v[0] + v[1] * v[1];
    let t = if len == 0. {
        0.
    } else {
        (((p[0] - a[0]) * v[0] + (p[1] - a[1]) * v[1]) / len).clamp(0., 1.)
    };
    (p[0] - a[0] - t * v[0]).powi(2) + (p[1] - a[1] - t * v[1]).powi(2)
}
// Iterative subdivision avoids a stack overflow; work is bounded even on noise.
fn simplify_open(p: &[[f32; 2]], t: f32, work: &mut usize) -> Result<Vec<[f32; 2]>, String> {
    if p.len() < 3 || t == 0. {
        return Ok(p.to_vec());
    }
    let mut keep = vec![false; p.len()];
    keep[0] = true;
    keep[p.len() - 1] = true;
    let mut stack = vec![(0, p.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let mut far = None;
        let mut max = t * t;
        for i in a + 1..b {
            *work += 1;
            if *work > 8_000_000 {
                return Err(COMPLEX.into());
            }
            let d = distance(p[i], p[a], p[b]);
            if d > max {
                max = d;
                far = Some(i);
            }
        }
        if let Some(i) = far {
            keep[i] = true;
            stack.push((a, i));
            stack.push((i, b));
        }
    }
    Ok(p.iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect())
}
fn simplify_closed(p: Vec<[f32; 2]>, t: f32, work: &mut usize) -> Result<Vec<[f32; 2]>, String> {
    if t == 0. || p.len() <= 4 {
        return Ok(p);
    }
    let split = (1..p.len())
        .max_by(|&a, &b| distance(p[a], p[0], p[0]).total_cmp(&distance(p[b], p[0], p[0])))
        .unwrap();
    let mut a = simplify_open(&p[..=split], t, work)?;
    let mut second = p[split..].to_vec();
    second.push(p[0]);
    let b = simplify_open(&second, t, work)?;
    a.pop();
    a.extend_from_slice(&b[..b.len() - 1]);
    // Never collapse a closed region to a line or point.
    if a.len() < 3 {
        Ok(p)
    } else {
        Ok(a)
    }
}
fn area(p: &[[f32; 2]]) -> f32 {
    p.iter()
        .zip(p.iter().cycle().skip(1))
        .take(p.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f32>()
        .abs()
        * 0.5
}
fn nodes(p: &[[f32; 2]], smooth: f32) -> Vec<Node> {
    if smooth == 0. {
        return p
            .iter()
            .map(|&point| Node {
                point,
                incoming: point,
                outgoing: point,
            })
            .collect();
    }
    let lerp =
        |a: [f32; 2], b: [f32; 2], r: f32| [a[0] + (b[0] - a[0]) * r, a[1] + (b[1] - a[1]) * r];
    let mut out = Vec::with_capacity(p.len() * 2);
    for i in 0..p.len() {
        let c = p[i];
        let entry = lerp(c, p[(i + p.len() - 1) % p.len()], smooth * 0.25);
        let exit = lerp(c, p[(i + 1) % p.len()], smooth * 0.25);
        out.push(Node {
            point: entry,
            incoming: entry,
            outgoing: lerp(entry, c, 2. / 3.),
        });
        out.push(Node {
            point: exit,
            incoming: lerp(exit, c, 2. / 3.),
            outgoing: exit,
        });
    }
    out
}
pub fn trace(input: &Input, settings: &Settings) -> Result<ResultData, String> {
    let start = std::time::Instant::now();
    settings.validate()?;
    let (sw, sh) = input.image.dimensions();
    crate::io::dimensions(sw, sh)?;
    let ratio = (settings.max_side as f64 / sw.max(sh) as f64).min(1.);
    let (w, h) = (
        ((sw as f64 * ratio).round() as u32).max(1),
        ((sh as f64 * ratio).round() as u32).max(1),
    );
    // A regular nearest-neighbour grid is intentional: no invented colours at
    // transparent boundaries. The preview shows the same grid used by Apply.
    let mut source = (*input.image).clone();
    if settings.use_selection {
        let s = input
            .selection
            .as_ref()
            .ok_or("Aucune sélection à limiter")?;
        crate::selection::validate(s, input.canvas[0], input.canvas[1])?;
        let mask = s.mask(input.canvas[0], input.canvas[1]);
        for (x, y, p) in source.enumerate_pixels_mut() {
            let dx = x as i64 + input.offset[0] as i64;
            let dy = y as i64 + input.offset[1] as i64;
            let a =
                if dx >= 0 && dy >= 0 && dx < input.canvas[0] as i64 && dy < input.canvas[1] as i64
                {
                    mask[(dy as u32 * input.canvas[0] + dx as u32) as usize]
                } else {
                    0
                };
            p[3] = ((p[3] as u16 * a as u16 + 127) / 255) as u8;
        }
    }
    let mut sampled = if (w, h) == (sw, sh) {
        source
    } else {
        imageops::resize(&source, w, h, imageops::FilterType::Nearest)
    };
    for p in sampled.pixels_mut() {
        if p[3] < settings.alpha_threshold {
            p.0 = [0; 4];
        }
    }
    let (palette, mut labels) = if settings.mode == "mono" {
        let ink = crate::raster::parse_color(&settings.color).unwrap();
        let palette = (1..=15)
            .map(|a| [ink[0], ink[1], ink[2], a * 17])
            .collect::<Vec<_>>();
        let labels = sampled
            .pixels()
            .map(|p| {
                let lum =
                    (2126 * p[0] as u32 + 7152 * p[1] as u32 + 722 * p[2] as u32 + 5000) / 10000;
                let dark = lum < settings.threshold as u32;
                if p[3] == 0 || dark == settings.invert {
                    u16::MAX
                } else {
                    ((p[3] as u16 + 8) / 17).clamp(1, 15) - 1
                }
            })
            .collect::<Vec<_>>();
        (palette, labels)
    } else {
        let indexed = crate::indexed::quantize(&sampled, settings.colors, false)?;
        let labels = indexed
            .indices
            .iter()
            .map(|&i| {
                if indexed.palette[i as usize][3] == 0 {
                    u16::MAX
                } else {
                    i as u16
                }
            })
            .collect();
        (indexed.palette, labels)
    };
    // Drop small connected islands before constructing the edge graph. This
    // makes the small-zone control useful on noisy images at the edge budget.
    let mut removed_islands = 0;
    if settings.min_area > 0. {
        let mut visited = vec![false; labels.len()];
        let mut queue = Vec::new();
        let cell_area = sw as f32 / w as f32 * sh as f32 / h as f32;
        for seed in 0..labels.len() {
            if visited[seed] || labels[seed] == u16::MAX {
                continue;
            }
            let color = labels[seed];
            queue.clear();
            queue.push(seed);
            visited[seed] = true;
            let mut read = 0;
            while read < queue.len() {
                let i = queue[read];
                read += 1;
                let x = i % w as usize;
                let y = i / w as usize;
                for next in [
                    (x > 0).then(|| i - 1),
                    (x + 1 < w as usize).then(|| i + 1),
                    (y > 0).then(|| i - w as usize),
                    (y + 1 < h as usize).then(|| i + w as usize),
                ]
                .into_iter()
                .flatten()
                {
                    if !visited[next] && labels[next] == color {
                        visited[next] = true;
                        queue.push(next);
                    }
                }
            }
            if queue.len() as f32 * cell_area < settings.min_area {
                removed_islands += 1;
                for &i in &queue {
                    labels[i] = u16::MAX;
                }
            }
        }
    }
    let mut edges = vec![Vec::<Edge>::new(); palette.len()];
    let mut count = 0;
    let label = |x: i32, y: i32| {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            u16::MAX
        } else {
            labels[(y as u32 * w + x as u32) as usize]
        }
    };
    let vertex = |x: u32, y: u32| y * (w + 1) + x;
    for y in 0..h {
        for x in 0..w {
            let c = label(x as i32, y as i32);
            if c == u16::MAX {
                continue;
            }
            let list = &mut edges[c as usize];
            for (nx, ny, a, b, dir) in [
                (x as i32, y as i32 - 1, vertex(x, y), vertex(x + 1, y), 0),
                (
                    x as i32 + 1,
                    y as i32,
                    vertex(x + 1, y),
                    vertex(x + 1, y + 1),
                    1,
                ),
                (
                    x as i32,
                    y as i32 + 1,
                    vertex(x + 1, y + 1),
                    vertex(x, y + 1),
                    2,
                ),
                (x as i32 - 1, y as i32, vertex(x, y + 1), vertex(x, y), 3),
            ] {
                if label(nx, ny) != c {
                    count += 1;
                    if count > MAX_EDGES {
                        return Err(COMPLEX.into());
                    }
                    list.push(Edge {
                        start: a,
                        end: b,
                        dir,
                    });
                }
            }
        }
    }
    let mut shapes = vec![];
    let (mut total_nodes, mut total_contours, mut removed, mut work) = (0, 0, removed_islands, 0);
    let mut used_palette = vec![];
    for (index, list) in edges.into_iter().enumerate() {
        if list.is_empty() {
            continue;
        }
        let mut outgoing = HashMap::<u32, Vec<usize>>::new();
        for (i, e) in list.iter().enumerate() {
            outgoing.entry(e.start).or_default().push(i);
        }
        let mut visited = vec![false; list.len()];
        let mut contours = vec![];
        for first in 0..list.len() {
            if visited[first] {
                continue;
            }
            let origin = list[first].start;
            let mut at = first;
            let mut ring = vec![];
            loop {
                let edge = list[at];
                if visited[at] {
                    return Err("Contour bitmap incohérent".into());
                }
                visited[at] = true;
                ring.push(edge.start);
                if edge.end == origin {
                    break;
                }
                // Right turn at a diagonal contact keeps 4-connected regions
                // separate and preserves holes rather than crossing outlines.
                at = *outgoing
                    .get(&edge.end)
                    .and_then(|ids| {
                        ids.iter().filter(|&&i| !visited[i]).min_by_key(|&&i| {
                            match (list[i].dir + 4 - edge.dir) % 4 {
                                1 => 0,
                                0 => 1,
                                3 => 2,
                                _ => 3,
                            }
                        })
                    })
                    .ok_or("Contour bitmap ouvert")?;
            }
            let point = |v: u32| {
                [
                    (v % (w + 1)) as f32 * sw as f32 / w as f32,
                    (v / (w + 1)) as f32 * sh as f32 / h as f32,
                ]
            };
            // Remove collinear grid vertices without changing the silhouette.
            let p = ring
                .iter()
                .enumerate()
                .filter_map(|(i, &v)| {
                    let a = point(ring[(i + ring.len() - 1) % ring.len()]);
                    let b = point(v);
                    let c = point(ring[(i + 1) % ring.len()]);
                    ((b[0] - a[0]) * (c[1] - b[1]) != (b[1] - a[1]) * (c[0] - b[0])).then_some(b)
                })
                .collect::<Vec<_>>();
            if area(&p) < settings.min_area {
                removed += 1;
                continue;
            }
            let p = simplify_closed(p, settings.simplify, &mut work)?;
            let ns = nodes(&p, settings.smoothing);
            total_nodes += ns.len();
            total_contours += 1;
            if total_nodes > MAX_NODES || total_contours > MAX_CONTOURS {
                return Err(COMPLEX.into());
            }
            contours.push(Contour {
                nodes: ns,
                closed: true,
            });
        }
        if !contours.is_empty() {
            let color = palette[index];
            let mut shape = crate::clip::base();
            shape.fill = Some(format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2]));
            shape.stroke = None;
            shape.stroke_width = 0.;
            shape.opacity = color[3] as f32 / 255.;
            shape = crate::paths::normalize(shape, contours);
            crate::io::validate_shape(&shape)?;
            shapes.push(shape);
            used_palette.push(color);
        }
    }
    if shapes.is_empty() {
        return Err("Aucune zone à vectoriser avec ces réglages. Ajustez le seuil, la transparence ou les petites zones.".into());
    }
    let stats = Stats {
        source_width: sw,
        source_height: sh,
        width: w,
        height: h,
        nodes: total_nodes,
        contours: total_contours,
        paths: shapes.len(),
        removed_contours: removed,
        palette: used_palette,
        elapsed_ms: start.elapsed().as_millis(),
    };
    Ok(ResultData {
        shapes,
        sampled,
        stats,
    })
}
pub fn preview(data: &ResultData) -> Result<(RgbaImage, RgbaImage), String> {
    let (sw, sh) = (data.stats.source_width, data.stats.source_height);
    let scale = (320. / sw.max(sh) as f32).min(1.);
    let (w, h) = (
        (sw as f32 * scale).round().max(1.) as u32,
        (sh as f32 * scale).round().max(1.) as u32,
    );
    let svg = crate::vector::svg_document(
        w,
        h,
        &format!(
            "<g transform=\"scale({} {})\">{}</g>",
            w as f32 / sw as f32,
            h as f32 / sh as f32,
            data.shapes
                .iter()
                .map(crate::vector::shape_svg)
                .collect::<String>()
        ),
    );
    Ok((
        imageops::resize(&data.sampled, w, h, imageops::FilterType::Nearest),
        crate::vector::render_svg(&svg, w, h)?,
    ))
}
pub fn apply(
    e: &mut Editor,
    id: u64,
    revision: u64,
    settings: Settings,
    hide_source: bool,
) -> Result<(), String> {
    let input = input(e, id, revision)?;
    let data = trace(&input, &settings)?;
    let index = e.layer_index(id)?;
    let source = e.doc()?.layers[index].clone();
    let mut layer = e.new_vector_layer(&format!("{} — vectorisé", source.name), data.shapes);
    layer.x = source.x;
    layer.y = source.y;
    layer.opacity = source.opacity;
    layer.blend = source.blend;
    layer.effects = source.effects;
    e.active_hint = Some(layer.id);
    e.doc_mut()?.layers.insert(index + 1, layer);
    if hide_source {
        e.doc_mut()?.layers[index].visible = false;
        e.touch(index);
    }
    e.commit("Vectoriser le bitmap");
    Ok(())
}
