//! Export the supplied editable fauna in 2D, and optionally native GPU 3D.
//! cargo run -p rusterix --example animals -- /tmp/eldiron-animals --gpu
use denrim_materials::geometry::{Document, Value as GeoValue};
use eldiron_ruleset as rules;
use image::{
    Delay, Frame, RgbaImage,
    codecs::gif::{GifEncoder, Repeat},
};
use rusterix::{Assets, Entity, procedural_avatar::Appearance};
use std::{fs::File, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let out = PathBuf::from(args.first().map_or("/tmp/eldiron-animals", String::as_str));
    std::fs::create_dir_all(&out)?;
    let mut assets = Assets::default();
    assets.palette = rules::palette_template_from_source(rules::latest_official_ruleset())?;
    let definitions = rules::bundled_appearance_assets_for_ruleset(
        rules::OFFICIAL_RULESET_ID,
        rules::OFFICIAL_RULESET_VERSION,
    );
    let animals = rules::animals::from_source(rules::latest_official_ruleset())?;
    let mut overview = RgbaImage::new(animals.len() as u32 * 256, 256);
    for (index, (id, animal)) in animals.iter().enumerate() {
        let asset = definitions.iter().find(|a| a.id == id).unwrap();
        let evaluated = Document::load(asset.source)?.evaluate()?;
        let appearance = Appearance::new(asset.source, &[], 1)?;
        let mut actor = Entity::default();
        actor.orientation = vek::Vec2::new(1., 1.).normalized();
        let mut sheet = RgbaImage::new(8 * 128, 6 * 128);
        for (row, clip) in ["idle", "walk", "run", "graze", "attack", "dead"]
            .iter()
            .enumerate()
        {
            let GeoValue::Motion(motion) = &evaluated.rows[*clip] else {
                unreachable!()
            };
            let mut gif = GifEncoder::new(File::create(out.join(format!("{id}-{clip}.gif")))?);
            gif.set_repeat(Repeat::Infinite)?;
            for frame in 0..16 {
                let phase = frame as f32 / if motion.looping { 16. } else { 15. };
                let rgba = appearance.sprite_isometric(
                    &actor,
                    &assets,
                    clip,
                    phase * motion.duration,
                    0.,
                    None,
                )?;
                let image = RgbaImage::from_raw(128, 128, rgba.as_ref().clone()).unwrap();
                let large =
                    image::imageops::resize(&image, 256, 256, image::imageops::FilterType::Nearest);
                gif.encode_frame(Frame::from_parts(
                    large,
                    0,
                    0,
                    Delay::from_numer_denom_ms((motion.duration * 1000. / 16.) as u32, 1),
                ))?;
            }
            for view in 0..8 {
                let yaw = (view as f32 * 45.).to_radians();
                actor.orientation = vek::Vec2::new(yaw.sin(), yaw.cos());
                let rgba = appearance.sprite_isometric(
                    &actor,
                    &assets,
                    clip,
                    motion.duration * 0.45,
                    0.,
                    None,
                )?;
                let image = RgbaImage::from_raw(128, 128, rgba.as_ref().clone()).unwrap();
                image::imageops::overlay(&mut sheet, &image, view * 128, row as i64 * 128);
            }
            actor.orientation = vek::Vec2::new(1., 1.).normalized();
        }
        sheet.save(out.join(format!("{id}-poses.png")))?;
        let rgba = appearance.sprite_isometric(&actor, &assets, "idle", 0., 0., None)?;
        let image = RgbaImage::from_raw(128, 128, rgba.as_ref().clone()).unwrap();
        let large = image::imageops::resize(&image, 256, 256, image::imageops::FilterType::Nearest);
        image::imageops::overlay(&mut overview, &large, index as i64 * 256, 0);
        #[cfg(feature = "graphics")]
        if args.iter().any(|a| a == "--gpu") {
            native(&appearance, &assets, &out, id)?;
        }
        println!("{}: {id}-poses.png, six motion GIFs", animal.name);
    }
    overview.save(out.join("animals.png"))?;
    Ok(())
}

#[cfg(feature = "graphics")]
fn native(
    appearance: &Appearance,
    assets: &Assets,
    out: &std::path::Path,
    id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use rusterix::{D3Camera, D3IsoCamera};
    use scenevm::{Atom, GeoId, RenderMode, SceneVM};
    use vek::{Vec3, Vec4};
    let height = appearance.model_height(assets)?;
    let mut vm = SceneVM::new(512, 384);
    let mut camera = D3IsoCamera::new();
    camera.scale = 1.7;
    camera.center = Vec3::new(0., height * 0.5, 0.);
    camera.azimuth_deg = 50.;
    camera.elevation_deg = 20.;
    vm.execute(Atom::SetRenderMode(RenderMode::Raster3D));
    vm.execute(Atom::SetBackground(Vec4::new(0.12, 0.14, 0.16, 1.)));
    vm.execute(Atom::SetGP0(Vec4::new(0.02, 0.02, 0.02, 0.)));
    vm.execute(Atom::SetGP1(Vec4::new(1., 1., 1., 0.6)));
    vm.execute(Atom::SetGP2(Vec4::new(0.4, 0.8, 0.5, 1.)));
    vm.execute(Atom::SetGP3(Vec4::new(1., 1., 1., 0.7)));
    vm.execute(Atom::SetGP8(Vec4::new(0., 5., 1., 1.)));
    vm.execute(Atom::SetGP9(Vec4::new(0., 0., 1., 1.)));
    vm.execute(Atom::SetCamera3D {
        camera: camera.as_scenevm_camera_for_surface(512., 384.),
    });
    let mut uploaded = Default::default();
    let mut gif = GifEncoder::new(File::create(out.join(format!("{id}-3d-walk.gif")))?);
    gif.set_repeat(Repeat::Infinite)?;
    let doc = Document::load(&appearance.source)?.evaluate()?;
    let GeoValue::Motion(walk) = &doc.rows["walk"] else {
        unreachable!()
    };
    for frame in 0..16 {
        vm.execute(Atom::ClearDynamics);
        let objects = appearance.objects(
            &Entity::default(),
            assets,
            GeoId::Character(1),
            Vec3::zero(),
            height,
            "walk",
            frame as f32 / 16. * walk.duration,
            &mut vm,
            &mut uploaded,
            false,
        )?;
        assert!(!objects.is_empty());
        for object in objects {
            vm.execute(Atom::AddDynamic { object });
        }
        if frame == 0 {
            vm.execute(Atom::BuildAtlas);
        }
        let mut pixels = vec![0; 512 * 384 * 4];
        vm.render_frame(&mut pixels, 512, 384);
        let image = RgbaImage::from_raw(512, 384, pixels).unwrap();
        if frame == 4 {
            image.save(out.join(format!("{id}-3d.png")))?;
        }
        gif.encode_frame(Frame::from_parts(
            image,
            0,
            0,
            Delay::from_numer_denom_ms((walk.duration * 1000. / 16.) as u32, 1),
        ))?;
    }
    Ok(())
}
