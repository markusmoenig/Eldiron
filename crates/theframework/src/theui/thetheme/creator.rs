use crate::prelude::*;

// Keep every role in the palette, including roles inherited by the Black theme.
const ROLES: &[TheThemeColors] = &[
    EditorCanvasBackground,
    EditorGrid,
    EditorSubGrid,
    MaterialCanvas,
    MaterialGrid,
    MaterialPanel,
    MaterialControl,
    MaterialText,
    MaterialMuted,
    MaterialWire,
    MaterialDropdown,
    MaterialSource,
    MaterialPopup,
    MaterialPopupHover,
    MaterialPopupSelection,
    MaterialShadow,
    MaterialCheckerDark,
    MaterialCheckerLight,
    Green,
    Red,
    DefaultWidgetBorder,
    DefaultSelection,
    DefaultWidgetBackground,
    DefaultWidgetDarkBackground,
    SwitchbarBorder,
    SectionbarHeaderBorder,
    SectionbarBackground,
    SectionbarNormalTextColor,
    SectionbarSelectedTextColor,
    TextLayoutBackground,
    TextLayoutBorder,
    TextEditBackground,
    TextEditRange,
    TextEditBorder,
    SelectedTextEditBorder1,
    SelectedTextEditBorder2,
    TextEditTextColor,
    TextEditCursorColor,
    TextEditLineNumberColor,
    TextEditLineNumberHighlightColor,
    TextEditLineNumberDebugColor,
    TextEditDebugLineBackground,
    TextLinkColor,
    TextLinkHoveredColor,
    MenubarPopupBackground,
    MenubarPopupBorder,
    SliderSmallColor1,
    SliderSmallColor2,
    SliderSmallColor3,
    SliderSmallColor4,
    MenubarButtonHover,
    MenubarButtonHoverBorder,
    MenubarButtonClicked,
    MenubarButtonClickedBorder,
    MenubarButtonSeparator1,
    MenubarButtonSeparator2,
    ToolbarButtonNormal,
    ToolbarButtonNormalBorder,
    ToolbarButtonHover,
    ToolbarButtonHoverBorder,
    ToolbarButtonClicked,
    ToolbarButtonClickedBorder,
    TraybarButtonNormal,
    TraybarButtonNormalBorder,
    TraybarButtonHover,
    TraybarButtonHoverBorder,
    TraybarButtonClicked,
    TraybarButtonClickedBorder,
    TraybarButtonDisabledBorder,
    TraybarButtonDisabledBackground,
    ListLayoutBackground,
    ListLayoutBorder,
    ListItemNormal,
    ListItemSelected,
    ListItemSelectedNoFocus,
    ListItemHover,
    ListItemText,
    ListItemIconBorder,
    ScrollbarBackground,
    ScrollbarSeparator,
    TabbarBackground,
    TabbarConnector,
    TabbarText,
    TraybarBorder,
    TraybarBackground,
    TraybarBottomBorder,
    StatusbarStart,
    StatusbarEnd,
    DividerStart,
    DividerEnd,
    GroupButtonNormalBorder,
    GroupButtonNormalBackground,
    GroupButtonHoverBorder,
    GroupButtonHoverBackground,
    GroupButtonSelectedBorder,
    GroupButtonSelectedBackground,
    CodeGridBackground,
    CodeGridNormal,
    CodeGridDark,
    CodeGridSelected,
    CodeGridHover,
    CodeGridText,
    DropItemBackground,
    DropItemBorder,
    DropItemText,
    ContextMenuBackground,
    ContextMenuBorder,
    ContextMenuHighlight,
    ContextMenuTextNormal,
    ContextMenuTextDisabled,
    ContextMenuTextHighlight,
    ContextMenuSeparator,
    WindowBorderOuter,
    WindowBorderInner,
    WindowHeaderBackground,
    WindowHeaderBorder1,
    WindowHeaderBorder2,
    TimeSliderBorder,
    TimeSliderBackground,
    TimeSliderText,
    TimeSliderMarker,
    TimeSliderLine,
    TimeSliderPosition,
    NodeCanvasBackground,
    NodeCanvasGrid,
    NodeConnection,
    NodeCutConnection,
    MenuText,
    MenuTextHighlighted,
    MenuHover,
    MenuSelected,
    NodeBackground,
    NodeBorder,
    NodeBorderSelected,
    NodeBody,
    NodeBodySelected,
    ToolListButtonNormalBorder,
    ToolListButtonSelectedBorder,
    ToolListButtonHoverBorder,
    ToolListButtonHoverBackground,
    ToolListButtonSelectedBackground,
    LayoutSeparator,
    TreeViewNodeBorder,
    TreeViewNode,
    TreeViewNodeSelectedBorder,
    TreeViewNodeSelected,
    TreeViewNodeText,
    TreeViewNodePlusMinus,
];

struct CreatorPalette {
    palette_base: TheBlackBlueTheme,
    colors: FxHashMap<TheThemeColors, RGBA>,
    temp: RGBA,
    light: bool,
}

impl CreatorPalette {
    fn new(light: bool) -> Self {
        let base = TheBlackBlueTheme::new();
        let mut colors = FxHashMap::default();
        for &role in ROLES {
            let c = *base.color(role);
            let name = format!("{:?}", role);
            let text = name.ends_with("Text")
                || name.contains("TextColor")
                || name.starts_with("MenuText")
                || name.starts_with("ContextMenuText")
                || name.starts_with("TextLink")
                || name.contains("LineNumber")
                || name.contains("PlusMinus")
                || name.contains("CursorColor")
                || name == "MaterialMuted";
            let accent = name.contains("Selected")
                || name.contains("Selection")
                || name.contains("Clicked")
                || name.contains("Marker")
                || name.contains("Highlight")
                || name == "Green";
            let border = name.contains("Border")
                || name.contains("Separator")
                || name.contains("Grid")
                || name.contains("Connection")
                || name.contains("Line");
            let color = if light {
                if text {
                    if name.contains("Disabled") {
                        [139, 132, 121, c[3]]
                    } else {
                        [48, 43, 38, c[3]]
                    }
                } else if accent {
                    if border {
                        [157, 104, 56, c[3]]
                    } else {
                        [211, 191, 165, c[3]]
                    }
                } else if border {
                    [170, 160, 148, c[3]]
                } else {
                    let lum = (c[0] as u16 + c[1] as u16 + c[2] as u16) / 3;
                    let v = (249_i16 - (lum as i16 * 65 / 255)).clamp(185, 249) as u8;
                    [v, v.saturating_sub(3), v.saturating_sub(9), c[3]]
                }
            } else {
                if text {
                    if name.contains("Disabled") {
                        [151, 139, 122, c[3]]
                    } else {
                        [232, 216, 181, c[3]]
                    }
                } else if accent {
                    if border {
                        [194, 155, 91, c[3]]
                    } else {
                        [91, 78, 67, c[3]]
                    }
                } else if border {
                    [91, 81, 75, c[3]]
                } else {
                    let lum = (c[0] as u16 + c[1] as u16 + c[2] as u16) / 3;
                    let v = (32 + lum * 55 / 255) as u8;
                    [v.saturating_add(5), v.saturating_add(2), v, c[3]]
                }
            };
            colors.insert(role, color);
        }
        // Material surfaces need independent values because they are used by the graph editor.
        for (role, color) in if light {
            [
                (MaterialCanvas, [238, 234, 226, 255]),
                (MaterialPanel, [248, 245, 239, 255]),
                (MaterialText, [48, 43, 38, 255]),
                (MaterialMuted, [110, 100, 89, 255]),
                (MaterialCheckerDark, [180, 175, 168, 255]),
                (MaterialCheckerLight, [225, 220, 212, 255]),
            ]
        } else {
            [
                (MaterialCanvas, [36, 35, 34, 255]),
                (MaterialPanel, [52, 48, 46, 255]),
                (MaterialText, [232, 216, 181, 255]),
                (MaterialMuted, [178, 162, 139, 255]),
                (MaterialCheckerDark, [70, 65, 60, 255]),
                (MaterialCheckerLight, [110, 101, 91, 255]),
            ]
        } {
            colors.insert(role, color);
        }
        // Connections need stronger contrast than the quiet construction grid.
        for (role, color) in if light {
            [
                (MaterialWire, [117, 80, 44, 255]),
                (MaterialGrid, [215, 207, 194, 255]),
                (NodeConnection, [117, 80, 44, 255]),
            ]
        } else {
            [
                (MaterialWire, [186, 159, 117, 255]),
                (MaterialGrid, [54, 48, 42, 255]),
                (NodeConnection, [186, 159, 117, 255]),
            ]
        } {
            colors.insert(role, color);
        }
        // Surface roles are explicit: text-colored cream must never fill a panel.
        for (role, color) in if light {
            [
                (TextLayoutBackground, [239, 235, 228, 255]),
                (ListLayoutBackground, [239, 235, 228, 255]),
                (TextEditBackground, [252, 249, 244, 255]),
                (TextEditTextColor, [48, 43, 38, 255]),
                (SectionbarNormalTextColor, [48, 43, 38, 255]),
                (EditorCanvasBackground, [238, 234, 226, 255]),
                (EditorGrid, [199, 191, 178, 255]),
                (EditorSubGrid, [222, 215, 204, 255]),
            ]
        } else {
            [
                (TextLayoutBackground, [35, 32, 29, 255]),
                (ListLayoutBackground, [35, 32, 29, 255]),
                (TextEditBackground, [52, 46, 41, 255]),
                (TextEditTextColor, [232, 216, 181, 255]),
                (SectionbarNormalTextColor, [232, 216, 181, 255]),
                (EditorCanvasBackground, [29, 26, 24, 255]),
                (EditorGrid, [65, 57, 49, 255]),
                (EditorSubGrid, [45, 39, 34, 255]),
            ]
        } {
            colors.insert(role, color);
        }
        Self {
            palette_base: base,
            colors,
            temp: BLACK,
            light,
        }
    }
    fn color(&self, role: TheThemeColors) -> &RGBA {
        self.colors.get(&role).expect("theme role missing")
    }
    fn palette_color(&self, palette: TheThemePalettes, index: usize) -> RGBA {
        let base = self.palette_base.palette_color(palette, index);
        if palette == TheThemePalettes::MaterialCategories {
            return base;
        }
        // Action-group rows are surfaces, so keep their category tint subdued
        // and in the same luminance range as the theme's normal tree rows.
        let surface = if self.light {
            [237, 231, 221]
        } else {
            [51, 43, 36]
        };
        std::array::from_fn(|i| {
            if i == 3 {
                255
            } else {
                ((surface[i] as u16 * 3 + base[i] as u16) / 4) as u8
            }
        })
    }
    fn disabled(&mut self, role: TheThemeColors, alpha: bool) -> &RGBA {
        let mut c = *self.color(role);
        if alpha {
            c[3] = (c[3] as f32 * 0.6) as u8;
        } else {
            let target = if self.light { 240.0 } else { 45.0 };
            for channel in &mut c[..3] {
                *channel = (*channel as f32 * 0.55 + target * 0.45) as u8;
            }
        }
        self.temp = c;
        &self.temp
    }
}

pub struct TheBrownTheme(CreatorPalette);
pub struct TheLightTheme(CreatorPalette);
macro_rules! impl_theme {
    ($ty:ident, $light:expr) => {
        impl TheTheme for $ty {
            fn new() -> Self {
                Self(CreatorPalette::new($light))
            }
            fn color(&self, role: TheThemeColors) -> &RGBA {
                self.0.color(role)
            }
            fn palette_color(&self, palette: TheThemePalettes, index: usize) -> RGBA {
                self.0.palette_color(palette, index)
            }
            fn color_disabled_switch(&mut self, role: TheThemeColors, disabled: bool) -> &RGBA {
                if disabled {
                    self.0.disabled(role, false)
                } else {
                    self.0.color(role)
                }
            }
            fn color_disabled(&mut self, role: TheThemeColors) -> &RGBA {
                self.0.disabled(role, false)
            }
            fn color_disabled_t(&mut self, role: TheThemeColors) -> &RGBA {
                self.0.disabled(role, true)
            }
        }
    };
}
impl_theme!(TheBrownTheme, false);
impl_theme!(TheLightTheme, true);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_connections_are_distinct_from_the_canvas_and_grid() {
        let luminance = |c: RGBA| {
            let linear = |v: u8| {
                let v = v as f32 / 255.;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            linear(c[0]) * 0.2126 + linear(c[1]) * 0.7152 + linear(c[2]) * 0.0722
        };
        for theme in [
            Box::new(TheBrownTheme::new()) as Box<dyn TheTheme>,
            Box::new(TheLightTheme::new()),
        ] {
            let wire = luminance(*theme.color(MaterialWire));
            for (role, minimum) in [(MaterialCanvas, 4.5), (MaterialGrid, 3.0)] {
                let surface = luminance(*theme.color(role));
                let contrast = (wire.max(surface) + 0.05) / (wire.min(surface) + 0.05);
                assert!(contrast >= minimum, "{role:?}: {contrast}");
            }
        }
    }

    #[test]
    fn tree_category_surfaces_have_readable_text_in_both_themes() {
        fn luminance(color: RGBA) -> f32 {
            let channel = |v: u8| {
                let v = v as f32 / 255.;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            channel(color[0]) * 0.2126 + channel(color[1]) * 0.7152 + channel(color[2]) * 0.0722
        }
        for theme in [
            Box::new(TheBrownTheme::new()) as Box<dyn TheTheme>,
            Box::new(TheLightTheme::new()),
        ] {
            let text = luminance(*theme.color(ListItemText));
            let header = luminance(*theme.color(SectionbarNormalTextColor));
            for index in 0..24 {
                let background =
                    luminance(theme.palette_color(TheThemePalettes::ActionGroups, index));
                for foreground in [text, header] {
                    let contrast =
                        (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05);
                    assert!(contrast >= 4.5, "slot {index}: contrast {contrast}");
                }
            }
        }
    }
    #[test]
    fn themes_define_every_role_and_have_readable_text() {
        for theme in [
            Box::new(TheBrownTheme::new()) as Box<dyn TheTheme>,
            Box::new(TheLightTheme::new()),
        ] {
            for &role in ROLES {
                let _ = theme.color(role);
            }
            let brightness = |role| {
                let c = theme.color(role);
                u16::from(c[0]) + u16::from(c[1]) + u16::from(c[2])
            };
            assert!(brightness(MenuText).abs_diff(brightness(DefaultWidgetBackground)) > 250);
            assert!(brightness(TextEditTextColor).abs_diff(brightness(TextEditBackground)) > 250);
        }
    }
}
