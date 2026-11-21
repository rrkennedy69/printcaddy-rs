use glam::Vec3;
use printcaddy_core::{AlignAxis, AlignTarget, Document};

#[test]
fn paste_steps_twelve_millimetres_and_stops_at_the_plate_edge() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_position(id, Vec3::new(40.0, 0.0, 10.0));
    let source = doc.object(id).unwrap().clone();
    let pasted = doc.paste_copies(&[source]);
    let copy = doc.object(pasted[0]).unwrap();
    assert!((copy.position.x - 52.0).abs() < 1e-3);
    assert!((copy.position.z - 22.0).abs() < 1e-3);

    doc.set_position(id, Vec3::new(200.0, 0.0, 200.0));
    let source = doc.object(id).unwrap().clone();
    let again = doc.paste_copies(&[source]);
    let copy = doc.object(again[0]).unwrap();
    assert!((copy.position.x - 110.0).abs() < 1e-3);
    assert!((copy.position.z - 110.0).abs() < 1e-3);
}

#[test]
fn align_matches_the_anchor_on_one_axis() {
    let mut doc = Document::new();
    let left = doc.add_box();
    let right = doc.add_box();
    doc.set_position(left, Vec3::new(-40.0, 0.0, 0.0));
    doc.set_position(right, Vec3::new(30.0, 0.0, 15.0));
    doc.select(Some(left));
    doc.toggle_select(right);
    assert_eq!(doc.align_selection(AlignAxis::X, AlignTarget::Min), 1);
    let left_min = doc.object(left).unwrap().world_mesh().bounds().0.x;
    let right_min = doc.object(right).unwrap().world_mesh().bounds().0.x;
    assert!((left_min - right_min).abs() < 1e-2);
    assert!((doc.object(right).unwrap().position.z - 15.0).abs() < 1e-3);
}

#[test]
fn center_puts_the_footprint_on_the_plate_centre() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_position(id, Vec3::new(40.0, 5.0, -20.0));
    doc.select(Some(id));
    assert!(doc.center_selection());
    let (min, max) = doc.object(id).unwrap().world_mesh().bounds();
    assert!((min.x + max.x).abs() < 1e-2);
    assert!((min.z + max.z).abs() < 1e-2);
    assert!((doc.object(id).unwrap().position.y - 5.0).abs() < 1e-3);
}

#[test]
fn drop_rests_on_the_plate_or_the_solid_below() {
    let mut doc = Document::new();
    let floor = doc.add_box();
    let above = doc.add_box();
    doc.set_position(floor, Vec3::ZERO);
    doc.set_position(above, Vec3::new(0.0, 30.0, 0.0));
    doc.select(Some(above));
    assert!(doc.drop_selection());
    let top = doc.object(floor).unwrap().world_mesh().bounds().1.y;
    let bottom = doc.object(above).unwrap().world_mesh().bounds().0.y;
    assert!((bottom - top).abs() < 1e-2);

    let alone = doc.add_box();
    doc.set_position(alone, Vec3::new(80.0, 12.0, 0.0));
    doc.select(Some(alone));
    assert!(doc.drop_selection());
    assert!(doc.object(alone).unwrap().world_mesh().bounds().0.y.abs() < 1e-2);
}

#[test]
fn lay_flat_turns_a_side_down_and_drops_it() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_rotation(id, Vec3::new(90.0, 0.0, 0.0));
    doc.set_position(id, Vec3::new(0.0, 40.0, 0.0));
    doc.select(Some(id));
    assert!(doc.lay_flat(id, Vec3::Y));
    let (min, max) = doc.object(id).unwrap().world_mesh().bounds();
    assert!(min.y.abs() < 0.2, "bottom {min:?} top {max:?}");
    assert!(max.y - min.y > 10.0);
}
