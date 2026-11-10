use printcaddy_core::{
    document_from_obj, document_from_step, document_to_obj, document_to_step, export_3mf,
    import_3mf, import_dxf, import_svg, Document, ExchangeError, DEFAULT_MODEL_COLOR,
};

fn colored_box() -> Document {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_color(id, printcaddy_core::Color::from_hex(0x336699));
    doc
}

fn world_bounds(doc: &Document) -> (glam::Vec3, glam::Vec3) {
    doc.objects()[0].world_mesh().bounds()
}

fn close(a: glam::Vec3, b: glam::Vec3) {
    assert!((a - b).length() < 0.05, "{a:?} vs {b:?}");
}

#[test]
fn three_mf_round_trip_keeps_the_solid_and_its_colour() {
    let doc = colored_box();
    let (min, max) = world_bounds(&doc);
    let bytes = export_3mf(&doc, false).unwrap();
    let mut loaded = Document::new();
    import_3mf(&mut loaded, &bytes).unwrap();
    assert_eq!(loaded.objects().len(), 1);
    assert_eq!(loaded.objects()[0].color.to_hex(), 0x336699);
    let (lo, hi) = world_bounds(&loaded);
    close(lo, min);
    close(hi, max);
}

#[test]
fn three_mf_garbage_is_an_error() {
    let mut doc = Document::new();
    assert_eq!(
        import_3mf(&mut doc, b"not a 3mf").unwrap_err(),
        ExchangeError::Malformed
    );
    assert!(doc.objects().is_empty());
}

#[test]
fn obj_round_trip_keeps_the_solid_and_its_colour() {
    let doc = colored_box();
    let (min, max) = world_bounds(&doc);
    let text = document_to_obj(&doc, false);
    let mut loaded = Document::new();
    document_from_obj(&mut loaded, text.as_bytes()).unwrap();
    assert_eq!(loaded.objects()[0].color.to_hex(), 0x336699);
    let (lo, hi) = world_bounds(&loaded);
    close(lo, min);
    close(hi, max);
}

#[test]
fn obj_without_faces_is_an_error() {
    let mut doc = Document::new();
    assert_eq!(
        document_from_obj(&mut doc, b"# Z-up, millimetres\nv 0 0 0\n").unwrap_err(),
        ExchangeError::Malformed
    );
}

#[test]
fn an_svg_outline_becomes_a_sketch_that_can_be_built() {
    let svg = br#"<svg><polygon points="0,0 12,0 12,8 0,8"/></svg>"#;
    let mut doc = Document::new();
    import_svg(&mut doc, svg).unwrap();
    let sketch = doc.sketch(doc.objects()[0].id).unwrap();
    assert_eq!(sketch.profile.len(), 4);
    assert!(printcaddy_core::is_closed(&doc.objects()[0].local_mesh()));
    let (min, max) = doc.objects()[0].local_mesh().bounds();
    assert!((max.x - min.x - 12.0).abs() < 0.05);
    assert!((max.z - min.z - 8.0).abs() < 0.05);
    assert!((max.y - min.y - 10.0).abs() < 0.05);
}

#[test]
fn an_svg_with_no_outline_is_an_error() {
    let mut doc = Document::new();
    assert_eq!(
        import_svg(&mut doc, b"<svg></svg>").unwrap_err(),
        ExchangeError::Malformed
    );
}

#[test]
fn a_dxf_inch_square_becomes_a_millimetre_sketch() {
    let dxf = b"0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n1\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n70\n1\n10\n0\n20\n0\n10\n1\n20\n0\n10\n1\n20\n1\n10\n0\n20\n1\n0\nENDSEC\n0\nEOF\n";
    let mut doc = Document::new();
    import_dxf(&mut doc, dxf).unwrap();
    let (min, max) = doc.objects()[0].local_mesh().bounds();
    assert!((max.x - min.x - 25.4).abs() < 0.1, "{} {}", min.x, max.x);
    assert!((max.z - min.z - 25.4).abs() < 0.1);
}

#[test]
fn a_dxf_that_is_not_a_drawing_is_an_error() {
    let mut doc = Document::new();
    assert_eq!(
        import_dxf(&mut doc, b"hello").unwrap_err(),
        ExchangeError::Malformed
    );
}

#[test]
fn step_round_trip_keeps_the_solid() {
    let doc = colored_box();
    let (min, max) = world_bounds(&doc);
    let text = document_to_step(&doc, false).unwrap();
    let mut loaded = Document::new();
    document_from_step(&mut loaded, text.as_bytes()).unwrap();
    assert_eq!(loaded.objects()[0].color, DEFAULT_MODEL_COLOR);
    let (lo, hi) = world_bounds(&loaded);
    close(lo, min);
    close(hi, max);
}

#[test]
fn analytic_step_is_rejected() {
    let step = b"ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1=B_SPLINE_SURFACE('',1,1,(),(),.U.);\nENDSEC;\nEND-ISO-10303-21;\n";
    let mut doc = Document::new();
    assert_eq!(
        document_from_step(&mut doc, step).unwrap_err(),
        ExchangeError::UnsupportedStep
    );
}

#[test]
fn oversized_files_are_refused() {
    let big = vec![0u8; 32 * 1024 * 1024 + 1];
    let mut doc = Document::new();
    assert_eq!(
        import_3mf(&mut doc, &big).unwrap_err(),
        ExchangeError::TooLarge
    );
    assert_eq!(
        document_from_obj(&mut doc, &big).unwrap_err(),
        ExchangeError::TooLarge
    );
    assert_eq!(
        import_svg(&mut doc, &big).unwrap_err(),
        ExchangeError::TooLarge
    );
    assert_eq!(
        import_dxf(&mut doc, &big).unwrap_err(),
        ExchangeError::TooLarge
    );
    assert_eq!(
        document_from_step(&mut doc, &big).unwrap_err(),
        ExchangeError::TooLarge
    );
}
