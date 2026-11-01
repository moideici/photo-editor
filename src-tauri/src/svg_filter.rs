//! Chaînes de filtres SVG normalisées ; aucun XML arbitraire dans le projet.
use crate::{
    document::Shape,
    groups::{self, Matrix},
};
use resvg::usvg;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Primitive {
    pub kind: String,
    pub attributes: BTreeMap<String, String>,
    #[serde(default)]
    pub children: Vec<Primitive>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<FilterResource>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilterResource {
    pub children: Vec<Shape>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Filter {
    pub region: [f32; 4],
    pub primitives: Vec<Primitive>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilterChain {
    pub transform: Matrix,
    pub filters: Vec<Filter>,
}
thread_local! {static RESOURCE_BUDGET:std::cell::Cell<(usize,usize,usize)>=const{std::cell::Cell::new((0,0,0))};}
struct ResourceGuard;
impl ResourceGuard {
    fn enter() -> Result<Self, String> {
        RESOURCE_BUDGET.with(|b| {
            let (d, s, n) = b.get();
            if d >= 8 {
                return Err(error("plus de huit niveaux de ressources de filtres"));
            }
            b.set((
                d + 1,
                if d == 0 { 0 } else { s },
                if d == 0 { 0 } else { n },
            ));
            Ok(Self)
        })
    }
}
impl Drop for ResourceGuard {
    fn drop(&mut self) {
        RESOURCE_BUDGET.with(|b| {
            let (d, s, n) = b.get();
            b.set((d - 1, s, n));
        });
    }
}
impl FilterResource {
    pub fn validate(&self) -> Result<(), String> {
        fn count(shapes: &[Shape], depth: usize) -> Result<(), String> {
            if depth > 16 {
                return Err(error("plus de seize niveaux dans une ressource"));
            }
            for s in shapes {
                if s.text_path.as_ref().is_some_and(|p| p.source_id.is_some()) {
                    return Err(error(
                        "détachez le chemin du texte avant de l’intégrer à une ressource de filtre",
                    ));
                }
                RESOURCE_BUDGET.with(|b| {
                    let (d, mut count, mut nodes) = b.get();
                    count += 1;
                    if let crate::document::ShapeKind::Path { contours, .. } = &s.kind {
                        nodes += contours.iter().map(|c| c.nodes.len()).sum::<usize>();
                    }
                    if count > 1000 || nodes > 20_000 {
                        return Err(error("ressources : plus de 1 000 formes ou 20 000 nœuds"));
                    }
                    b.set((d, count, nodes));
                    Ok(())
                })?;
                if let crate::document::ShapeKind::Group { children, .. } = &s.kind {
                    count(children, depth + 1)?;
                }
            }
            Ok(())
        }
        // Direct preview/file commands have no enclosing chain. Share the
        // same bound with resources encountered through masks or nested filters.
        let _guard = if RESOURCE_BUDGET.with(|b| b.get().0 == 0) {
            Some(ResourceGuard::enter()?)
        } else {
            None
        };
        if self.children.is_empty() {
            return Err(error(
                "la ressource image doit conserver au moins une forme",
            ));
        }
        count(&self.children, 0)?;
        let mut shapes = self.children.clone();
        let mut id = 0;
        groups::assign_ids(&mut shapes, &mut || {
            id += 1;
            id
        });
        groups::validate(&shapes)
    }
    pub fn preview(&self) -> Result<image::RgbaImage, String> {
        self.validate()?;
        let mut s = crate::clip::base();
        s.fill = None;
        s.kind = crate::document::ShapeKind::Group {
            children: self.children.clone(),
            intrinsic_w: 1.,
            intrinsic_h: 1.,
        };
        preview(&s)
    }
    pub fn from_asset(
        asset: crate::embedded::ImageAsset,
        width: f32,
        height: f32,
        fit: &str,
        align: &str,
    ) -> Result<Self, String> {
        asset.validate()?;
        if ![width, height]
            .iter()
            .all(|v| v.is_finite() && *v >= 1. && *v <= 16384.)
            || width * height > 16_777_216.
        {
            return Err(error("dimensions de placement de l’image invalides"));
        }
        if !["meet", "slice", "none"].contains(&fit)
            || ![
                "xMinYMin", "xMidYMin", "xMaxYMin", "xMinYMid", "xMidYMid", "xMaxYMid", "xMinYMax",
                "xMidYMax", "xMaxYMax",
            ]
            .contains(&align)
        {
            return Err(error("ajustement ou alignement de l’image invalide"));
        }
        let aspect = if fit == "none" {
            "none".into()
        } else {
            format!("{align} {fit}")
        };
        let source=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\"><image width=\"{width}\" height=\"{height}\" style=\"image-rendering:{}\" preserveAspectRatio=\"{aspect}\" href=\"{}\"/></svg>",asset.rendering,asset.url());
        let mut s = crate::clip::base();
        s.fill = None;
        s.w = width;
        s.h = height;
        s.kind = crate::document::ShapeKind::Svg {
            source,
            intrinsic_w: width,
            intrinsic_h: height,
        };
        let value = Self {
            children: crate::svg_edit::in_filter_resource(|| crate::svg_edit::decompose(&s))?,
        };
        value.validate()?;
        Ok(value)
    }
}

fn error(why: &str) -> String {
    format!("Filtre SVG : {why}")
}
fn node(kind: &str, attrs: &[(&str, &str)], children: Vec<Primitive>) -> Primitive {
    Primitive {
        resource: None,
        kind: kind.into(),
        attributes: attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        children,
    }
}
/// This same catalogue drives creation and the attribute allow-list in the editor.
pub fn catalog() -> Vec<Primitive> {
    let mut out = vec![
        node(
            "feGaussianBlur",
            &[("in", "SourceGraphic"), ("stdDeviation", "3 3")],
            vec![],
        ),
        node(
            "feDropShadow",
            &[
                ("in", "SourceGraphic"),
                ("stdDeviation", "3 3"),
                ("dx", "5"),
                ("dy", "5"),
                ("flood-color", "#000000"),
                ("flood-opacity", "0.6"),
            ],
            vec![],
        ),
        node(
            "feOffset",
            &[("in", "SourceGraphic"), ("dx", "5"), ("dy", "5")],
            vec![],
        ),
        node(
            "feColorMatrix",
            &[
                ("in", "SourceGraphic"),
                ("type", "matrix"),
                ("values", "1 0 0 0 0 0 1 0 0 0 0 0 1 0 0 0 0 0 1 0"),
            ],
            vec![],
        ),
        node(
            "feComponentTransfer",
            &[("in", "SourceGraphic")],
            ["feFuncR", "feFuncG", "feFuncB", "feFuncA"]
                .iter()
                .map(|k| node(k, &[("type", "identity")], vec![]))
                .collect(),
        ),
        node(
            "feBlend",
            &[
                ("in", "SourceGraphic"),
                ("in2", "SourceGraphic"),
                ("mode", "multiply"),
            ],
            vec![],
        ),
        node(
            "feComposite",
            &[
                ("in", "SourceGraphic"),
                ("in2", "SourceGraphic"),
                ("operator", "over"),
                ("k1", "0"),
                ("k2", "0"),
                ("k3", "0"),
                ("k4", "0"),
            ],
            vec![],
        ),
        node(
            "feMerge",
            &[],
            vec![node("feMergeNode", &[("in", "SourceGraphic")], vec![])],
        ),
        node(
            "feFlood",
            &[("flood-color", "#000000"), ("flood-opacity", "1")],
            vec![],
        ),
        node(
            "feMorphology",
            &[
                ("in", "SourceGraphic"),
                ("operator", "dilate"),
                ("radius", "2 2"),
            ],
            vec![],
        ),
        node(
            "feConvolveMatrix",
            &[
                ("in", "SourceGraphic"),
                ("order", "3 3"),
                ("kernelMatrix", "0 -1 0 -1 5 -1 0 -1 0"),
                ("divisor", "1"),
                ("bias", "0"),
                ("targetX", "1"),
                ("targetY", "1"),
                ("edgeMode", "duplicate"),
                ("preserveAlpha", "true"),
            ],
            vec![],
        ),
        node(
            "feDisplacementMap",
            &[
                ("in", "SourceGraphic"),
                ("in2", "SourceGraphic"),
                ("scale", "10"),
                ("xChannelSelector", "R"),
                ("yChannelSelector", "G"),
            ],
            vec![],
        ),
        node(
            "feTurbulence",
            &[
                ("baseFrequency", "0.05 0.05"),
                ("numOctaves", "2"),
                ("seed", "1"),
                ("stitchTiles", "noStitch"),
                ("type", "fractalNoise"),
            ],
            vec![],
        ),
        node("feTile", &[("in", "SourceGraphic")], vec![]),
        node(
            "feDiffuseLighting",
            &[
                ("in", "SourceGraphic"),
                ("surfaceScale", "2"),
                ("diffuseConstant", "1"),
                ("lighting-color", "#ffffff"),
            ],
            vec![node(
                "feDistantLight",
                &[("azimuth", "45"), ("elevation", "45")],
                vec![],
            )],
        ),
        node(
            "feSpecularLighting",
            &[
                ("in", "SourceGraphic"),
                ("surfaceScale", "2"),
                ("specularConstant", "1"),
                ("specularExponent", "10"),
                ("lighting-color", "#ffffff"),
            ],
            vec![node(
                "feDistantLight",
                &[("azimuth", "45"), ("elevation", "45")],
                vec![],
            )],
        ),
    ];
    let mut image = node("feImage", &[], vec![]);
    let mut s = crate::clip::base();
    s.kind = crate::document::ShapeKind::Rect { radius: 0. };
    s.w = 128.;
    s.h = 128.;
    s.fill = Some("#ffffff".into());
    image.resource = Some(FilterResource { children: vec![s] });
    out.push(image);
    out
}
pub fn child_catalog() -> Vec<Primitive> {
    let mut out = vec![
        node("feMergeNode", &[("in", "SourceGraphic")], vec![]),
        node(
            "feDistantLight",
            &[("azimuth", "45"), ("elevation", "45")],
            vec![],
        ),
        node(
            "fePointLight",
            &[("x", "0"), ("y", "0"), ("z", "100")],
            vec![],
        ),
        node(
            "feSpotLight",
            &[
                ("x", "0"),
                ("y", "0"),
                ("z", "100"),
                ("pointsAtX", "50"),
                ("pointsAtY", "50"),
                ("pointsAtZ", "0"),
                ("specularExponent", "1"),
                ("limitingConeAngle", "90"),
            ],
            vec![],
        ),
    ];
    out.extend(
        ["feFuncR", "feFuncG", "feFuncB", "feFuncA"]
            .iter()
            .map(|k| {
                node(
                    k,
                    &[
                        ("type", "identity"),
                        ("tableValues", "0 1"),
                        ("slope", "1"),
                        ("intercept", "0"),
                        ("amplitude", "1"),
                        ("exponent", "1"),
                        ("offset", "0"),
                    ],
                    vec![],
                )
            }),
    );
    out
}
impl Primitive {
    fn xml(&self, defs: &mut String) -> String {
        let attrs = self
            .attributes
            .iter()
            .map(|(k, v)| format!(" {k}=\"{}\"", crate::vector::escape(v)))
            .collect::<String>();
        if let Some(resource) = &self.resource {
            let id = crate::paint::next_id();
            *defs += &format!(
                "<g id=\"{id}\">{}</g>",
                resource
                    .children
                    .iter()
                    .map(crate::vector::shape_svg)
                    .collect::<String>()
            );
            return format!("<feImage{attrs} href=\"#{id}\"/>");
        }
        format!(
            "<{}{attrs}>{}</{}>",
            self.kind,
            self.children
                .iter()
                .map(|c| c.xml(defs))
                .collect::<String>(),
            self.kind
        )
    }
}
fn parse_node(n: roxmltree::Node<'_, '_>) -> Primitive {
    Primitive {
        resource: None,
        kind: n.tag_name().name().into(),
        attributes: n
            .attributes()
            .map(|a| (a.name().to_string(), a.value().to_string()))
            .collect(),
        children: n
            .children()
            .filter(|c| c.is_element())
            .map(parse_node)
            .collect(),
    }
}
fn input(i: &usvg::filter::Input) -> String {
    match i {
        usvg::filter::Input::SourceGraphic => "SourceGraphic".into(),
        usvg::filter::Input::SourceAlpha => "SourceAlpha".into(),
        usvg::filter::Input::Reference(s) => s.clone(),
    }
}
impl FilterChain {
    pub fn imported(
        group: &usvg::Group,
        transform: Matrix,
        depth: usize,
        nodes: &mut usize,
        total: &mut usize,
    ) -> Result<Option<Self>, String> {
        if group.filters().is_empty() {
            return Ok(None);
        }
        // The normalized writer owns the SVG spelling of every primitive.
        // Its lighting writer omits
        // `in`; restore that input explicitly from the parsed graph below.
        let filters = group
            .filters()
            .iter()
            .map(|f| {
                let text = FILTER_DEFS.with(|slot| slot.borrow().clone());
                let doc = roxmltree::Document::parse(&text).map_err(|e| e.to_string())?;
                let n = doc
                    .descendants()
                    .find(|n| n.has_tag_name("filter") && n.attribute("id") == Some(f.id()))
                    .ok_or_else(|| error("définition normalisée absente"))?;
                let mut primitives = n
                    .children()
                    .filter(|c| c.is_element())
                    .map(parse_node)
                    .collect::<Vec<_>>();
                for (p, raw) in primitives.iter_mut().zip(f.primitives()) {
                    match raw.kind() {
                        usvg::filter::Kind::DiffuseLighting(l) => {
                            p.attributes.insert("in".into(), input(l.input()));
                        }
                        usvg::filter::Kind::SpecularLighting(l) => {
                            p.attributes.insert("in".into(), input(l.input()));
                        }
                        usvg::filter::Kind::Image(image) => {
                            p.attributes.remove("href");
                            p.resource = Some(FilterResource {
                                children: crate::svg_edit::filter_resource_children(
                                    image.root(),
                                    depth + 1,
                                    nodes,
                                    total,
                                )?,
                            });
                        }
                        _ => (),
                    }
                }
                let r = f.rect();
                Ok(Filter {
                    region: [r.x(), r.y(), r.width(), r.height()],
                    primitives,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let value = Self { transform, filters };
        value
            .validate()
            .map_err(|e| format!("{e}. Le bloc original est conservé."))?;
        Ok(Some(value))
    }
    pub fn apply(&mut self, m: Matrix) {
        self.transform = groups::multiply(m, self.transform);
    }
    pub fn body(&self, defs: &mut String, body: String, attributes: &str) -> String {
        let mut urls = Vec::new();
        for f in &self.filters {
            let id = crate::paint::next_id();
            let [x, y, w, h] = f.region;
            let primitives = f.primitives.iter().map(|p| p.xml(defs)).collect::<String>();
            *defs+=&format!("<filter id=\"{id}\" filterUnits=\"userSpaceOnUse\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\">{primitives}</filter>");
            urls.push(format!("url(#{id})"));
        }
        let m = self.transform;
        let inv = inverse(m);
        format!(
            "<g transform=\"{}\" filter=\"{}\" {attributes}><g transform=\"{}\">{body}</g></g>",
            matrix(m),
            urls.join(" "),
            matrix(inv)
        )
    }
    pub fn validate(&self) -> Result<(), String> {
        let _guard = ResourceGuard::enter()?;
        let m = self.transform;
        if m.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
            || (m[0] * m[3] - m[1] * m[2]).abs() < 1e-10
        {
            return Err(error("matrice invalide"));
        }
        if self.filters.is_empty() || self.filters.len() > 8 {
            return Err(error("une à huit chaînes attendues"));
        }
        for f in &self.filters {
            let [x, y, w, h] = f.region;
            if [x, y, w, h].iter().any(|v| !v.is_finite() || v.abs() > 1e6)
                || w <= 0.
                || h <= 0.
                || w > 16384.
                || h > 16384.
                || w * h > 16_777_216.
            {
                return Err(error(
                    "région invalide ou supérieure à 16 millions de pixels",
                ));
            }
            if f.primitives.is_empty() || f.primitives.len() > 64 {
                return Err(error("une à 64 étapes attendues"));
            }
            let mut names = HashSet::from(["SourceGraphic".to_string(), "SourceAlpha".to_string()]);
            for p in &f.primitives {
                validate_node(p, false, &names)?;
                if let Some(name) = p.attributes.get("result") {
                    if name.is_empty()
                        || name.len() > 128
                        || name.chars().any(char::is_whitespace)
                        || !names.insert(name.clone())
                    {
                        return Err(error("nom de résultat vide, réservé ou dupliqué"));
                    }
                }
            }
        }
        Ok(())
    }
}
thread_local! { static FILTER_DEFS: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) }; }
/// Scope normalized definitions per synchronous decomposition; nested SVGs
/// restore their parent's definitions even when conversion fails.
pub fn with_definitions<T>(tree: &usvg::Tree, f: impl FnOnce() -> T) -> T {
    let old = FILTER_DEFS.with(|s| {
        s.replace(tree.to_string(&usvg::WriteOptions {
            coordinates_precision: 8,
            transforms_precision: 8,
            ..Default::default()
        }))
    });
    let out = f();
    FILTER_DEFS.with(|s| s.replace(old));
    out
}
pub fn attach(group: &mut Shape, mut filters: FilterChain) {
    filters.apply(groups::translation(-group.x, -group.y));
    group.filters = Some(filters);
}
fn matrix(m: Matrix) -> String {
    format!(
        "matrix({} {} {} {} {} {})",
        m[0], m[1], m[2], m[3], m[4], m[5]
    )
}
pub(crate) fn inverse(m: Matrix) -> Matrix {
    let d = m[0] * m[3] - m[1] * m[2];
    [
        m[3] / d,
        -m[1] / d,
        -m[2] / d,
        m[0] / d,
        (m[2] * m[5] - m[3] * m[4]) / d,
        (m[1] * m[4] - m[0] * m[5]) / d,
    ]
}

fn validate_node(p: &Primitive, child: bool, names: &HashSet<String>) -> Result<(), String> {
    if p.kind == "feImage" {
        p.resource
            .as_ref()
            .ok_or_else(|| error("source de l’image absente"))?
            .validate()?;
    } else if p.resource.is_some() {
        return Err(error("une ressource image est réservée à feImage"));
    }
    let templates = if child { child_catalog() } else { catalog() };
    let template = templates.iter().find(|c| c.kind == p.kind).ok_or_else(|| {
        error(&format!(
            "effet {} non pris en charge ; le bloc original est conservé",
            p.kind
        ))
    })?;
    let common = [
        "x",
        "y",
        "width",
        "height",
        "color-interpolation-filters",
        "result",
    ];
    for (k, v) in &p.attributes {
        if v.len() > 8192
            || !(template.attributes.contains_key(k) || (!child && common.contains(&k.as_str())))
        {
            return Err(error(&format!(
                "paramètre {k} non autorisé pour {}",
                p.kind
            )));
        }
        if k == "in" || k == "in2" {
            if !names.contains(v) {
                return Err(error(&format!(
                    "entrée « {v} » absente ou située après cette étape"
                )));
            }
            continue;
        }
        let allowed=match k.as_str(){
            "result"=>continue,
            "color-interpolation-filters"=>Some("sRGB linearRGB"),
            "mode"=>Some("normal multiply screen overlay darken lighten color-dodge color-burn hard-light soft-light difference exclusion hue saturation color luminosity"),
            "operator"=>Some(if p.kind=="feMorphology"{"erode dilate"}else{"over in out atop xor arithmetic"}),
            "type"=>Some(if p.kind=="feColorMatrix"{"matrix saturate hueRotate luminanceToAlpha"}else if p.kind=="feTurbulence"{"turbulence fractalNoise"}else{"identity table discrete linear gamma"}),
            "edgeMode"=>Some("none duplicate wrap"),"preserveAlpha"=>Some("true false"),"stitchTiles"=>Some("stitch noStitch"),"xChannelSelector"|"yChannelSelector"=>Some("R G B A"),
            "flood-color"|"lighting-color"=>{if v.len()!=7||!v.starts_with('#')||!v[1..].chars().all(|c|c.is_ascii_hexdigit()){return Err(error("couleur attendue : #RRGGBB"));}continue;},
            _=>None
        };
        if let Some(allowed) = allowed {
            if !allowed.split_whitespace().any(|a| a == v) {
                return Err(error(&format!("valeur invalide pour {k}")));
            }
            continue;
        }
        let numbers = v
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|v| !v.is_empty())
            .map(|v| {
                v.parse::<f32>()
                    .map_err(|_| error(&format!("nombre invalide pour {k}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if numbers.is_empty()
            || numbers.len() > 225
            || numbers.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
        {
            return Err(error(&format!("valeur hors limites pour {k}")));
        }
        let pair = matches!(
            k.as_str(),
            "stdDeviation" | "radius" | "baseFrequency" | "order"
        );
        if (!pair
            && !matches!(k.as_str(), "values" | "tableValues" | "kernelMatrix")
            && numbers.len() != 1)
            || (pair && numbers.len() > 2)
        {
            return Err(error(&format!("nombre de valeurs invalide pour {k}")));
        }
        if matches!(
            k.as_str(),
            "stdDeviation" | "radius" | "baseFrequency" | "diffuseConstant" | "specularConstant"
        ) && numbers.iter().any(|n| *n < 0.)
        {
            return Err(error(&format!("{k} doit être positif ou nul")));
        }
        if matches!(k.as_str(), "stdDeviation" | "radius") && numbers.iter().any(|n| *n > 512.) {
            return Err(error("rayon supérieur à 512"));
        }
        if k == "baseFrequency" && numbers.iter().any(|n| *n > 10.) {
            return Err(error("fréquence supérieure à 10"));
        }
        if k == "numOctaves" && (numbers[0] < 1. || numbers[0] > 10. || numbers[0].fract() != 0.) {
            return Err(error("une à dix octaves attendues"));
        }
        if k == "flood-opacity" && !(0.0..=1.0).contains(&numbers[0]) {
            return Err(error("opacité hors de 0–1"));
        }
        if k == "seed" && numbers[0].fract() != 0. {
            return Err(error("la graine doit être entière"));
        }
        if k == "specularExponent"
            && (numbers[0] < if p.kind == "feSpotLight" { 0. } else { 1. } || numbers[0] > 128.)
        {
            return Err(error(
                "concentration lumineuse hors limites (1–128, 0–128 pour un projecteur)",
            ));
        }
        if matches!(k.as_str(), "width" | "height" | "divisor") && numbers[0] == 0. {
            return Err(error(&format!("{k} ne peut pas être nul")));
        }
        if matches!(k.as_str(), "width" | "height") && numbers[0] < 0. {
            return Err(error("dimensions négatives"));
        }
    }
    if p.kind == "feColorMatrix" {
        let t = p
            .attributes
            .get("type")
            .map(String::as_str)
            .unwrap_or("matrix");
        if t != "luminanceToAlpha" {
            let n = number_list(p, "values")?;
            if n.len() != if t == "matrix" { 20 } else { 1 } || t == "saturate" && n[0] < 0. {
                return Err(error("valeurs de matrice couleur invalides"));
            }
        }
    }
    if p.kind == "feConvolveMatrix" {
        let order = number_list(p, "order")?;
        let x = order[0];
        let y = *order.get(1).unwrap_or(&x);
        if x < 1.
            || y < 1.
            || x > 15.
            || y > 15.
            || x.fract() != 0.
            || y.fract() != 0.
            || number_list(p, "kernelMatrix")?.len() != (x * y) as usize
        {
            return Err(error(
                "noyau attendu : une à 15 lignes/colonnes avec toutes ses valeurs",
            ));
        }
        for (k, n) in [("targetX", x), ("targetY", y)] {
            if let Some(v) = p.attributes.get(k) {
                let v = v
                    .parse::<f32>()
                    .map_err(|_| error("cible de noyau invalide"))?;
                if v < 0. || v >= n || v.fract() != 0. {
                    return Err(error("cible de noyau hors limites"));
                }
            }
        }
    }
    if p.kind == "feMerge" {
        if p.children.is_empty()
            || p.children.len() > 64
            || p.children.iter().any(|c| c.kind != "feMergeNode")
        {
            return Err(error("fusion : une à 64 entrées attendues"));
        }
    } else if p.kind == "feComponentTransfer" {
        let mut seen = HashSet::new();
        if p.children.len() != 4
            || p.children.iter().any(|c| {
                !matches!(
                    c.kind.as_str(),
                    "feFuncR" | "feFuncG" | "feFuncB" | "feFuncA"
                ) || !seen.insert(&c.kind)
            })
        {
            return Err(error("transfert : quatre canaux distincts attendus"));
        }
    } else if matches!(p.kind.as_str(), "feDiffuseLighting" | "feSpecularLighting") {
        if p.children.len() != 1
            || !matches!(
                p.children[0].kind.as_str(),
                "feDistantLight" | "fePointLight" | "feSpotLight"
            )
        {
            return Err(error("une source lumineuse attendue"));
        }
    } else if !p.children.is_empty() {
        return Err(error("sous-étape non autorisée"));
    }
    for c in &p.children {
        validate_node(c, true, names)?;
    }
    Ok(())
}
fn number_list(p: &Primitive, key: &str) -> Result<Vec<f32>, String> {
    p.attributes
        .get(key)
        .ok_or_else(|| error(&format!("paramètre {key} manquant")))?
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<f32>().map_err(|_| error("nombre invalide")))
        .collect()
}

pub fn preview(s: &Shape) -> Result<image::RgbaImage, String> {
    let body = crate::vector::shape_svg(s);
    let tree = crate::vector::internal_tree(&crate::vector::svg_document(1, 1, &body))?;
    let r = tree.root().abs_layer_bounding_box();
    let scale = (224. / r.width()).min(224. / r.height()).min(2.);
    let (w, h) = (
        (r.width() * scale).ceil().max(1.) as u32,
        (r.height() * scale).ceil().max(1.) as u32,
    );
    crate::vector::render_svg(&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{} {} {} {}\">{body}</svg>",r.x(),r.y(),r.width(),r.height()),w,h)
}

/// PDF converters use an older filter renderer. Sample only the affected
/// subtree with the editor's renderer on the page pixel grid; preserve its
/// outer opacity/blend and all unrelated vector content in the export copy.
pub fn pdf_image(
    s: &Shape,
    parent: Matrix,
    crop: [f32; 4],
    pixels: &mut u64,
    viewport: Option<(&Shape, Matrix)>,
) -> Result<Shape, String> {
    let (x, y) = (crop[0].floor(), crop[1].floor());
    let (w, h) = (
        (crop[2].ceil() - x).max(1.) as u32,
        (crop[3].ceil() - y).max(1.) as u32,
    );
    crate::io::dimensions(w, h)
        .map_err(|_| error("export PDF : surface des effets trop grande ; exportez en SVG"))?;
    *pixels += w as u64 * h as u64;
    if *pixels > 16_777_216 {
        return Err(error(
            "export PDF : plus de 16 millions de pixels à préparer ; exportez en SVG",
        ));
    }
    let mut source = s.clone();
    // Bake opacity with the native filter paint. Reapplying it later as a
    // PDF graphics-state opacity changes constant colors through a second
    // alpha quantization (up to four RGB levels in the independent fixture).
    source.group_style = None;
    let source = if let Some((frame, world)) = viewport {
        let mut frame = frame.clone();
        if let crate::document::ShapeKind::Group { children, .. } = &mut frame.kind {
            *children = vec![source];
        }
        frame.opacity = 1.;
        frame.group_style = None;
        frame.transform = Some(groups::multiply(
            groups::translation(-x, -y),
            groups::multiply(world, frame.transform.unwrap_or(groups::IDENTITY)),
        ));
        frame
    } else {
        source.transform = Some(groups::multiply(
            groups::translation(-x, -y),
            groups::multiply(parent, source.transform.unwrap_or(groups::IDENTITY)),
        ));
        source
    };
    let image = crate::vector::rasterize(&[source], w, h)?;
    let mut lo = [w, h];
    let mut hi = [0, 0];
    for (px, py, p) in image.enumerate_pixels() {
        if p[3] > 0 {
            lo[0] = lo[0].min(px);
            lo[1] = lo[1].min(py);
            hi[0] = hi[0].max(px + 1);
            hi[1] = hi[1].max(py + 1);
        }
    }
    let mut result = crate::clip::base();
    result.id = s.id;
    result.fill = None;
    result.opacity = 1.;
    result.transform = Some(inverse(parent));
    let bytes = if hi[0] > lo[0] && hi[1] > lo[1] {
        result.x = x + lo[0] as f32;
        result.y = y + lo[1] as f32;
        result.w = (hi[0] - lo[0]) as f32;
        result.h = (hi[1] - lo[1]) as f32;
        crate::io::png_bytes(
            &image::imageops::crop_imm(&image, lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1])
                .to_image(),
        )?
    } else {
        result.x = 0.;
        result.y = 0.;
        result.w = 1.;
        result.h = 1.;
        crate::io::png_bytes(&image::RgbaImage::new(1, 1))?
    };
    result.kind = crate::document::ShapeKind::Image {
        asset: crate::embedded::ImageAsset::from_bytes("image/png", &bytes, "optimizeSpeed")?,
    };
    if let Some(style) = s.group_style.as_ref().filter(|s| !s.neutral()) {
        let opacity = result.opacity;
        result.opacity = 1.;
        let mut group = crate::svg_edit::wrap_group(vec![result], opacity, Some(style.clone()))?;
        group.id = s.id;
        Ok(group)
    } else {
        Ok(result)
    }
}

/// A referenced filter pruned by the parser must not delete its recoverable
/// source shapes during editable import. Unknown primitives and feImage stay
/// blocks until their own editable resource model exists.
pub fn check_source(source: &str, tree: &usvg::Tree) -> Result<(), String> {
    let doc = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    let known = tree
        .filters()
        .iter()
        .map(|f| f.id())
        .collect::<HashSet<_>>();
    let referenced = referenced_filters(&doc);
    let allowed = catalog()
        .into_iter()
        .chain(child_catalog())
        .map(|p| p.kind)
        .collect::<HashSet<_>>();
    for n in doc.descendants().filter(|n| {
        n.has_tag_name("filter") && n.attribute("id").is_some_and(|id| referenced.contains(id))
    }) {
        let id = n.attribute("id").unwrap();
        let normalized = tree.filters().iter().find(|f| f.id() == id);
        let template = filter_template(&doc, n)?;
        let images = template
            .children()
            .filter(|n| n.has_tag_name("feImage"))
            .count();
        let normalized_images = normalized
            .map(|f| {
                f.primitives()
                    .iter()
                    .filter(|p| matches!(p.kind(), usvg::filter::Kind::Image(_)))
                    .count()
            })
            .unwrap_or(0);
        if !known.contains(id)
            || images != normalized_images
            || template
                .children()
                .filter(|n| n.is_element())
                .any(|n| !allowed.contains(n.tag_name().name()))
        {
            return Err(format!("Décomposition SVG impossible : filtre « {id} » ou ressource feImage éliminé pendant la normalisation, ou effet inconnu. Le bloc original est conservé."));
        }
    }
    Ok(())
}
fn filter_template<'a>(
    doc: &'a roxmltree::Document<'a>,
    mut node: roxmltree::Node<'a, 'a>,
) -> Result<roxmltree::Node<'a, 'a>, String> {
    let mut visited = HashSet::new();
    for _ in 0..64 {
        if !visited.insert(node.id()) {
            return Err(error(
                "cycle de références de filtres ; le bloc original est conservé",
            ));
        }
        if node.children().any(|n| n.is_element()) {
            return Ok(node);
        }
        let Some(id) = node
            .attribute("href")
            .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")))
            .and_then(|v| v.strip_prefix('#'))
        else {
            return Ok(node);
        };
        let Some(next) = doc
            .descendants()
            .find(|n| n.has_tag_name("filter") && n.attribute("id") == Some(id))
        else {
            return Ok(node);
        };
        node = next;
    }
    Err(error(
        "plus de 64 références de filtres ; le bloc original est conservé",
    ))
}
pub(crate) fn referenced_filters(doc: &roxmltree::Document<'_>) -> HashSet<String> {
    let mut referenced = HashSet::new();
    for n in doc.descendants().filter(|n| n.is_element()) {
        for text in n
            .attributes()
            .filter(|a| a.name() == "filter" || a.name() == "style")
            .map(|a| a.value())
            .chain(if n.has_tag_name("style") {
                n.text()
            } else {
                None
            })
        {
            let lower = text.to_ascii_lowercase();
            let mut at = 0;
            while let Some(start) = lower[at..].find("url(") {
                let begin = at + start + 4;
                let Some(end) = text[begin..].find(')') else {
                    break;
                };
                let id = &text[begin..begin + end];
                if let Some(id) = id.trim().trim_matches(['\'', '"']).strip_prefix('#') {
                    referenced.insert(id.to_string());
                }
                at = begin + end + 1;
            }
        }
    }
    referenced
}
pub(crate) fn referenced_filter_templates(doc: &roxmltree::Document<'_>) -> HashSet<String> {
    let mut ids = referenced_filters(doc);
    for _ in 0..64 {
        let before = ids.len();
        for n in doc
            .descendants()
            .filter(|n| {
                n.has_tag_name("filter") && n.attribute("id").is_some_and(|id| ids.contains(id))
            })
            .collect::<Vec<_>>()
        {
            if let Some(id) = n
                .attribute("href")
                .or_else(|| n.attribute(("http://www.w3.org/1999/xlink", "href")))
                .and_then(|h| h.strip_prefix('#'))
            {
                ids.insert(id.to_string());
            }
        }
        if ids.len() == before {
            break;
        }
    }
    ids
}
