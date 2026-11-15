//! Advance fresh view requests through the same bounded frames as a host.
pub fn ready_view(
    views: &mut eldiron_presentation::SceneViews,
    id: eldiron_presentation::ViewId,
    revision: u64,
    images: &mut renderkit::ImageRegistry,
) -> eldiron_presentation::ViewImage {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        views.begin_surface_paint_upload_frame();
        if let Some(output) = views.render(id, revision, images).unwrap() {
            return output;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "editor source preparation did not settle"
        );
        images
            .context()
            .device
            .poll(renderkit::wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
    }
}
