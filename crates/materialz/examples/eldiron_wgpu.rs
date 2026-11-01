//! Headless integration example. An embedded host supplies its own device/queue.
use denrim_materials::{
    gpu::{MaterialZ, RenderRequest, SurfaceDomain},
    integration::{EditableMaterialAsset, InstanceRecord},
    metadata::ValueType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let source = "project 1\nmaterial \"m\" called \"Eldiron Tile\"\n  mode pbr\n  tiling raw\n  default roughness to 0.8\n  row \"color\"\n    use \"Color\" id \"pigment\"\n      set \"color\" to 0.15 0.3 0.08 1\n  output base color from row \"color\" xyz\n";
    // All of this works before acquiring a GPU, including validation and saving.
    let mut asset = EditableMaterialAsset::parse("eldiron-grass", source, "m")?;
    let metadata = asset.parameter_metadata()?;
    let color = metadata
        .iter()
        .find(|p| p.definition.kind == ValueType::Color)
        .unwrap()
        .key
        .clone();
    asset.validate()?;
    let saved = asset.save()?;
    let reopened = EditableMaterialAsset::reopen(&saved)?;

    let instance = wgpu::Instance::default();
    let adapter = instance.request_adapter(&Default::default()).await?;
    let (device, queue) = adapter.request_device(&Default::default()).await?;
    let runtime = MaterialZ::new(&device, &queue).await?;
    let mut material = runtime
        .compile(&reopened, InstanceRecord::new("placement-1"))
        .await?;
    let generation = material.pipeline_generation();
    material.set_parameter(color, vec![0.2, 0.4, 0.1, 1.])?;
    assert_eq!(generation, material.pipeline_generation());

    let request = RenderRequest {
        time: 0.,
        resolution: [32, 32],
        domain: SurfaceDomain::rect([12., 8.], [1., 1.]),
    };
    let surface = runtime.create_surface(&request)?;
    let mut encoder = device.create_command_encoder(&Default::default());
    let ticket = runtime.encode(&mut encoder, &material, &request, &surface)?;
    // Eldiron can record its own rendering work before submitting this encoder.
    queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    ticket.map_diagnostics(move |result| {
        let _ = send.send(result);
    });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(30)),
    })?;
    receive.recv()??;

    asset.instances.push(material.snapshot());
    let saved = asset.save()?; // Persist this bundle in Eldiron's asset store.
    for map in &surface.maps {
        println!(
            "{}: {:?}, {}",
            map.layout.channel.name(),
            map.layout.format,
            map.layout.components
        );
    }
    println!(
        "Rendered 32x32; retained pipeline {generation}; editable bundle {} bytes",
        saved.len()
    );
    Ok(())
}
