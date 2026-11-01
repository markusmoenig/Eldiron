//! Row connections are authoring data, independent of a tile that currently uses them.
use denrim_materials::format::{Binding, Material, Reference, Row, RowConnection};

/// Upgrade existing coordinate, image and parameter bindings before structural edits.
/// The same source selection can serve several receivers through one row connection.
pub(super) fn capture_material(material: &mut Material) {
    for row in &mut material.rows {
        let references = row
            .coordinates
            .iter()
            .chain(row.tiles.iter().flat_map(|tile| {
                tile.input
                    .iter()
                    .chain(
                        tile.parameters
                            .iter()
                            .filter_map(|(_, binding)| match binding {
                                Binding::From(reference) => Some(reference),
                                Binding::Constant(_) => None,
                            }),
                    )
            }))
            .cloned()
            .collect::<Vec<_>>();
        for source in references {
            if !row.incoming.iter().any(|link| link.source == source) {
                row.incoming.push(RowConnection {
                    id: crate::prelude::Uuid::new_v4().to_string(),
                    source,
                });
            }
        }
    }
}

fn is_used(row: &Row, reference: &Reference) -> bool {
    row.coordinates.as_ref() == Some(reference)
        || row.tiles.iter().any(|tile| {
            tile.input.as_ref() == Some(reference)
                || tile.parameters.iter().any(|(_, binding)| {
                    matches!(binding, Binding::From(current) if current == reference)
                })
        })
}

/// Used connections already appear at their receiver in the editor. These need a
/// separate row port so they remain visible and can be assigned to a new tile.
pub(super) fn unassigned(row: &Row) -> Vec<&RowConnection> {
    row.incoming
        .iter()
        .filter(|link| !is_used(row, &link.source))
        .collect()
}

pub(super) fn connection<'a>(
    material: &'a Material,
    row: &str,
    id: &str,
) -> Result<&'a Reference, String> {
    material
        .rows
        .iter()
        .find(|current| current.id == row)
        .ok_or("Row is missing")?
        .incoming
        .iter()
        .find(|link| link.id == id)
        .map(|link| &link.source)
        .ok_or_else(|| "Row connection is missing".into())
}

/// Change a row port's source or explicitly disconnect it. Any current uses of
/// that connection follow the edit; merely removing a receiver never calls this.
pub(super) fn set_connection(
    material: &mut Material,
    row: &str,
    id: &str,
    source: Option<Reference>,
) -> Result<(), String> {
    let row = material
        .rows
        .iter_mut()
        .find(|current| current.id == row)
        .ok_or("Row is missing")?;
    let index = row
        .incoming
        .iter()
        .position(|link| link.id == id)
        .ok_or("Row connection is missing")?;
    let old = row.incoming[index].source.clone();
    if row.coordinates.as_ref() == Some(&old) {
        row.coordinates = source.clone();
    }
    for tile in &mut row.tiles {
        if tile.input.as_ref() == Some(&old) {
            tile.input = source.clone();
            tile.input_disconnected = source.is_none();
        }
        // An omitted parameter returns to its source definition's default.
        tile.parameters.retain_mut(|(_, binding)| {
            if matches!(binding, Binding::From(current) if *current == old) {
                if let Some(source) = &source {
                    *binding = Binding::From(source.clone());
                } else {
                    return false;
                }
            }
            true
        });
    }
    if let Some(source) = source {
        row.incoming[index].source = source;
    } else {
        row.incoming.remove(index);
    }
    Ok(())
}

/// An empty row with incoming links remains an editable target. An empty source
/// that is still referenced is retained too, so validation diagnoses its removal
/// transactionally instead of silently deleting other rows' connections.
pub(super) fn preserve_empty_rows(material: &mut Material) {
    let empty = material
        .rows
        .iter()
        .filter(|row| {
            row.tiles.is_empty()
                && row.incoming.is_empty()
                && !material.rows.iter().any(|other| {
                    other.incoming.iter().any(|link| link.source.row == row.id)
                        || other.coordinates.as_ref().is_some_and(|r| r.row == row.id)
                        || other.tiles.iter().any(|tile| {
                            tile.input.as_ref().is_some_and(|r| r.row == row.id)
                                || tile.parameters.iter().any(|(_, binding)| {
                                    matches!(binding, Binding::From(r) if r.row == row.id)
                                })
                        })
                })
        })
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    material.rows.retain(|row| !empty.contains(&row.id));
    material.outputs.retain(|o| !empty.contains(&o.source.row));
}

#[cfg(test)]
mod tests {
    use super::*;
    use denrim_materials::{format, integration::EditableMaterialAsset, ir::Program};

    fn material() -> Material {
        format::parse(
            "project 1\nmaterial \"m\" called \"Material\"\n  mode pbr\n  row \"result\"\n    use \"Perlin Noise\" id \"noise\"\n    use \"Mix\" id \"mix\"\n      set \"color1\" from row \"dark\" xyzw\n      set \"color2\" from row \"light\" xyzw\n  row \"dark\"\n    use \"Color\" id \"dark-color\"\n  row \"light\"\n    use \"Color\" id \"light-color\"\n  output base color from row \"result\" xyz\n",
        )
        .unwrap()
        .materials
        .remove(0)
    }

    fn project(material: Material) -> format::Project {
        format::Project {
            result: None,
            materials: vec![material],
        }
    }

    #[test]
    fn deleting_mix_retains_color_connections_through_bundle_and_reassignment() {
        let mut mat = material();
        capture_material(&mut mat);
        assert!(unassigned(&mat.rows[0]).is_empty());
        assert_eq!(mat.rows[0].incoming.len(), 2);
        let original = mat.clone();
        mat.rows[0].tiles.retain(|tile| tile.id != "mix");
        preserve_empty_rows(&mut mat);
        assert_eq!(unassigned(&mat.rows[0]).len(), 2);
        assert_eq!(mat.rows[0].incoming, original.rows[0].incoming);
        Program::lower(&project(mat.clone())).unwrap();
        let mut asset =
            EditableMaterialAsset::parse("asset", &format::write(&project(mat)), "m").unwrap();
        let reopened = EditableMaterialAsset::reopen(&asset.save().unwrap()).unwrap();
        assert_eq!(reopened.root.project, asset.root.project);
        let row = &mut asset.root.project.materials[0].rows[0];
        row.tiles.push(original.rows[0].tiles[1].clone());
        capture_material(&mut asset.root.project.materials[0]);
        assert!(unassigned(&asset.root.project.materials[0].rows[0]).is_empty());
        assert_eq!(asset.root.project.materials[0].rows[0].incoming.len(), 2);
        asset.validate().unwrap();
    }

    #[test]
    fn empty_receiver_row_is_retained_and_unreferenced_empty_rows_are_removed() {
        let mut mat = material();
        capture_material(&mut mat);
        mat.rows[0].tiles.clear();
        preserve_empty_rows(&mut mat);
        assert!(mat.rows[0].tiles.is_empty());
        assert_eq!(unassigned(&mat.rows[0]).len(), 2);
        Program::lower(&project(mat.clone())).unwrap();
        let id = mat.rows[0].incoming[0].id.clone();
        set_connection(&mut mat, "result", &id, None).unwrap();
        let id = mat.rows[0].incoming[0].id.clone();
        set_connection(&mut mat, "result", &id, None).unwrap();
        preserve_empty_rows(&mut mat);
        assert!(!mat.rows.iter().any(|row| row.id == "result"));
        assert!(mat.outputs.is_empty());
    }

    #[test]
    fn explicit_row_connection_edit_updates_uses_and_disconnect_restores_defaults() {
        let mut mat = material();
        capture_material(&mut mat);
        let id = mat.rows[0].incoming[0].id.clone();
        let mut source = connection(&mat, "result", &id).unwrap().clone();
        source.mapping = Some((-1., 2.));
        set_connection(&mut mat, "result", &id, Some(source.clone())).unwrap();
        assert_eq!(connection(&mat, "result", &id).unwrap(), &source);
        assert_eq!(mat.rows[0].tiles[1].parameters[0].1, Binding::From(source));
        set_connection(&mut mat, "result", &id, None).unwrap();
        assert_eq!(mat.rows[0].incoming.len(), 1);
        assert!(
            !mat.rows[0].tiles[1]
                .parameters
                .iter()
                .any(|(id, _)| id == "color1")
        );
        Program::lower(&project(mat)).unwrap();
    }

    #[test]
    fn deleting_a_referenced_source_is_diagnosed_without_dropping_its_links() {
        let mut mat = material();
        capture_material(&mut mat);
        mat.rows[1].tiles.clear();
        preserve_empty_rows(&mut mat);
        assert!(mat.rows.iter().any(|row| row.id == "dark"));
        assert_eq!(mat.rows[0].incoming.len(), 2);
        assert!(
            Program::lower(&project(mat))
                .unwrap_err()
                .to_string()
                .contains("needs a tile")
        );
    }
}
