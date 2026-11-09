use glam::Vec3;
use printcaddy_core::{build_sketch, is_closed, Document, SketchDef, SketchError, SketchOp};

fn square() -> Vec<[f32; 2]> {
    vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
}

#[test]
fn extrude_makes_a_closed_solid_of_the_requested_height() {
    let def = SketchDef::on_plate(SketchOp::Extrude, square(), vec![], 10.0, 360.0);
    let mesh = build_sketch(&def).unwrap();
    assert!(is_closed(&mesh));
    assert_eq!(mesh.triangle_count(), 12);
    let (min, max) = mesh.bounds();
    assert!((min.y).abs() < 1e-3);
    assert!((max.y - 10.0).abs() < 1e-3);
    assert!((max.x - min.x - 10.0).abs() < 1e-3);
    assert!((max.z - min.z - 10.0).abs() < 1e-3);
    let top = mesh.normals.iter().filter(|n| n[1] > 0.5).count();
    assert!(top >= 6);
}

#[test]
fn bad_profiles_are_rejected_without_a_mesh() {
    let open = SketchDef::on_plate(
        SketchOp::Extrude,
        vec![[0.0, 0.0], [5.0, 0.0]],
        vec![],
        10.0,
        360.0,
    );
    assert_eq!(
        build_sketch(&open).unwrap_err(),
        SketchError::TooFewProfilePoints
    );
    let flat = SketchDef::on_plate(
        SketchOp::Extrude,
        vec![[0.0, 0.0], [4.0, 0.0], [8.0, 0.0]],
        vec![],
        10.0,
        360.0,
    );
    assert_eq!(build_sketch(&flat).unwrap_err(), SketchError::ZeroArea);
    let bowtie = SketchDef::on_plate(
        SketchOp::Extrude,
        vec![[0.0, 0.0], [6.0, 2.0], [0.0, 2.0], [2.0, 0.0]],
        vec![],
        10.0,
        360.0,
    );
    assert_eq!(
        build_sketch(&bowtie).unwrap_err(),
        SketchError::SelfIntersecting
    );
}

#[test]
fn revolve_spins_a_profile_around_the_plate_axis() {
    let profile = vec![[5.0, -2.0], [8.0, -2.0], [8.0, 2.0], [5.0, 2.0]];
    let def = SketchDef::on_plate(SketchOp::Revolve, profile, vec![], 10.0, 360.0);
    let mesh = build_sketch(&def).unwrap();
    assert!(is_closed(&mesh));
    let (min, max) = mesh.bounds();
    assert!((min.x + 8.0).abs() < 0.2 && (max.x - 8.0).abs() < 0.2);
    assert!((min.y + 8.0).abs() < 0.2 && (max.y - 8.0).abs() < 0.2);
    assert!((min.z + 2.0).abs() < 0.05 && (max.z - 2.0).abs() < 0.05);
    let crossing = SketchDef::on_plate(
        SketchOp::Revolve,
        vec![[-2.0, 0.0], [4.0, 0.0], [4.0, 2.0], [-2.0, 2.0]],
        vec![],
        10.0,
        360.0,
    );
    assert_eq!(
        build_sketch(&crossing).unwrap_err(),
        SketchError::CrossesAxis
    );
}

#[test]
fn sweep_carries_a_profile_along_a_bent_path() {
    let profile = vec![[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
    let path = vec![[0.0, 0.0], [12.0, 0.0], [12.0, 12.0]];
    let def = SketchDef::on_plate(SketchOp::Sweep, profile, path, 10.0, 360.0);
    let mesh = build_sketch(&def).unwrap();
    assert!(is_closed(&mesh));
    let (min, max) = mesh.bounds();
    assert!(max.x - min.x > 12.0);
    assert!(max.z - min.z > 12.0);
    assert!(max.y - min.y > 1.5);
    let short = SketchDef::on_plate(SketchOp::Sweep, square(), vec![[0.0, 0.0]], 10.0, 360.0);
    assert_eq!(
        build_sketch(&short).unwrap_err(),
        SketchError::TooFewPathPoints
    );
}

#[test]
fn baking_a_sketch_keeps_the_profile_and_clears_the_angle() {
    let mut doc = Document::new();
    let def = SketchDef::on_plate(SketchOp::Extrude, square(), vec![], 10.0, 360.0);
    let id = doc.add_sketch(def).unwrap();
    let before = doc.sketch(id).unwrap().profile.len();
    doc.set_rotation(id, Vec3::new(90.0, 0.0, 0.0));
    doc.bake_rotations();
    let object = doc.object(id).unwrap();
    assert_eq!(object.rotation, Vec3::ZERO);
    let kept = doc.sketch(id).expect("profile survives the bake");
    assert_eq!(kept.profile.len(), before);
    assert_eq!(kept.op, SketchOp::Extrude);
    let (min, max) = object.world_mesh().bounds();
    assert!(max.x - min.x > 5.0 && max.y - min.y > 5.0);
}

#[test]
fn a_concave_profile_extrudes_to_a_closed_solid() {
    let profile = vec![
        [0.0, 0.0],
        [6.0, 0.0],
        [6.0, 2.0],
        [2.0, 2.0],
        [2.0, 6.0],
        [0.0, 6.0],
    ];
    let mesh = build_sketch(&SketchDef::on_plate(
        SketchOp::Extrude,
        profile,
        vec![],
        4.0,
        360.0,
    ))
    .unwrap();
    assert!(is_closed(&mesh));
    let (min, max) = mesh.bounds();
    assert!((max.y - 4.0).abs() < 1e-3);
    assert!((max.x - min.x - 6.0).abs() < 1e-3);
}

#[test]
fn a_new_revolve_sits_on_the_plate() {
    let mut doc = Document::new();
    let profile = vec![[20.0, -5.0], [30.0, -5.0], [30.0, 5.0], [20.0, 5.0]];
    let id = doc
        .add_sketch(SketchDef::on_plate(
            SketchOp::Revolve,
            profile,
            vec![],
            10.0,
            360.0,
        ))
        .unwrap();
    let (min, max) = doc.object(id).unwrap().world_mesh().bounds();
    assert!(min.y.abs() < 1e-2);
    assert!(max.y > 10.0);
    assert!(max.x > 20.0 && min.x < -20.0);
}

#[test]
fn editing_a_moved_sketch_keeps_its_place() {
    let mut doc = Document::new();
    let id = doc
        .add_sketch(SketchDef::on_plate(
            SketchOp::Extrude,
            square(),
            vec![],
            10.0,
            360.0,
        ))
        .unwrap();
    doc.set_position(id, Vec3::new(40.0, 0.0, 30.0));
    let mut def = doc.sketch(id).unwrap().clone();
    def.height = 12.0;
    doc.replace_sketch(id, def).unwrap();
    let object = doc.object(id).unwrap();
    assert!((object.position.x - 40.0).abs() < 1.0);
    assert!((object.position.z - 30.0).abs() < 1.0);
    assert!((object.rotation).length() < 1e-4);
    let kept = doc.sketch(id).unwrap();
    assert_eq!(kept.profile.len(), 4);
    assert!((kept.height - 12.0).abs() < 1e-3);
}
