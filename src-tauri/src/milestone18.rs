//! Independent SVG corpus for project v18 and PDF fallback; Poppler is checked separately.
use crate::{document::{Editor,Shape,Content},io,operations,vector,vector_export};
use serde_json::json;
#[test]
fn genuine_v17_project_migrates_without_changing_pixels_or_locks(){
    let legacy=include_bytes!("fixtures/experience-v17.prisme");
    let doc=io::read_project(legacy).unwrap();let pixels=vector::flatten(&doc).unwrap();
    let current=io::project_bytes(&doc).unwrap();let next=io::read_project(&current).unwrap();
    assert_eq!(pixels,vector::flatten(&next).unwrap());
    assert_eq!(doc.layers.len(),next.layers.len());
    for (a,b) in doc.layers.iter().zip(&next.layers){assert_eq!(a.name,b.name);assert_eq!(a.locks,b.locks);assert_eq!(a.opacity,b.opacity);assert_eq!(a.x,b.x);assert_eq!(a.y,b.y);}
    let mut archive=zip::ZipArchive::new(std::io::Cursor::new(current)).unwrap();
    let project:serde_json::Value=serde_json::from_reader(archive.by_name("document.json").unwrap()).unwrap();
    assert_eq!(project["version"],io::PROJECT_VERSION);
}
#[test]
fn complex_resources_export_roundtrip_and_preserve_native_pixels(){
    use base64::Engine;
    let png=base64::engine::general_purpose::STANDARD.encode(io::png_bytes(&image::RgbaImage::from_fn(4,4,|x,y|image::Rgba([(x*60)as u8,(y*60)as u8,180,180]))).unwrap());
    let samples=[
      ("repeat",r##"<defs><linearGradient id="p" x2=".2" spreadMethod="repeat"><stop stop-color="#fa1243" stop-opacity=".3"/><stop offset="1" stop-color="#237dee"/></linearGradient></defs><rect x="10" y="8" width="220" height="160" fill="url(#p)"/>"##),
      ("reflect",r##"<defs><linearGradient id="p" x2=".25" spreadMethod="reflect" gradientTransform="rotate(12 .5 .5)"><stop stop-color="#fa1243"/><stop offset="1" stop-color="#237dee"/></linearGradient></defs><rect x="10" y="8" width="220" height="160" fill="url(#p)"/>"##),
      ("focal",r##"<defs><radialGradient id="p" fr=".18" fx=".3" fy=".4"><stop stop-color="#ffee42"/><stop offset="1" stop-color="#237dee" stop-opacity=".7"/></radialGradient></defs><rect x="10" y="8" width="220" height="160" fill="url(#p)"/>"##),
      ("additive",r##"<rect x="10" y="10" width="140" height="140" fill="#cc3388"/><g style="mix-blend-mode:plus-lighter"><rect x="70" y="40" width="150" height="120" fill="#3388cc" fill-opacity=".8"/></g>"##),
      ("nested-pattern",r##"<defs><pattern id="q" width="6" height="8" patternUnits="userSpaceOnUse"><path d="M0 0L5 0L0 7Z" fill="#3344aa"/></pattern><pattern id="p" width="24" height="24" patternUnits="userSpaceOnUse"><rect width="24" height="24" fill="#ffcc55"/><g opacity=".7" transform="rotate(8 12 12)"><rect width="18" height="22" fill="url(#q)"/></g></pattern></defs><rect x="10" y="8" width="220" height="160" fill="url(#p)"/>"##),
      ("media-pattern",r##"<defs><clipPath id="c"><circle cx="12" cy="12" r="11"/></clipPath><mask id="m"><rect width="24" height="24" fill="white"/><circle cx="12" cy="12" r="5" fill="#444"/></mask><filter id="f"><feGaussianBlur stdDeviation=".3"/></filter><pattern id="p" width="24" height="24" patternUnits="userSpaceOnUse"><g clip-path="url(#c)" mask="url(#m)"><rect width="24" height="24" fill="#aa2255"/><image x="2" y="3" width="18" height="17" filter="url(#f)" href="data:image/png;base64,__PNG__"/></g></pattern></defs><rect x="10" y="8" width="220" height="160" fill="url(#p)"/>"##),
    ];
    let out=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../work/v18-pdf");std::fs::create_dir_all(&out).unwrap();
    for (name,body) in samples {
      let source=format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="180">{}</svg>"#,body.replace("__PNG__",&png));
      let block:Shape=serde_json::from_value(json!({"id":1,"type":"svg","source":source,"intrinsicW":240,"intrinsicH":180,"x":0,"y":0,"w":240,"h":180,"rotation":0,"fill":null,"stroke":null,"strokeWidth":0,"opacity":1})).unwrap();
      let mut editor=Editor::default();operations::execute(&mut editor,serde_json::from_value(json!({"type":"new","width":240,"height":180,"background":null})).unwrap()).unwrap();
      let decomposed=crate::svg_edit::decompose(&block).unwrap();
      let original=vector::rasterize(&[block],240,180).unwrap();
      let layer=editor.new_vector_layer(name,decomposed);editor.doc_mut().unwrap().layers.push(layer);
      let before=vector::flatten(editor.doc().unwrap()).unwrap();
      let delta=original.pixels().zip(before.pixels()).flat_map(|(p,q)|(0..4).map(move|c|if c==3 {p[c].abs_diff(q[c]) as f64}else{(p[c]as f64*p[3]as f64/255.-q[c]as f64*q[3]as f64/255.).abs()})).sum::<f64>()/(240.*180.*4.);
      assert!(delta<0.15,"{name}: {delta}");
      let archive=io::project_bytes(editor.doc().unwrap()).unwrap();let reopened=io::read_project(&archive).unwrap();assert_eq!(vector::flatten(&reopened).unwrap(),before);
      assert!(matches!(&reopened.layers[1].content,Content::Vector(shapes) if !shapes.is_empty()));
      let pdf=vector_export::pdf(editor.doc().unwrap()).unwrap();assert!(pdf.starts_with(b"%PDF-"));
      assert_eq!(archive,io::project_bytes(editor.doc().unwrap()).unwrap());assert_eq!(vector::flatten(editor.doc().unwrap()).unwrap(),before);
      std::fs::write(out.join(format!("{name}.pdf")),pdf).unwrap();std::fs::write(out.join(format!("{name}.png")),io::png_bytes(&before).unwrap()).unwrap();std::fs::write(out.join(format!("{name}.svg")),source).unwrap();std::fs::write(out.join(format!("{name}.prisme")),archive).unwrap();
    }
}
