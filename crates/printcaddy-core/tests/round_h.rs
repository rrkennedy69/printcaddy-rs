use glam::Vec3;
use printcaddy_core::{mesh_volume, AlignAxis, Document};

#[test]
fn mirror_flips_a_body_across_the_selection_centre() {
    let mut doc = Document::new();
    let left = doc.add_box();
    let right = doc.add_box();
    doc.set_position(left, Vec3::new(-40.0, 0.0, 0.0));
    doc.set_position(right, Vec3::new(40.0, 0.0, 0.0));
    doc.select(Some(left));
    doc.toggle_select(right);
    assert_eq!(doc.mirror_selection(AlignAxis::X, false), 2);
    assert!(doc.object(left).unwrap().position.x > 0.0);
    assert!(doc.object(right).unwrap().position.x < 0.0);
}

#[test]
fn pattern_repeats_along_x_and_keeps_the_original() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    let created = doc.pattern_row(3, Vec3::new(25.0, 0.0, 0.0));
    assert_eq!(created.len(), 2);
    let xs: Vec<f32> = [id].into_iter().chain(created).map(|id| doc.object(id).unwrap().position.x).collect();
    assert!((xs[1] - xs[0] - 25.0).abs() < 1e-3);
    assert!((xs[2] - xs[0] - 50.0).abs() < 1e-3);
}

#[test]
fn chamfer_lowers_the_top_corners_of_a_box() {
    let mut doc = Document::new();
    let id = doc.add_box();
    let before = doc.object(id).unwrap().local_mesh().bounds().1.y;
    doc.select(Some(id));
    assert_eq!(doc.chamfer_selection(2.0), 1);
    let mesh = doc.object(id).unwrap().local_mesh();
    let top = mesh.bounds().1.y;
    assert!((top - before).abs() < 1e-3);
    let inset = mesh.positions.iter().any(|p| (p[1] - before).abs() < 0.05 && p[0].abs() < 9.0);
    assert!(inset, "the top face should be inset");
    assert!(mesh_volume(&mesh).abs() > 0.0);
}

#[test]
fn hollow_opens_the_top_and_keeps_a_floor() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    assert_eq!(doc.hollow_selection(2.0), 1);
    let mesh = doc.object(id).unwrap().local_mesh();
    let full = {
        let mut plain = Document::new();
        let box_id = plain.add_box();
        mesh_volume(&plain.object(box_id).unwrap().local_mesh())
    };
    let volume = mesh_volume(&mesh).abs();
    let full = full.abs();
    assert!(volume > 0.0 && volume < full * 0.85, "volume {volume} full {full}");
}

#[test]
fn split_makes_two_pieces_that_share_the_cut() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    assert_eq!(doc.split_selection(AlignAxis::X, 0.5), 1);
    let names: Vec<_> = doc.objects().iter().map(|object| object.name.as_str()).collect();
    assert!(names.iter().any(|name| name.ends_with(" A")));
    assert!(names.iter().any(|name| name.ends_with(" B")));
    let volume: f32 = doc.objects().iter().map(|object| mesh_volume(&object.local_mesh()).abs()).sum();
    assert!(volume > 1000.0, "volume {volume}");
}

#[test]
fn mirror_copy_keeps_the_original() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_position(id, Vec3::new(30.0, 0.0, 0.0));
    doc.select(Some(id));
    assert_eq!(doc.mirror_selection(AlignAxis::X, true), 1);
    assert!((doc.object(id).unwrap().position.x - 30.0).abs() < 1e-2);
    assert_eq!(doc.objects().len(), 2);
}

#[test]
fn circle_pattern_turns_a_copy_about_the_origin() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_position(id, Vec3::new(40.0, 0.0, 0.0));
    doc.select(Some(id));
    let created = doc.pattern_circle(4, 360.0, Vec3::ZERO, false);
    assert_eq!(created.len(), 3);
    let z = doc.object(created[0]).unwrap().position.z;
    assert!(z.abs() > 20.0, "first copy z {z}");
}

#[test]
fn offset_grows_a_box() {
    let mut doc = Document::new();
    let id = doc.add_box();
    let before = doc.object(id).unwrap().local_mesh().bounds();
    doc.select(Some(id));
    assert_eq!(doc.offset_selection(1.0), 1);
    let after = doc.object(id).unwrap().local_mesh().bounds();
    let grew = (after.1.x - after.0.x) - (before.1.x - before.0.x);
    assert!(grew > 1.0, "grew {grew}");
}

#[test]
fn hole_pattern_places_cutters_inside_the_face() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    assert_eq!(doc.hole_pattern(6.0, 9.0, 1.0), 1);
    let holes = doc.objects().iter().filter(|object| object.hole).count();
    assert!(holes >= 2, "holes {holes}");
}
