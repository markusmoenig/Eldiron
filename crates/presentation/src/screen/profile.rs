use super::*;
use rusterix::{
    Entity, Rect,
    client::widget::{
        avatar::AvatarWidget,
        profile::{ProfilePresentation, ProfileWidget},
    },
};
use theframework::{
    paint::{ImageHandle, PaintOp, Rect as Uv, Sampling},
    text::{TextLayout, TextSystem},
};

#[derive(PartialEq)]
struct IdentityKey {
    revision: u64,
    family: String,
    name: String,
    class: String,
    name_size: u32,
    class_size: u32,
}
/// Retained identity shaping and prepared profile geometry. Preparation handles
/// controller/source/font changes; painting reads this projection only.
#[derive(Default)]
pub struct ProfileLayout {
    key: Option<IdentityKey>,
    view: Option<ProfilePresentation>,
    identity: Option<(TextLayout, TextLayout)>,
    uv: Uv,
    source_key: Option<String>,
    builds: u64,
}
impl ProfileLayout {
    pub fn layouts_built(&self) -> u64 {
        self.builds
    }
    pub fn view(&self) -> Option<&ProfilePresentation> {
        self.view.as_ref()
    }
    pub fn source_uv(&self) -> Uv {
        self.uv
    }
    pub fn prepare(
        &mut self,
        widget: &ProfileWidget,
        entity: &Entity,
        text: &mut TextSystem,
        family: &str,
        content: Option<&PreparedContent>,
    ) {
        let Some(view) = widget.presentation(entity) else {
            self.view = None;
            self.source_key = None;
            return;
        };
        let key = IdentityKey {
            revision: text.font_revision(),
            family: family.into(),
            name: view.name.clone(),
            class: view.class.clone(),
            name_size: view.name_size.to_bits(),
            class_size: view.class_size.to_bits(),
        };
        if self.key.as_ref() != Some(&key) {
            self.identity = Some((
                text.layout(&view.name, family, view.name_size, None),
                text.layout(&view.class, family, view.class_size, None),
            ));
            self.key = Some(key);
            self.builds += 2;
        }
        let (name, class) = self.identity.as_ref().unwrap();
        let height = name.height()
            + if view.class.is_empty() {
                0.
            } else {
                view.text_gap + class.height()
            };
        self.view = widget.measured_presentation(entity, height);
        self.uv = Uv::new(0., 0., 1., 1.);
        self.source_key = content.map(|c| c.image_key().to_string());
        if let Some(content) = content.filter(|c| c.avatar) {
            if let Some(bounds) = content.alpha_bounds {
                let (x, y, w, h) = widget.source_crop(content.image().width(), bounds);
                let (iw, ih) = (
                    content.image().width() as f32,
                    content.image().height() as f32,
                );
                self.uv = Uv::new(x as f32 / iw, y as f32 / ih, w as f32 / iw, h as f32 / ih);
            } else {
                self.source_key = None;
            }
        }
    }
}
impl ScreenPainter<'_> {
    fn silhouette(
        &mut self,
        image: ImageHandle,
        bounds: Rect,
        uv: Uv,
        width: usize,
        pixel: [u8; 4],
    ) {
        if width > 0 && pixel[3] > 0 {
            self.list.ops.push(PaintOp::ImageOutline {
                image,
                rect: rect(bounds),
                uv,
                width: width as f32,
                color: color(pixel),
                sampling: Sampling::Nearest,
            });
        }
        self.list.ops.push(PaintOp::Image {
            image,
            rect: rect(bounds),
            uv,
            sampling: Sampling::Nearest,
            tint: [1.; 4],
        });
    }
    /// The controller updates damage timing before paint. Source generation is a
    /// separate ContentSources operation; health changes only alter the outline.
    pub fn avatar(
        &mut self,
        widget: &AvatarWidget,
        content: Option<&PreparedContent>,
        outline: ([u8; 4], usize),
    ) -> Result<(), String> {
        if widget.rect.width <= 0. || widget.rect.height <= 0. {
            return Ok(());
        }
        let Some(content) = content else {
            return Ok(());
        };
        let size = widget.rect.width.min(widget.rect.height);
        let bounds = Rect::new(
            widget.rect.x + (widget.rect.width - size) * 0.5,
            widget.rect.y + (widget.rect.height - size) * 0.5,
            size,
            size,
        );
        let image = content.resolve(self.images)?;
        self.list.ops.push(PaintOp::PushClip(rect(widget.rect)));
        self.silhouette(
            image,
            bounds,
            Uv::new(0., 0., 1., 1.),
            outline.1,
            outline.0,
        );
        self.list.ops.push(PaintOp::PopClip);
        Ok(())
    }
    pub fn profile(
        &mut self,
        layout: &ProfileLayout,
        content: Option<&PreparedContent>,
    ) -> Result<(), String> {
        let Some(view) = &layout.view else {
            return Ok(());
        };
        let content = if let Some(key) = &layout.source_key {
            let content = content.ok_or("profile source lease missing after preparation")?;
            if content.image_key() != key {
                return Err("profile source changed; prepare its layout before painting".into());
            }
            Some(content)
        } else {
            None
        };
        let image = content.map(|c| c.resolve(self.images)).transpose()?;
        self.list.ops.push(PaintOp::PushClip(rect(view.bounds)));
        if let Some(image) = image {
            self.silhouette(
                image,
                view.image,
                layout.uv,
                view.outline_size,
                view.outline_color,
            );
            if let Some(bounds) = view.text {
                let (name, class) = layout.identity.as_ref().unwrap();
                let height = name.height()
                    + if view.class.is_empty() {
                        0.
                    } else {
                        view.text_gap + class.height()
                    };
                let top = (bounds.y + (bounds.height - height) * 0.5).max(bounds.y);
                self.list.ops.push(PaintOp::PushClip(rect(bounds)));
                for (shaped, origin, pixel) in [
                    (name, [bounds.x, top], view.name_color),
                    (
                        class,
                        [bounds.x, top + name.height() + view.text_gap],
                        view.class_color,
                    ),
                ] {
                    for run in &shaped.runs {
                        self.list.ops.push(PaintOp::GlyphRun {
                            run: run.clone(),
                            origin,
                            color: color(pixel),
                        });
                    }
                }
                self.list.ops.push(PaintOp::PopClip);
            }
            for bar in &view.bars {
                self.fill(bar.bounds, bar.background);
                let mut fill = bar.bounds;
                if bar.vertical {
                    fill.height = (fill.height * bar.ratio).round();
                    fill.y += bar.bounds.height - fill.height;
                } else {
                    fill.width = (fill.width * bar.ratio).round();
                }
                self.fill(fill, bar.fill);
                self.border(bar.bounds, bar.border_size as f32, bar.border);
            }
        }
        self.border(view.bounds, view.selected_size as f32, view.selected_color);
        self.list.ops.push(PaintOp::PopClip);
        Ok(())
    }
}
