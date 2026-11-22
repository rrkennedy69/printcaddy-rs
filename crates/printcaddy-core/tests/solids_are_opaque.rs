use printcaddy_core::{Document, DEFAULT_MODEL_COLOR};

#[test]
fn a_new_solid_is_fully_opaque() {
    let mut doc = Document::new();
    let id = doc.add_box();
    let object = doc.object(id).unwrap();
    assert!(!object.hole);
    assert!(!object.transparent);
    assert_eq!(object.color, DEFAULT_MODEL_COLOR);
    assert_eq!(doc.display_alpha(id), 1.0);
    let cylinder = doc.add_cylinder();
    assert_eq!(doc.display_alpha(cylinder), 1.0);
}

#[test]
fn a_hole_or_a_transparent_flag_is_what_makes_a_body_see_through() {
    let mut doc = Document::new();
    let id = doc.add_box();
    doc.set_hole(id, true);
    assert!(doc.display_alpha(id) < 0.5);
    doc.set_hole(id, false);
    assert_eq!(doc.display_alpha(id), 1.0);
    doc.object_mut(id).unwrap().transparent = true;
    let glass = doc.display_alpha(id);
    assert!(glass < 1.0 && glass > 0.0);
}
