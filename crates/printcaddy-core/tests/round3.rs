use glam::Vec3;
use printcaddy_core::{Document, History, Shape};

#[test]
fn a_committed_rotation_bakes_to_zero_and_keeps_the_turned_shape() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_rotation(id, Vec3::new(0.0, 90.0, 0.0));
    let before = doc.object(id).unwrap().world_mesh().bounds();
    doc.bake_rotations();
    let object = doc.object(id).unwrap();
    assert_eq!(object.rotation, Vec3::ZERO);
    assert!(matches!(
        object.kind,
        printcaddy_core::ObjectKind::Mesh { .. }
    ));
    let after = object.world_mesh().bounds();
    assert!((before.0 - after.0).length() < 0.05);
    assert!((before.1 - after.1).length() < 0.05);
    // A second bake does nothing.
    let revision = object.revision;
    doc.bake_rotations();
    assert_eq!(doc.object(id).unwrap().revision, revision);
}

#[test]
fn undo_of_a_baked_rotation_restores_the_original_body() {
    let mut doc = Document::new();
    let mut history = History::default();
    let id = doc.add_box();
    history.begin(&doc);
    doc.set_rotation(id, Vec3::new(90.0, 0.0, 0.0));
    doc.bake_rotations();
    history.end();
    assert_eq!(doc.object(id).unwrap().rotation, Vec3::ZERO);
    history.undo(&mut doc);
    let object = doc.object(id).unwrap();
    assert_eq!(object.rotation, Vec3::ZERO);
    assert_eq!(object.size(), (20.0, 20.0, 20.0));
    assert!(matches!(
        object.kind,
        printcaddy_core::ObjectKind::Box { .. }
    ));
}

#[test]
fn multi_select_toggle_and_group_move_are_one_translation() {
    let mut doc = Document::new();
    let a = doc.add_box();
    let b = doc.add_shape(Shape::Sphere);
    doc.select(Some(a));
    doc.toggle_select(b);
    assert_eq!(doc.selected_ids(), &[a, b]);
    doc.toggle_select(a);
    assert_eq!(doc.selected_ids(), &[b]);
    doc.toggle_select(a);
    let before_a = doc.object(a).unwrap().position;
    let before_b = doc.object(b).unwrap().position;
    doc.translate_selected(Vec3::new(5.0, 2.0, -3.0));
    assert_eq!(
        doc.object(a).unwrap().position,
        before_a + Vec3::new(5.0, 2.0, -3.0)
    );
    assert_eq!(
        doc.object(b).unwrap().position,
        before_b + Vec3::new(5.0, 2.0, -3.0)
    );
    let (min, max) = doc.selection_bounds().unwrap();
    assert!(max.x > min.x && max.y > min.y);
}

#[test]
fn previously_rotated_bodies_bake_when_the_document_is_adopted() {
    let mut doc = Document::new();
    let id = doc.add_cylinder();
    doc.set_rotation(id, Vec3::new(0.0, 0.0, 45.0));
    doc.bake_rotations();
    let object = doc.object(id).unwrap();
    assert_eq!(object.rotation, Vec3::ZERO);
    assert_eq!(object.color.hex_string(), "#8A98A6");
    assert!(object.local_mesh().triangle_count() > 12);
}
