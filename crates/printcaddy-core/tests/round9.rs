use printcaddy_core::{
    compare_builds, document_from_json, document_to_json, Document, ProjectError, SketchDef,
    SketchOp, UpdateReport, DEFAULT_MODEL_COLOR,
};

#[test]
fn a_project_round_trips_bodies_sketches_and_holes() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_position(id, glam::Vec3::new(12.0, 0.0, -4.0));
    doc.set_hole(id, true);
    doc.set_color(id, printcaddy_core::Color::from_hex(0x112233));
    let sketch = doc
        .add_sketch(SketchDef::on_plate(
            SketchOp::Extrude,
            vec![[0.0, 0.0], [8.0, 0.0], [8.0, 6.0], [0.0, 6.0]],
            vec![],
            5.0,
            360.0,
        ))
        .unwrap();
    let json = document_to_json(&doc);
    let mut loaded = document_from_json(&json).unwrap();
    assert_eq!(loaded.objects().len(), 2);
    let body = loaded.object(id).unwrap();
    assert!(body.hole);
    assert_eq!(body.color.to_hex(), 0x112233);
    assert!((body.position.x - 12.0).abs() < 1e-3);
    let kept = loaded.sketch(sketch).unwrap();
    assert_eq!(kept.op, SketchOp::Extrude);
    assert_eq!(kept.profile.len(), 4);
    let again = loaded.add_box();
    assert_ne!(again, id);
    assert_ne!(again, sketch);
    let _ = DEFAULT_MODEL_COLOR;
}

#[test]
fn a_bad_project_file_is_rejected() {
    assert_eq!(
        document_from_json("nope").unwrap_err(),
        ProjectError::BadJson
    );
    let foreign = r#"{"format":"other","version":1,"objects":[],"next_id":1}"#;
    assert_eq!(
        document_from_json(foreign).unwrap_err(),
        ProjectError::NotAProject
    );
    let future = r#"{"format":"printcaddy-project","version":9,"objects":[],"next_id":1}"#;
    assert_eq!(
        document_from_json(future).unwrap_err(),
        ProjectError::UnsupportedVersion
    );
}

#[test]
fn the_update_check_compares_wasm_names() {
    let current = r#"<script>from '/printcaddy-rs/printcaddy-app-aaa_bg.wasm'</script>"#;
    let same = r#"printcaddy-app-aaa_bg.wasm"#;
    let newer = r#"printcaddy-app-bbb_bg.wasm"#;
    assert_eq!(compare_builds(current, same), UpdateReport::Current);
    assert_eq!(
        compare_builds(current, newer),
        UpdateReport::Newer {
            current: "printcaddy-app-aaa_bg.wasm".into(),
            remote: "printcaddy-app-bbb_bg.wasm".into(),
        }
    );
    assert_eq!(compare_builds("no build", same), UpdateReport::Unknown);
    let messy = "printcaddy-app-abc.js'; wasm = 'printcaddy-app-def_bg.wasm'";
    assert_eq!(
        printcaddy_core::build_name(messy).as_deref(),
        Some("printcaddy-app-def_bg.wasm")
    );
}
