//! Editable documents. Value edits are transactional and retain the text model.
use crate::{Error, Result, format, ir::Program};

pub struct Document {
    source: format::Project,
    program: Program,
}
impl Document {
    pub fn parse(text: &str) -> Result<Self> {
        let source = format::parse(text)?;
        let program = Program::lower(&source)?;
        Ok(Self { source, program })
    }
    pub fn program(&self) -> &Program {
        &self.program
    }
    pub fn text(&self) -> String {
        format::write(&self.source)
    }
    /// Structured authoring model for native clients; computation remains in Rust.
    pub fn json(&self) -> String {
        use crate::json::{numbers, string as j};
        fn reference(r: &format::Reference) -> String {
            use crate::json::string as j;
            let mapping = r
                .mapping
                .map(|(a, b)| crate::json::numbers(&[a, b]))
                .unwrap_or_else(|| "null".into());
            format!(
                "{{\"row\":{},\"components\":{},\"mapping\":{mapping}}}",
                j(&r.row),
                j(&r.components)
            )
        }
        let materials=self.source.materials.iter().map(|m| {
            let rows=m.rows.iter().map(|r| {
                let tiles=r.tiles.iter().enumerate().map(|(i,t)| {
                    let params=t.parameters.iter().map(|(id,b)| {
                        let value=match b {
                            format::Binding::Constant(v)=>format!("{{\"values\":{}}}",numbers(v)),
                            format::Binding::From(r)=>format!("{{\"reference\":{}}}",reference(r)),
                        };
                        format!("{}:{value}",j(id))
                    }).collect::<Vec<_>>().join(",");
                    let code=t.code.as_deref().map(j).unwrap_or_else(||"null".into());
                    let input=t.input.as_ref().map(reference).unwrap_or_else(|| "null".into());
                    let distance=self.program.tile(&m.id,&r.id,i).map(|id|self.program.nodes[id.0].has_distance).unwrap_or(false);
                    format!("{{\"id\":{},\"kernel\":{},\"combine\":{},\"input\":{input},\"input_disconnected\":{},\"parameters\":{{{params}}},\"has_distance\":{distance},\"code\":{code}}}",j(&t.id),j(&t.kernel),j(t.combine.name()),t.input_disconnected)
                }).collect::<Vec<_>>().join(",");
                let coordinates=r.coordinates.as_ref().map(reference).unwrap_or_else(|| "null".into());
                let incoming=r.incoming.iter().map(|link|format!("{{\"id\":{},\"source\":{}}}",j(&link.id),reference(&link.source))).collect::<Vec<_>>().join(",");
                format!("{{\"id\":{},\"label\":{},\"coordinates\":{coordinates},\"incoming\":[{incoming}],\"tiles\":[{tiles}]}}",j(&r.id),j(&r.label))
            }).collect::<Vec<_>>().join(",");
            let stack_slot=m.stack_slot.map(|v|v.to_string()).unwrap_or_else(||"null".into());
            let position=m.position.map(|p|numbers(&p)).unwrap_or_else(||"null".into());
            let defaults=m.defaults.iter().map(|(name,value)|format!("{}:{}",j(name),numbers(value))).collect::<Vec<_>>().join(",");
            let outputs=m.outputs.iter().map(|o|format!("{{\"channel\":{},\"source\":{}}}",j(&o.channel),reference(&o.source))).collect::<Vec<_>>().join(",");
            let mixing=m.mixing.as_ref().map(|mix| {
                let inputs=mix.inputs.iter().map(|v|v.as_deref().map(j).unwrap_or_else(||"null".into())).collect::<Vec<_>>().join(",");
                let mask=match &mix.mask {format::Binding::Constant(v)=>format!("{{\"values\":{}}}",numbers(v)),format::Binding::From(r)=>format!("{{\"reference\":{}}}",reference(r))};
                let mask_material=mix.mask_material.as_deref().map(j).unwrap_or_else(||"null".into());
                format!("{{\"kind\":{},\"inputs\":[{inputs}],\"mask_material\":{mask_material},\"mask\":{mask},\"blend\":{},\"direction\":{},\"levels\":{}}}",j(&mix.kind),mix.blend,mix.direction,numbers(&mix.levels))
            }).unwrap_or_else(||"null".into());
            let animation=format!("{{\"frames\":{},\"duration\":{}}}",m.animation.frames,m.animation.duration);
            let tiling=format!("{{\"mode\":{},\"period\":{},\"blend\":{}}}",j(m.tiling.mode.name()),numbers(&m.tiling.period),m.tiling.blend);
            format!("{{\"stackSlot\":{stack_slot},\"locked\":{},\"tiling\":{tiling},\"animation\":{animation},\"id\":{},\"label\":{},\"mode\":{},\"position\":{position},\"mixing\":{mixing},\"defaults\":{{{defaults}}},\"rows\":[{rows}],\"outputs\":[{outputs}]}}",m.locked,j(&m.id),j(&m.label),j(&m.mode))
        }).collect::<Vec<_>>().join(",");
        let result = self
            .source
            .result
            .as_deref()
            .map(j)
            .unwrap_or_else(|| "null".into());
        format!("{{\"version\":1,\"result\":{result},\"materials\":[{materials}]}}")
    }

    /// Change an existing constant (including an implicit default). Connected
    /// parameters require a deliberate structural edit, never silent replacement.
    /// On error neither the document nor the evaluator changes.
    pub fn set_constant(
        &mut self,
        material: &str,
        row: &str,
        tile: usize,
        parameter: &str,
        values: &[f32],
    ) -> Result<()> {
        let mut next = self.source.clone();
        let material = next
            .materials
            .iter_mut()
            .find(|m| m.id == material)
            .ok_or_else(|| Error(format!("Unknown material {material:?}")))?;
        let row = material
            .rows
            .iter_mut()
            .find(|r| r.id == row)
            .ok_or_else(|| Error(format!("Unknown row {row:?}")))?;
        let tile = row
            .tiles
            .get_mut(tile)
            .ok_or_else(|| Error("Unknown tile index".into()))?;
        if let Some((_, binding)) = tile.parameters.iter_mut().find(|(id, _)| id == parameter) {
            if matches!(binding, format::Binding::From(_)) {
                return Err(Error(
                    "Parameter is connected; disconnect it through a structural edit first".into(),
                ));
            }
            *binding = format::Binding::Constant(values.to_vec());
        } else {
            tile.parameters
                .push((parameter.into(), format::Binding::Constant(values.to_vec())));
        }
        let program = Program::lower(&next)?;
        self.source = next;
        self.program = program;
        Ok(())
    }
}
