use glam::Vec3;
use printcaddy_core::{
    document_to_stl, import_stl, mesh_from_stl, mesh_to_stl_binary, Document, ObjectKind, Ray,
    CYLINDER_HEIGHT, CYLINDER_RADIUS, CYLINDER_SEGMENTS, DEFAULT_BOX, DEFAULT_MODEL_COLOR,
    GRID_MAJOR_MM, GRID_MINOR_MM, PLATE_SIZE_MM,
};

#[test]
fn new_bodies_use_the_slate_default_and_sit_on_the_plate() {
    assert_eq!(DEFAULT_MODEL_COLOR.hex_string(), "#8A98A6");
    assert_eq!(PLATE_SIZE_MM, 256.0);
    assert_eq!(GRID_MINOR_MM, 10.0);
    assert_eq!(GRID_MAJOR_MM, 50.0);

    let mut doc = Document::new();
    let box_id = doc.add_box();
    let cyl_id = doc.add_cylinder();

    let box_obj = doc.object(box_id).unwrap();
    assert_eq!(box_obj.name, "Box 1");
    assert_eq!(box_obj.color, DEFAULT_MODEL_COLOR);
    assert_eq!(box_obj.position, Vec3::ZERO);
    match box_obj.kind {
        ObjectKind::Box {
            width,
            depth,
            height,
        } => assert_eq!((width, depth, height), DEFAULT_BOX),
        _ => panic!("expected a box"),
    }
    assert_eq!(box_obj.local_mesh().triangle_count(), 12);

    let cyl = doc.object(cyl_id).unwrap();
    assert_eq!(cyl.name, "Cylinder 1");
    assert_eq!(cyl.color.hex_string(), "#8A98A6");
    assert_eq!(cyl.position, Vec3::new(40.0, 0.0, 0.0));
    match cyl.kind {
        ObjectKind::Cylinder {
            radius,
            height,
            segments,
        } => {
            assert_eq!(
                (radius, height, segments),
                (CYLINDER_RADIUS, CYLINDER_HEIGHT, CYLINDER_SEGMENTS)
            );
        }
        _ => panic!("expected a cylinder"),
    }
    assert_eq!(
        cyl.local_mesh().triangle_count(),
        CYLINDER_SEGMENTS as usize * 4
    );
    // Adding a body selects it.
    assert_eq!(doc.selection(), Some(cyl_id));
}

#[test]
fn move_keeps_the_body_on_the_plate_and_selection_hits_it() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.move_object_to(id, 15.0, -8.0);
    let object = doc.object(id).unwrap();
    assert_eq!(object.position, Vec3::new(15.0, 0.0, -8.0));

    let hit = Ray::new(Vec3::new(15.0, 80.0, -8.0), Vec3::new(0.0, -1.0, 0.0));
    assert_eq!(doc.hit_test(hit), Some(id));

    let miss = Ray::new(Vec3::new(80.0, 80.0, 80.0), Vec3::new(0.0, -1.0, 0.0));
    assert_eq!(doc.hit_test(miss), None);

    doc.select(None);
    assert_eq!(doc.selection(), None);
    doc.select(Some(id));
    assert_eq!(doc.selection(), Some(id));
}

#[test]
fn binary_and_ascii_stl_round_trip_and_import_seats_on_the_plate() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.move_object_to(id, 12.0, -4.0);
    let bytes = document_to_stl(&doc, true);
    let mesh = mesh_from_stl(&bytes).unwrap();
    assert_eq!(mesh.triangle_count(), 12);
    let (min, max) = mesh.bounds();
    assert!((min.x - 2.0).abs() < 1e-3, "min.x {}", min.x);
    assert!((max.x - 22.0).abs() < 1e-3, "max.x {}", max.x);
    assert!((min.y).abs() < 1e-3);
    assert!((max.y - 20.0).abs() < 1e-3);
    assert!((min.z - (-14.0)).abs() < 1e-3, "min.z {}", min.z);

    let ascii = b"solid demo
  facet normal 0 0 1
    outer loop
      vertex 10 5 0
      vertex 20 5 0
      vertex 20 15 4
    endloop
  endfacet
endsolid demo
";
    let mut doc = Document::new();
    let imported = import_stl(&mut doc, "demo.stl", ascii).unwrap();
    let object = doc.object(imported).unwrap();
    assert_eq!(object.name, "demo.stl");
    assert_eq!(object.color.hex_string(), "#8A98A6");
    assert_eq!(object.position, Vec3::ZERO);
    let (min, max) = object.local_mesh().bounds();
    assert!(
        min.y.abs() < 1e-4,
        "bottom should sit on the plate, got {}",
        min.y
    );
    assert!(((min.x + max.x) * 0.5).abs() < 1e-3);
    assert!(((min.z + max.z) * 0.5).abs() < 1e-3);
    assert_eq!(object.local_mesh().triangle_count(), 1);

    let again = mesh_from_stl(&mesh_to_stl_binary(&object.local_mesh())).unwrap();
    assert_eq!(again.triangle_count(), 1);
}

#[test]
fn a_vertical_ray_hits_the_cylinder_wall_and_misses_the_square_corner() {
    let mut doc = Document::new();
    let id = doc.add_cylinder();
    let object = doc.object(id).unwrap();
    let center = object.position;
    let on_wall = Ray::new(
        center + Vec3::new(CYLINDER_RADIUS - 0.5, 40.0, 0.0),
        Vec3::NEG_Y,
    );
    assert_eq!(doc.hit_test(on_wall), Some(id));
    // Outside the radius but inside the bounding square.
    let corner = Ray::new(
        center + Vec3::new(CYLINDER_RADIUS + 2.0, 40.0, CYLINDER_RADIUS + 2.0),
        Vec3::NEG_Y,
    );
    assert_eq!(doc.hit_test(corner), None);
}
