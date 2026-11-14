use printcaddy_core::{
    apply_preset, approximate_layer, material_temps, parse_gcode, slice_plate, standard_settings,
    Document, PresetId, Shape, SliceError,
};

#[test]
fn presets_match_the_typescript_page() {
    let draft = apply_preset(standard_settings(), PresetId::Draft);
    assert!((draft.layer_height - 0.28).abs() < 1e-6);
    assert_eq!(draft.walls, 2);
    assert!((draft.infill - 10.0).abs() < 1e-6);
    let fine = apply_preset(standard_settings(), PresetId::Fine);
    assert!((fine.layer_height - 0.12).abs() < 1e-6);
    assert_eq!(fine.walls, 3);
    let standard = standard_settings();
    assert!((standard.layer_height - 0.2).abs() < 1e-6);
    assert_eq!(standard.walls, 2);
    assert!((standard.infill - 15.0).abs() < 1e-6);
    assert_eq!(material_temps(Some("PETG")).nozzle, 250);
    assert_eq!(material_temps(None).kind, "PLA");
}

#[test]
fn a_box_slices_into_closed_layers() {
    let mut doc = Document::new();
    doc.add_shape(Shape::Box);
    let before = doc.objects()[0].revision;
    let sliced = slice_plate(&doc, 1, &standard_settings()).unwrap();
    assert_eq!(doc.objects()[0].revision, before);
    assert_eq!(sliced.layers.len(), 100);
    assert!((sliced.layers[0].z - 0.2).abs() < 1e-3);
    assert!((sliced.layers.last().unwrap().z - 20.0).abs() < 1e-2);
    let outer = sliced.layers[50]
        .paths
        .iter()
        .find(|path| path.kind == printcaddy_core::PathKind::OuterWall)
        .expect("outer wall");
    let (min_x, max_x, min_z, max_z) = bounds(&outer.points);
    assert!((max_x - min_x - 20.0).abs() < 0.6, "{min_x} {max_x}");
    assert!((max_z - min_z - 20.0).abs() < 0.6, "{min_z} {max_z}");
    assert!(sliced.filament_mm > 100.0, "{}", sliced.filament_mm);
    assert!(sliced.print_time_sec > 0.0);
    assert!(sliced.gcode.contains(";LAYER:"));
}

#[test]
fn more_infill_uses_more_filament() {
    let mut doc = Document::new();
    doc.add_shape(Shape::Box);
    let mut sparse = standard_settings();
    sparse.infill = 0.0;
    let mut solid = standard_settings();
    solid.infill = 100.0;
    let a = slice_plate(&doc, 1, &sparse).unwrap().filament_mm;
    let b = slice_plate(&doc, 1, &solid).unwrap().filament_mm;
    assert!(b > a * 1.5, "{a} {b}");
}

#[test]
fn an_empty_plate_and_an_oversized_body_are_errors() {
    let empty = slice_plate(&Document::new(), 1, &standard_settings()).unwrap_err();
    assert!(matches!(empty, SliceError::Empty));
    let mut doc = Document::new();
    let id = doc.add_shape(Shape::Box);
    doc.set_size(id, 20.0, 20.0, 300.0);
    let err = slice_plate(&doc, 1, &standard_settings()).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("256"), "{text}");
}

#[test]
fn bambu_process_keys_update_the_page() {
    let mut settings = standard_settings();
    printcaddy_core::apply_bambu_process(
        &mut settings,
        r#"{"layer_height":"0.16","wall_loops":"4","sparse_infill_density":"40%","nozzle_temperature":"230"}"#,
    )
    .unwrap();
    assert!((settings.layer_height - 0.16).abs() < 1e-6);
    assert_eq!(settings.walls, 4);
    assert!((settings.infill - 40.0).abs() < 1e-6);
    assert_eq!(settings.nozzle_temp, Some(230.0));
    let bad = printcaddy_core::apply_bambu_process(&mut settings, "nope").unwrap_err();
    assert!(!bad.to_string().is_empty());
}

#[test]
fn gcode_layers_parse_like_the_typescript_app() {
    let text = "\
;LAYER:0
;TYPE:WALL-OUTER
G1 X0 Y0 Z0.2
G1 X10 Y0 Z0.2 E1
;TYPE:FILL
G1 X10 Y5 Z0.2 E2
";
    let parsed = parse_gcode(text);
    assert_eq!(parsed.layer_z.len(), 1);
    assert!((parsed.layer_z[0] - 0.2).abs() < 1e-4);
    assert!((parsed.filament_mm - 2.0).abs() < 1e-3);
    assert!(parsed.segments.len() >= 2);
    assert!((parsed.segments[0].0[0] - 0.0).abs() < 1e-4);
    assert!((parsed.segments[0].1[0] - 10.0).abs() < 1e-4);
}

#[test]
fn the_layer_readout_uses_a_fifth_of_a_millimetre() {
    assert_eq!(approximate_layer(10.0), 50);
    assert_eq!(approximate_layer(0.0), 1);
}

fn bounds(points: &[[f32; 2]]) -> (f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    for point in points {
        min_x = min_x.min(point[0]);
        max_x = max_x.max(point[0]);
        min_z = min_z.min(point[1]);
        max_z = max_z.max(point[1]);
    }
    (min_x, max_x, min_z, max_z)
}
