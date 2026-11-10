use printcaddy_core::{
    bambu_material, export_bambu_3mf, extract_loaded_filaments, import_bambu_3mf, Document,
    FilamentRef, DEFAULT_BAMBU_MATERIAL, DEFAULT_MODEL_COLOR, PLATES_PER_ROW, PLATE_GAP_MM,
    PLATE_SIZE_MM,
};

fn pitch() -> f32 {
    PLATE_SIZE_MM + PLATE_GAP_MM
}

#[test]
fn plates_sit_in_rows_of_three() {
    assert_eq!(PLATES_PER_ROW, 3);
    assert_eq!(PLATE_GAP_MM, 64.0);
    let mut doc = Document::new();
    assert_eq!(doc.plate_layouts().len(), 1);
    assert_eq!(doc.plate_layouts()[0].center_x, 0.0);
    assert_eq!(doc.plate_layouts()[0].center_z, 0.0);
    doc.add_plate();
    doc.add_plate();
    doc.add_plate();
    let plates = doc.plate_layouts();
    assert_eq!(plates.len(), 4);
    assert!((plates[1].center_x - pitch()).abs() < 1e-3);
    assert!(plates[1].center_z.abs() < 1e-3);
    assert!(plates[3].center_x.abs() < 1e-3);
    assert!((plates[3].center_z - pitch()).abs() < 1e-3);
    assert_eq!(plates[3].number, 4);
}

#[test]
fn deleting_a_plate_slides_the_later_ones_over() {
    let mut doc = Document::new();
    doc.add_plate();
    doc.add_plate();
    let kept = doc.add_shape(printcaddy_core::Shape::Box);
    doc.set_position(kept, glam::Vec3::new(pitch(), 0.0, 0.0));
    let moved = doc.add_shape(printcaddy_core::Shape::Box);
    doc.set_position(moved, glam::Vec3::new(pitch() * 2.0, 0.0, 0.0));
    doc.delete_plate(1).unwrap();
    assert_eq!(doc.plate_layouts().len(), 2);
    let kept_at = doc.object(kept).unwrap().position;
    assert!((kept_at.x - pitch() * 2.0).abs() < 1e-2, "{kept_at:?}");
    let moved_at = doc.object(moved).unwrap().position;
    assert!((moved_at.x - pitch()).abs() < 1e-2, "{moved_at:?}");
    assert!(doc.delete_plate(0).is_err() || doc.plate_layouts().len() >= 1);
    let mut only = Document::new();
    assert!(only.delete_plate(0).is_err());
}

#[test]
fn a_locked_plate_locks_the_bodies_on_it() {
    let mut doc = Document::new();
    let id = doc.add_shape(printcaddy_core::Shape::Box);
    doc.lock_plate(0, true);
    assert!(doc.object(id).unwrap().locked);
    assert!(doc
        .hit_test(printcaddy_core::Ray {
            origin: glam::Vec3::new(0.0, 40.0, 0.0),
            direction: glam::Vec3::new(0.0, -1.0, 0.0),
        })
        .is_none());
    doc.lock_plate(0, false);
    assert!(!doc.object(id).unwrap().locked);
}

#[test]
fn pla_basic_is_the_default_bambu_material() {
    let material = bambu_material(DEFAULT_BAMBU_MATERIAL);
    assert_eq!(material.id, "GFA00");
    assert_eq!(material.name, "PLA Basic");
    assert_eq!(material.colors[0].name, "Orange");
    assert_eq!(material.colors[0].hex.to_ascii_uppercase(), "#FF6A13");
    assert!(printcaddy_core::bambu_materials().len() >= 49);
}

#[test]
fn a_filament_preset_json_lists_the_loaded_colours() {
    let json = br##"{"type":"filament","name":"Bambu PLA Basic @BBL X1C","filament_id":"GFA00","filament_type":"PLA","filament_colour":"#FF6A13"}"##;
    let loaded = extract_loaded_filaments(json, "pla.json").unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].color.to_ascii_uppercase(), "#FF6A13");
    assert_eq!(loaded[0].material_id.as_deref(), Some("GFA00"));
    assert_eq!(loaded[0].name, "PLA Basic");
}

#[test]
fn a_project_3mf_lists_every_loaded_slot() {
    let settings = r##"{"filament_colour":["#FF6A13FF","#000000"],"filament_type":["PLA","PETG"],"filament_settings_id":["Bambu PLA Basic @BBL X1C","Bambu PETG HF @BBL X1C"],"filament_ids":["GFA00","GFG02"]}"##;
    let zip = mini_zip(&[("Metadata/project_settings.config", settings.as_bytes())]);
    let loaded = extract_loaded_filaments(&zip, "printer.3mf").unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].slot, 1);
    assert_eq!(loaded[1].color, "#000000");
    assert_eq!(loaded[1].material_id.as_deref(), Some("GFG02"));
}

#[test]
fn a_bad_bambu_file_is_an_error() {
    let err = extract_loaded_filaments(b"hello", "notes.txt").unwrap_err();
    let text = err.to_string();
    assert!(!text.is_empty());
    assert!(!text.to_ascii_lowercase().contains("panic"));
    let zip = mini_zip(&[("3D/3dmodel.model", b"<model/>")]);
    let err = extract_loaded_filaments(&zip, "plain.3mf").unwrap_err();
    assert!(err.to_string().contains("project_settings"));
}

#[test]
fn bambu_3mf_round_trip_keeps_filament_colour() {
    let mut doc = Document::new();
    let orange = doc.add_shape(printcaddy_core::Shape::Box);
    doc.set_color(orange, printcaddy_core::Color::from_hex(0xFF6A13));
    doc.set_filament(
        orange,
        FilamentRef {
            material: "GFA00".into(),
            name: "Orange".into(),
            code: "10300".into(),
        },
    );
    let black = doc.add_shape(printcaddy_core::Shape::Box);
    doc.set_position(black, glam::Vec3::new(40.0, 0.0, 0.0));
    doc.set_color(black, printcaddy_core::Color::from_hex(0x000000));
    doc.set_filament(
        black,
        FilamentRef {
            material: "GFG02".into(),
            name: "Black".into(),
            code: "33102".into(),
        },
    );
    let bytes = export_bambu_3mf(&doc, false).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("BambuStudio-"));
    assert!(text.contains("filament_colour"));
    let mut back = Document::new();
    import_bambu_3mf(&mut back, &bytes).unwrap();
    assert_eq!(back.objects().len(), 2);
    let colours: Vec<_> = back
        .objects()
        .iter()
        .map(|object| object.color.hex_string().to_ascii_uppercase())
        .collect();
    assert!(colours.contains(&"#FF6A13".to_string()), "{colours:?}");
    assert!(colours.contains(&"#000000".to_string()), "{colours:?}");
    let materials: Vec<_> = back
        .objects()
        .iter()
        .filter_map(|object| object.filament.as_ref().map(|f| f.material.clone()))
        .collect();
    assert!(materials.contains(&"GFA00".to_string()), "{materials:?}");
    assert!(materials.contains(&"GFG02".to_string()), "{materials:?}");
    let first = &back.objects()[0];
    assert!((first.size().0 - 20.0).abs() < 0.2, "{:?}", first.size());
}

#[test]
fn a_broken_bambu_3mf_is_refused() {
    let err = import_bambu_3mf(&mut Document::new(), b"PK not really").unwrap_err();
    assert!(!err.to_string().is_empty());
    let huge = vec![0u8; 32 * 1024 * 1024 + 1];
    let err = import_bambu_3mf(&mut Document::new(), &huge).unwrap_err();
    assert!(err.to_string().contains("32 MB"));
}

#[test]
fn an_unpainted_body_stays_the_default_colour() {
    let doc = Document::new();
    let id = {
        let mut doc = doc;
        let id = doc.add_shape(printcaddy_core::Shape::Box);
        assert_eq!(doc.object(id).unwrap().color, DEFAULT_MODEL_COLOR);
        assert!(doc.object(id).unwrap().filament.is_none());
        id
    };
    let _ = id;
}

fn mini_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    // A stored zip the production reader already accepts.
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
        !crc
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let name_bytes = name.as_bytes();
        let crc = crc32(data);
        let local_at = out.len() as u32;
        out.extend_from_slice(&0x04034b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);
        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&local_at.to_le_bytes());
        central.extend_from_slice(name_bytes);
    }
    let central_at = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x06054b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&central_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}
