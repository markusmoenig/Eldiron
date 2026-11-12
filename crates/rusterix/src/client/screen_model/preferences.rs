//! Shared preferences policy and floating-panel geometry; no renderer or clock.
use super::PreferencesPanelConfig;
use crate::Rect;
use vek::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreferenceSetting {
    Tooltips,
    TooltipDelay,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipDelay {
    Instant,
    Short,
    #[default]
    Normal,
}
impl TooltipDelay {
    pub fn milliseconds(self) -> u64 {
        match self {
            Self::Instant => 0,
            Self::Short => 300,
            Self::Normal => 650,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Instant => "Instant",
            Self::Short => "Short",
            Self::Normal => "Normal",
        }
    }
    pub fn from_milliseconds(value: u64) -> Self {
        if value == 0 {
            Self::Instant
        } else if value <= 350 {
            Self::Short
        } else {
            Self::Normal
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiPreferences {
    pub tooltips_enabled: bool,
    pub tooltip_delay: TooltipDelay,
}
impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            tooltips_enabled: true,
            tooltip_delay: TooltipDelay::Normal,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PreferencesPanelLayout {
    pub rect: Rect,
    pub close: Rect,
    pub labels: [Rect; 2],
    pub tooltips: Rect,
    pub delay: Rect,
    pub reset: Rect,
}
pub fn preferences_panel_geometry(
    config: &PreferencesPanelConfig,
    position: Option<Vec2<i32>>,
    viewport: Vec2<i32>,
) -> PreferencesPanelLayout {
    let natural_height = config.title_height + config.padding * 2. + config.row_height * 3. + 12.;
    let width = config.width.max(0.).min((viewport.x - 4).max(0) as f32);
    let height = natural_height.max(0.).min((viewport.y - 4).max(0) as f32);
    let x = position
        .map_or(16., |p| p.x as f32)
        .clamp(2., (viewport.x as f32 - width - 2.).max(2.));
    let y = position
        .map_or((viewport.y as f32 - natural_height - 72.).max(2.), |p| {
            p.y as f32
        })
        .clamp(2., (viewport.y as f32 - height - 2.).max(2.));
    let first_y = y + config.title_height + config.padding;
    let label_width = 108f32.min((width - config.padding * 2.).max(0.) * 0.6);
    let control_x = x + config.padding + label_width;
    let control_width = (width - config.padding * 2. - label_width).max(0.);
    let row = |index: f32| first_y + (config.row_height + 4.) * index;
    PreferencesPanelLayout {
        rect: Rect::new(x, y, width, height),
        close: Rect::new(x + width - 28., y + 6., 20., 20.),
        labels: [0., 1.].map(|index| {
            Rect::new(
                x + config.padding,
                row(index),
                (label_width - 8.).max(0.),
                config.row_height,
            )
        }),
        tooltips: Rect::new(control_x, first_y, control_width, config.row_height),
        delay: Rect::new(control_x, row(1.), control_width, config.row_height),
        reset: Rect::new(
            x + config.padding,
            row(2.),
            (width - config.padding * 2.).max(0.),
            config.row_height,
        ),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preference_geometry_keeps_reference_placement_and_moves_all_controls_together() {
        let c = PreferencesPanelConfig::default();
        let first = preferences_panel_geometry(&c, None, Vec2::new(960, 540));
        assert_eq!(first.rect, Rect::new(16., 306., 290., 162.));
        assert!(first.tooltips.x >= first.labels[0].x + first.labels[0].width);
        assert!(first.reset.y + first.reset.height <= first.rect.y + first.rect.height);
        let moved = preferences_panel_geometry(&c, Some(Vec2::new(80, 90)), Vec2::new(960, 540));
        assert_eq!(
            [
                moved.tooltips.x - first.tooltips.x,
                moved.tooltips.y - first.tooltips.y
            ],
            [64., -216.]
        );
        assert_eq!(TooltipDelay::from_milliseconds(350), TooltipDelay::Short);
        assert_eq!(UiPreferences::default().tooltip_delay.milliseconds(), 650);
    }
}
