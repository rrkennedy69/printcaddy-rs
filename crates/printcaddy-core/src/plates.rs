//! Extra build plates, in the same grid as the TypeScript app.

use glam::Vec3;

use crate::document::{Document, ObjectId};
use crate::PLATE_SIZE_MM;

pub const PLATES_PER_ROW: u32 = 3;
pub const PLATE_GAP_MM: f32 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateLayout {
    pub slot: u32,
    pub number: u32,
    pub center_x: f32,
    pub center_z: f32,
}

fn pitch() -> f32 {
    PLATE_SIZE_MM + PLATE_GAP_MM
}

pub fn center_of(slot: u32) -> (f32, f32) {
    let step = pitch();
    (
        (slot % PLATES_PER_ROW) as f32 * step,
        (slot / PLATES_PER_ROW) as f32 * step,
    )
}

impl Document {
    pub fn plate_layouts(&self) -> Vec<PlateLayout> {
        let mut slots = vec![0];
        slots.extend(self.extra_slots.iter().copied());
        slots
            .into_iter()
            .enumerate()
            .map(|(index, slot)| {
                let (center_x, center_z) = center_of(slot);
                PlateLayout {
                    slot,
                    number: index as u32 + 1,
                    center_x,
                    center_z,
                }
            })
            .collect()
    }

    pub fn add_plate(&mut self) -> u32 {
        let next = self.extra_slots.iter().copied().max().unwrap_or(0) + 1;
        self.extra_slots.push(next);
        next
    }

    /// Removes a plate. Its bodies move to the floor beside the grid, and later plates slide over.
    pub fn delete_plate(&mut self, slot: u32) -> Result<(), String> {
        let plates = self.plate_layouts();
        if plates.len() < 2 {
            return Err("The last plate stays.".into());
        }
        let Some(plate) = plates.iter().copied().find(|plate| plate.slot == slot) else {
            return Err("That plate is not on the bed.".into());
        };
        let remaining: Vec<PlateLayout> = plates
            .into_iter()
            .filter(|item| item.slot != slot)
            .collect();
        let move_to: Vec<(u32, u32)> = remaining
            .iter()
            .enumerate()
            .map(|(index, item)| (item.slot, index as u32))
            .collect();
        let floor_x = (remaining.len().min(PLATES_PER_ROW as usize) as f32) * pitch();
        let half = PLATE_SIZE_MM * 0.5;
        let on = |x: f32, z: f32, item: &PlateLayout| {
            (x - item.center_x).abs() <= half + 0.5 && (z - item.center_z).abs() <= half + 0.5
        };
        for object in self.objects_mut() {
            if on(object.position.x, object.position.z, &plate) {
                object.position.x = object.position.x - plate.center_x + floor_x;
                object.position.z -= plate.center_z;
                object.locked = false;
                continue;
            }
            if let Some(from) = remaining
                .iter()
                .find(|item| on(object.position.x, object.position.z, item))
            {
                let to_slot = move_to
                    .iter()
                    .find(|(from_slot, _)| *from_slot == from.slot)
                    .map(|(_, to)| *to)
                    .unwrap_or(from.slot);
                let (to_x, to_z) = center_of(to_slot);
                object.position.x += to_x - from.center_x;
                object.position.z += to_z - from.center_z;
            }
        }
        self.locked_slots = self
            .locked_slots
            .iter()
            .filter_map(|locked| {
                if *locked == slot {
                    None
                } else {
                    move_to
                        .iter()
                        .find(|(from, _)| from == locked)
                        .map(|(_, to)| *to)
                }
            })
            .collect();
        self.extra_slots = (1..remaining.len() as u32).collect();
        Ok(())
    }

    pub fn lock_plate(&mut self, slot: u32, locked: bool) {
        let plates = self.plate_layouts();
        let Some(plate) = plates.iter().copied().find(|plate| plate.slot == slot) else {
            return;
        };
        let half = PLATE_SIZE_MM * 0.5;
        for object in self.objects_mut() {
            if (object.position.x - plate.center_x).abs() <= half + 0.5
                && (object.position.z - plate.center_z).abs() <= half + 0.5
            {
                object.locked = locked;
            }
        }
        if locked {
            if !self.locked_slots.contains(&slot) {
                self.locked_slots.push(slot);
            }
        } else {
            self.locked_slots.retain(|item| *item != slot);
        }
    }

    /// A body that has just been dropped onto a locked plate becomes locked.
    pub fn apply_plate_lock(&mut self, id: ObjectId) {
        let Some(object) = self.object(id) else {
            return;
        };
        let x = object.position.x;
        let z = object.position.z;
        let half = PLATE_SIZE_MM * 0.5;
        let lock = self.plate_layouts().iter().any(|plate| {
            self.locked_slots.contains(&plate.slot)
                && (x - plate.center_x).abs() <= half + 0.5
                && (z - plate.center_z).abs() <= half + 0.5
        });
        if lock {
            if let Some(object) = self.object_mut(id) {
                object.locked = true;
            }
        }
    }

    pub fn plate_at(&self, position: Vec3) -> PlateLayout {
        let plates = self.plate_layouts();
        plates
            .iter()
            .copied()
            .min_by(|a, b| {
                let da = (position.x - a.center_x).hypot(position.z - a.center_z);
                let db = (position.x - b.center_x).hypot(position.z - b.center_z);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(PlateLayout {
                slot: 0,
                number: 1,
                center_x: 0.0,
                center_z: 0.0,
            })
    }
}
