use glam::Vec3;
use printcaddy_core::{snap_mm, Document, History, Shape, DEFAULT_SNAP_MM};

#[test]
fn simple_shapes_use_the_library_starting_sizes() {
    let mut doc = Document::new();
    let expected = [
        (Shape::RoundedBox, (40.0, 30.0, 20.0)),
        (Shape::Ellipse, (26.0, 16.0, 20.0)),
        (Shape::Sphere, (22.0, 22.0, 22.0)),
        (Shape::HalfSphere, (22.0, 22.0, 11.0)),
        (Shape::RoundRoof, (20.0, 20.0, 10.0)),
        (Shape::Paraboloid, (22.0, 22.0, 20.0)),
        (Shape::Torus, (22.0, 22.0, 5.0)),
        (Shape::Tube, (20.0, 20.0, 20.0)),
        (Shape::Capsule, (40.0, 20.0, 20.0)),
        (Shape::Star, (40.0, 40.0, 10.0)),
        (Shape::Heart, (40.0, 40.0, 10.0)),
        (Shape::Cone, (20.0, 20.0, 20.0)),
        (Shape::Pyramid, (20.0, 20.0, 20.0)),
        (Shape::Wedge, (20.0, 20.0, 20.0)),
    ];
    for (shape, size) in expected {
        assert_eq!(shape.default_size(), size, "{}", shape.label());
        let id = doc.add_shape(shape);
        let object = doc.object(id).unwrap();
        assert_eq!(object.size(), size);
        assert_eq!(object.color.hex_string(), "#8A98A6");
        assert!(
            object.local_mesh().triangle_count() >= 4,
            "{}",
            shape.label()
        );
        let (min, max) = object.local_mesh().bounds();
        assert!(
            (max.x - min.x - size.0).abs() < 0.6,
            "{} width",
            shape.label()
        );
        assert!(
            (max.z - min.z - size.1).abs() < 0.6,
            "{} depth",
            shape.label()
        );
        assert!(
            (max.y - min.y - size.2).abs() < 0.6,
            "{} height",
            shape.label()
        );
    }
    let (w, d, h) = Shape::Polygon.default_size();
    assert!((w - 17.320508).abs() < 1e-3, "hexagon width {w}");
    assert!((d - 20.0).abs() < 1e-3);
    assert!((h - 20.0).abs() < 1e-3);
}

#[test]
fn snap_defaults_to_one_millimetre_and_zero_is_off() {
    assert_eq!(DEFAULT_SNAP_MM, 1.0);
    assert_eq!(snap_mm(10.4, 1.0), 10.0);
    assert_eq!(snap_mm(10.6, 1.0), 11.0);
    assert_eq!(snap_mm(-0.6, 1.0), -1.0);
    assert_eq!(snap_mm(10.4, 0.0), 10.4);
}

#[test]
fn undo_redo_and_typed_rotation_keep_one_step_each() {
    let mut doc = Document::new();
    let mut history = History::default();
    history.begin(&doc);
    let id = doc.add_shape(Shape::Sphere);
    history.end();
    history.undo(&mut doc);
    assert!(doc.objects().is_empty());
    history.redo(&mut doc);
    assert_eq!(doc.objects().len(), 1);

    history.begin(&doc);
    doc.set_position(id, Vec3::new(3.0, 1.5, -2.0));
    doc.set_rotation(id, Vec3::new(10.0, 20.0, 30.0));
    doc.set_size(id, 30.0, 30.0, 30.0);
    history.end();
    let object = doc.object(id).unwrap();
    assert_eq!(object.position, Vec3::new(3.0, 1.5, -2.0));
    assert_eq!(object.rotation, Vec3::new(10.0, 20.0, 30.0));
    assert_eq!(object.size(), (30.0, 30.0, 30.0));
    history.undo(&mut doc);
    let object = doc.object(id).unwrap();
    assert_eq!(object.size(), (22.0, 22.0, 22.0));
    assert_eq!(object.rotation, Vec3::ZERO);
    assert_eq!(object.position.y, 0.0);
}

#[test]
fn a_second_edit_while_open_does_not_add_another_undo_step() {
    let mut doc = Document::new();
    let mut history = History::default();
    let id = doc.add_box();
    history.begin(&doc);
    doc.move_object_to(id, 1.0, 0.0);
    history.begin(&doc);
    doc.move_object_to(id, 4.0, 2.0);
    history.end();
    history.undo(&mut doc);
    assert_eq!(doc.object(id).unwrap().position, Vec3::ZERO);
}
