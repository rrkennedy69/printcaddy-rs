use printcaddy_core::{
    boolean, mesh_volume, BooleanError, BooleanOp, Document, FilletError, Shape,
};

fn box_at(doc: &mut Document, x: f32) -> printcaddy_core::ObjectId {
    let id = doc.add_box();
    doc.set_position(id, glam::Vec3::new(x, 0.0, 0.0));
    id
}

#[test]
fn union_of_overlapping_boxes_is_a_closed_solid() {
    let mut doc = Document::new();
    let a = box_at(&mut doc, 0.0);
    let _b = box_at(&mut doc, 10.0);
    doc.select(Some(a));
    doc.toggle_select(doc.objects().last().unwrap().id);
    doc.boolean_selected(BooleanOp::Union).unwrap();
    assert_eq!(doc.objects().len(), 1);
    let mesh = doc.objects()[0].world_mesh();
    assert!(printcaddy_core::is_closed(&mesh));
    let volume = mesh_volume(&mesh).abs();
    assert!((volume - 12_000.0).abs() < 50.0, "volume {volume}");
}

#[test]
fn subtract_cuts_the_other_body_out_of_the_primary() {
    let mut doc = Document::new();
    let _tool = box_at(&mut doc, 10.0);
    let stock = box_at(&mut doc, 0.0);
    doc.select(Some(doc.objects()[0].id));
    doc.toggle_select(stock);
    doc.boolean_selected(BooleanOp::Subtract).unwrap();
    let mesh = doc.objects()[0].world_mesh();
    assert!(printcaddy_core::is_closed(&mesh));
    let volume = mesh_volume(&mesh).abs();
    assert!((volume - 4_000.0).abs() < 50.0, "volume {volume}");
}

#[test]
fn intersect_keeps_only_the_shared_volume() {
    let mut doc = Document::new();
    box_at(&mut doc, 0.0);
    box_at(&mut doc, 10.0);
    let ids: Vec<_> = doc.objects().iter().map(|o| o.id).collect();
    doc.select(Some(ids[0]));
    doc.toggle_select(ids[1]);
    doc.boolean_selected(BooleanOp::Intersect).unwrap();
    let volume = mesh_volume(&doc.objects()[0].world_mesh()).abs();
    assert!((volume - 4_000.0).abs() < 50.0, "volume {volume}");
}

#[test]
fn disjoint_intersection_is_rejected() {
    let mut doc = Document::new();
    box_at(&mut doc, 0.0);
    box_at(&mut doc, 80.0);
    let ids: Vec<_> = doc.objects().iter().map(|o| o.id).collect();
    doc.select(Some(ids[0]));
    doc.toggle_select(ids[1]);
    let err = doc.boolean_selected(BooleanOp::Intersect).unwrap_err();
    assert_eq!(err, BooleanError::Empty);
    assert_eq!(doc.objects().len(), 2);
}

#[test]
fn an_open_mesh_is_rejected() {
    let mesh = printcaddy_core::Mesh {
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        indices: vec![0, 1, 2],
    };
    let err = boolean(BooleanOp::Union, &mesh, &mesh).unwrap_err();
    assert_eq!(err, BooleanError::NotClosed);
}

#[test]
fn a_cylinder_can_be_cut_out_of_a_box() {
    let stock = printcaddy_core::Shape::Box;
    let _ = stock;
    let box_mesh = {
        let mut doc = Document::new();
        let id = doc.add_box();
        doc.object(id).unwrap().world_mesh()
    };
    let cutter = {
        let mut doc = Document::new();
        let id = doc.add_cylinder();
        doc.set_position(id, glam::Vec3::new(0.0, 0.0, 0.0));
        doc.object(id).unwrap().world_mesh()
    };
    let mesh = boolean(BooleanOp::Subtract, &box_mesh, &cutter).unwrap();
    assert!(printcaddy_core::is_closed(&mesh));
    let volume = mesh_volume(&mesh).abs();
    assert!(volume < 8_000.0 - 100.0, "volume {volume}");
    assert!(volume > 1_000.0, "volume {volume}");
}

#[test]
fn grouping_moves_duplicates_and_ungroups_together() {
    let mut doc = Document::new();
    box_at(&mut doc, 0.0);
    box_at(&mut doc, 30.0);
    let ids: Vec<_> = doc.objects().iter().map(|o| o.id).collect();
    doc.select(Some(ids[0]));
    doc.toggle_select(ids[1]);
    let group = doc.group_selected().unwrap();
    assert_eq!(doc.objects().len(), 3);
    doc.select(Some(group));
    doc.translate_selected(glam::Vec3::new(0.0, 0.0, 15.0));
    for id in &ids {
        assert!((doc.object(*id).unwrap().position.z - 15.0).abs() < 1e-3);
    }
    doc.duplicate(group).unwrap();
    assert_eq!(doc.objects().len(), 6);
    doc.select(Some(group));
    doc.ungroup_selected().unwrap();
    assert!(doc.object(group).is_none());
    assert_eq!(doc.objects().len(), 5);
    assert!(doc.object(ids[0]).unwrap().parent.is_none());
}

#[test]
fn a_grouped_hole_cuts_the_solid_until_ungroup() {
    let mut doc = Document::new();
    let solid = box_at(&mut doc, 0.0);
    let hole = box_at(&mut doc, 10.0);
    doc.set_hole(hole, true);
    assert!(doc.object(hole).unwrap().hole);
    doc.select(Some(solid));
    doc.toggle_select(hole);
    let group = doc.group_selected().unwrap();
    let shown = doc.object(group).unwrap().world_mesh();
    assert!(printcaddy_core::is_closed(&shown));
    let volume = mesh_volume(&shown).abs();
    assert!((volume - 4_000.0).abs() < 80.0, "volume {volume}");
    doc.select(Some(group));
    doc.ungroup_selected().unwrap();
    let restored = doc.object(solid).unwrap().world_mesh();
    assert!((mesh_volume(&restored).abs() - 8_000.0).abs() < 20.0);
}

#[test]
fn fillet_rounds_a_box_and_rejects_a_bad_radius() {
    let mut doc = Document::new();
    let id = doc.add_shape(Shape::Box);
    doc.fillet(id, 3.0).unwrap();
    let mesh = doc.object(id).unwrap().local_mesh();
    assert!(printcaddy_core::is_closed(&mesh));
    assert!(mesh_volume(&mesh).abs() < 8_000.0 - 10.0);
    let (min, max) = mesh.bounds();
    assert!((max.x - min.x - 20.0).abs() < 0.2);
    let err = doc.fillet(id, 12.0).unwrap_err();
    assert_eq!(err, FilletError::RadiusTooLarge);
    let cyl = doc.add_cylinder();
    assert_eq!(doc.fillet(cyl, 2.0).unwrap_err(), FilletError::NotABox);
}
