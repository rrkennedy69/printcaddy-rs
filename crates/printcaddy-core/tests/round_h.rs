use glam::Vec3;
use printcaddy_core::{mesh_volume, AlignAxis, Document, ObjectKind};

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
    let xs: Vec<f32> = [id]
        .into_iter()
        .chain(created)
        .map(|id| doc.object(id).unwrap().position.x)
        .collect();
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
    let inset = mesh
        .positions
        .iter()
        .any(|p| (p[1] - before).abs() < 0.05 && p[0].abs() < 9.0);
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
    assert!(
        volume > 0.0 && volume < full * 0.85,
        "volume {volume} full {full}"
    );
}

#[test]
fn split_pieces_are_closed() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    assert_eq!(doc.split_selection(AlignAxis::X, 0.5), 1);
    for object in doc.objects() {
        assert!(printcaddy_core::is_closed(&object.local_mesh()), "{}", object.name);
    }
}

#[test]
fn chamfer_takes_both_rims_off_a_cylinder() {
    let mut doc = Document::new();
    let id = doc.add_cylinder();
    doc.select(Some(id));
    assert_eq!(doc.chamfer_selection(1.0), 1);
    let mesh = doc.object(id).unwrap().local_mesh();
    assert!(printcaddy_core::is_closed(&mesh));
    let (min, max) = mesh.bounds();
    let top = mesh.positions.iter().filter(|p| (p[1] - max.y).abs() < 0.05).map(|p| p[0].hypot(p[2])).fold(0.0_f32, f32::max);
    assert!(top < 9.2, "top radius {top}");
    assert!((max.y - min.y - 20.0).abs() < 0.2);
}

#[test]
fn split_makes_two_pieces_that_share_the_cut() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    assert_eq!(doc.split_selection(AlignAxis::X, 0.5), 1);
    let names: Vec<_> = doc
        .objects()
        .iter()
        .map(|object| object.name.as_str())
        .collect();
    assert!(names.iter().any(|name| name.ends_with(" A")));
    assert!(names.iter().any(|name| name.ends_with(" B")));
    let volume: f32 = doc
        .objects()
        .iter()
        .map(|object| mesh_volume(&object.local_mesh()).abs())
        .sum();
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
fn cookie_cutter_is_a_closed_wall_around_the_outline() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    assert_eq!(doc.cookie_cutter(1.2, 15.0, 4.0, 2.0, 0.4, true), 1);
    let cutter = doc.objects().iter().find(|object| object.name == "Cookie cutter").unwrap();
    assert!(printcaddy_core::is_closed(&cutter.local_mesh()));
    let (min, max) = cutter.local_mesh().bounds();
    assert!(max.x - min.x > 24.0, "width {}", max.x - min.x);
    assert!((max.y - min.y - 15.0).abs() < 0.2);
    assert!(mesh_volume(&cutter.local_mesh()).abs() > 100.0);
}

#[test]
fn one_hole_subtracts_quickly() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    let started = std::time::Instant::now();
    assert_eq!(doc.hole_pattern(6.0, 40.0, 4.0), 1);
    let elapsed = started.elapsed();
    eprintln!("one hole {:?}", elapsed);
    assert!(elapsed.as_millis() < 2000, "{elapsed:?}");
    assert!(doc.objects().iter().all(|object| !object.hole));
    assert!(printcaddy_core::is_closed(&doc.object(id).unwrap().local_mesh()));
}

#[test]
fn hole_pattern_places_cutters_inside_the_face() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    let full = mesh_volume(&doc.object(id).unwrap().local_mesh()).abs();
    assert_eq!(doc.hole_pattern(6.0, 9.0, 1.0), 1);
    assert!(doc.objects().iter().all(|object| !object.hole));
    assert_eq!(doc.objects().len(), 1);
    let mesh = doc.object(id).unwrap().local_mesh();
    assert!(printcaddy_core::is_closed(&mesh));
    let volume = mesh_volume(&mesh).abs();
    assert!(volume < full * 0.9 && volume > full * 0.4, "volume {volume} full {full}");
}

#[test]
fn push_face_lifts_the_top_of_a_box() {
    let mut doc = Document::new();
    let id = doc.add_box();
    let x = doc.object(id).unwrap().position.x;
    assert!(doc.push_face(id, Vec3::new(x, 20.0, 0.0), Vec3::Y, 4.0));
    let (min, max) = doc.object(id).unwrap().local_mesh().bounds();
    let height = max.y - min.y;
    assert!((height - 24.0).abs() < 0.5, "height {height}");
}

#[test]
fn delete_face_drops_the_clicked_triangles() {
    let mut doc = Document::new();
    let id = doc.add_box();
    let x = doc.object(id).unwrap().position.x;
    let before = doc.object(id).unwrap().local_mesh().triangle_count();
    assert!(doc.delete_face(id, Vec3::new(x, 20.0, 0.0), Vec3::Y));
    let after = doc.object(id).unwrap().local_mesh().triangle_count();
    assert!(after < before, "before {before} after {after}");
}

#[test]
fn simplify_keeps_fewer_triangles() {
    let mut doc = Document::new();
    let id = doc.add_cylinder();
    doc.select(Some(id));
    let before = doc.object(id).unwrap().local_mesh().triangle_count();
    assert_eq!(doc.simplify_selection(0.4), 1);
    let after = doc.object(id).unwrap().local_mesh().triangle_count();
    assert!(after < before, "before {before} after {after}");
}

#[test]
fn split_off_centre_makes_the_low_piece_smaller() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.select(Some(id));
    assert_eq!(doc.split_selection(AlignAxis::X, 0.25), 1);
    let width = |name: &str| {
        let object = doc
            .objects()
            .iter()
            .find(|object| object.name.ends_with(name))
            .unwrap();
        let (min, max) = object.local_mesh().bounds();
        max.x - min.x
    };
    assert!((width(" A") - 5.0).abs() < 0.2, "low {}", width(" A"));
    assert!((width(" B") - 15.0).abs() < 0.2, "high {}", width(" B"));
}

#[test]
fn circle_pattern_groups_more_than_five() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_position(id, Vec3::new(40.0, 0.0, 0.0));
    doc.select(Some(id));
    let created = doc.pattern_circle(6, 360.0, Vec3::ZERO, true);
    assert_eq!(created.len(), 5);
    assert!(doc
        .objects()
        .iter()
        .any(|object| matches!(object.kind, ObjectKind::Group { .. })));
    assert!(doc.object(id).unwrap().parent.is_some());
}

#[test]
fn hollow_works_on_a_cylinder() {
    let mut doc = Document::new();
    let id = doc.add_cylinder();
    doc.select(Some(id));
    let full = mesh_volume(&doc.object(id).unwrap().local_mesh()).abs();
    assert_eq!(doc.hollow_selection(2.0), 1);
    let volume = mesh_volume(&doc.object(id).unwrap().local_mesh()).abs();
    assert!(
        volume > 0.0 && volume < full * 0.85,
        "volume {volume} full {full}"
    );
}
