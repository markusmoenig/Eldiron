use crate::prelude::*;

/// Near-black theme with deep blue selection and a brighter blue focus ring.
///
/// It inherits unmigrated component roles from `TheDarkTheme`, allowing PNG-backed widgets to be
/// converted independently while the theme remains usable throughout the transition.
pub struct TheBlackBlueTheme {
    base: TheDarkTheme,
    overrides: FxHashMap<TheThemeColors, RGBA>,
    palette_overrides: FxHashMap<(TheThemePalettes, usize), RGBA>,
    temp_color: RGBA,
}

impl TheBlackBlueTheme {
    pub fn set_color(&mut self, role: TheThemeColors, color: RGBA) {
        self.overrides.insert(role, color);
    }

    pub fn set_palette_color(&mut self, palette: TheThemePalettes, index: usize, color: RGBA) {
        self.palette_overrides.insert((palette, index), color);
    }
}

impl TheTheme for TheBlackBlueTheme {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut overrides = FxHashMap::default();
        for (role, color) in [
            (Green, [220, 255, 0, 255]),
            (DefaultWidgetBackground, [39, 41, 44, 255]),
            (DefaultWidgetDarkBackground, [12, 13, 15, 255]),
            (DefaultWidgetBorder, [95, 98, 102, 255]),
            (DefaultSelection, [57, 75, 105, 255]),
            (SelectedTextEditBorder1, [83, 151, 207, 255]),
            (SelectedTextEditBorder2, [57, 75, 105, 255]),
            (TextEditBackground, [49, 51, 54, 255]),
            (TextEditRange, [57, 75, 105, 255]),
            (TextEditBorder, [102, 105, 109, 255]),
            (TextEditCursorColor, [83, 151, 207, 255]),
            (ListLayoutBackground, [12, 13, 15, 255]),
            (ListLayoutBorder, [78, 81, 85, 255]),
            (ListItemNormal, [53, 55, 58, 255]),
            (ListItemSelected, [57, 75, 105, 255]),
            (ListItemSelectedNoFocus, [67, 70, 74, 255]),
            (ListItemHover, [82, 86, 91, 255]),
            (ListItemText, [238, 240, 242, 255]),
            (TabbarBackground, [12, 13, 15, 255]),
            (TabbarConnector, [57, 75, 105, 255]),
            (SectionbarHeaderBorder, [67, 70, 74, 255]),
            (SectionbarBackground, [31, 33, 36, 255]),
            (SectionbarNormalTextColor, [238, 240, 242, 255]),
            (SectionbarSelectedTextColor, [244, 247, 250, 255]),
            (ToolbarButtonNormal, [39, 41, 44, 255]),
            (ToolbarButtonNormalBorder, [86, 89, 94, 255]),
            (ToolbarButtonHover, [62, 66, 71, 255]),
            (ToolbarButtonHoverBorder, [129, 133, 139, 255]),
            (ToolbarButtonClicked, [55, 72, 101, 255]),
            (ToolbarButtonClickedBorder, [91, 120, 163, 255]),
            (ToolListButtonNormalBorder, [67, 70, 74, 255]),
            (ToolListButtonHoverBorder, [112, 118, 126, 255]),
            (ToolListButtonSelectedBorder, [83, 151, 207, 255]),
            (ToolListButtonHoverBackground, [55, 58, 63, 255]),
            (ToolListButtonSelectedBackground, [57, 75, 105, 255]),
            (StatusbarStart, [70, 73, 77, 255]),
            (StatusbarEnd, [9, 10, 12, 255]),
            (ContextMenuBackground, [28, 29, 32, 255]),
            (ContextMenuBorder, [93, 96, 100, 255]),
            (ContextMenuHighlight, [57, 75, 105, 255]),
            (ContextMenuTextNormal, [238, 240, 242, 255]),
            (ContextMenuTextDisabled, [115, 118, 122, 255]),
            (ContextMenuTextHighlight, [244, 247, 250, 255]),
            (ContextMenuSeparator, [73, 76, 80, 255]),
            (WindowBorderOuter, [81, 84, 88, 255]),
            (WindowBorderInner, [122, 125, 130, 255]),
            (WindowHeaderBackground, [45, 47, 50, 255]),
            (NodeBody, [20, 21, 23, 255]),
            (NodeBodySelected, [30, 38, 51, 255]),
            (NodeBorder, [80, 83, 87, 255]),
            (NodeBorderSelected, [83, 151, 207, 255]),
            (TextLayoutBackground, [18, 19, 21, 255]),
            (TextLayoutBorder, [72, 75, 79, 255]),
            (TextLinkColor, [111, 174, 226, 255]),
            (TextLinkHoveredColor, [151, 202, 242, 255]),
            (MenubarPopupBackground, [28, 29, 32, 255]),
            (MenubarPopupBorder, [93, 96, 100, 255]),
            (SliderSmallColor1, [48, 51, 55, 255]),
            (SliderSmallColor2, [83, 151, 207, 255]),
            (SliderSmallColor3, [126, 181, 225, 255]),
            (SliderSmallColor4, [25, 27, 30, 255]),
            (MenubarButtonHover, [55, 58, 62, 255]),
            (MenubarButtonHoverBorder, [112, 118, 126, 255]),
            (MenubarButtonClicked, [57, 75, 105, 255]),
            (MenubarButtonClickedBorder, [83, 151, 207, 255]),
            (MenubarButtonSeparator1, [42, 44, 47, 255]),
            (MenubarButtonSeparator2, [79, 82, 86, 255]),
            (TraybarButtonNormal, [39, 41, 44, 255]),
            (TraybarButtonNormalBorder, [78, 81, 85, 255]),
            (TraybarButtonHover, [62, 66, 71, 255]),
            (TraybarButtonHoverBorder, [129, 133, 139, 255]),
            (TraybarButtonClicked, [55, 72, 101, 255]),
            (TraybarButtonClickedBorder, [91, 120, 163, 255]),
            (TraybarButtonDisabledBorder, [52, 54, 57, 255]),
            (TraybarButtonDisabledBackground, [29, 31, 34, 255]),
            (ListItemIconBorder, [78, 81, 85, 255]),
            (ScrollbarBackground, [17, 18, 20, 255]),
            (ScrollbarSeparator, [61, 64, 68, 255]),
            (TabbarText, [238, 240, 242, 255]),
            (TraybarBorder, [77, 80, 84, 255]),
            (TraybarBackground, [26, 28, 31, 255]),
            (TraybarBottomBorder, [8, 9, 11, 255]),
            (DividerStart, [70, 73, 77, 255]),
            (DividerEnd, [17, 18, 20, 255]),
            (GroupButtonNormalBorder, [78, 81, 85, 255]),
            (GroupButtonNormalBackground, [39, 41, 44, 255]),
            (GroupButtonHoverBorder, [129, 133, 139, 255]),
            (GroupButtonHoverBackground, [62, 66, 71, 255]),
            (GroupButtonSelectedBorder, [83, 151, 207, 255]),
            (GroupButtonSelectedBackground, [57, 75, 105, 255]),
            (CodeGridBackground, [18, 19, 21, 255]),
            (CodeGridNormal, [53, 55, 58, 255]),
            (CodeGridDark, [12, 13, 15, 255]),
            (CodeGridSelected, [57, 75, 105, 255]),
            (CodeGridHover, [82, 86, 91, 255]),
            (CodeGridText, [238, 240, 242, 255]),
            (DropItemBackground, [39, 41, 44, 255]),
            (DropItemBorder, [95, 98, 102, 255]),
            (DropItemText, [238, 240, 242, 255]),
            (WindowHeaderBorder1, [90, 93, 97, 255]),
            (WindowHeaderBorder2, [56, 59, 63, 255]),
            (TimeSliderBorder, [82, 86, 91, 255]),
            (TimeSliderBackground, [31, 33, 36, 255]),
            (TimeSliderText, [213, 217, 221, 255]),
            (TimeSliderMarker, [83, 151, 207, 255]),
            (TimeSliderLine, [68, 71, 75, 255]),
            (TimeSliderPosition, [126, 181, 225, 255]),
            (NodeCanvasBackground, [128, 128, 128, 255]),
            (NodeCanvasGrid, [74, 74, 74, 255]),
            (NodeConnection, [90, 90, 90, 255]),
            (NodeCutConnection, [209, 42, 42, 255]),
            (MenuText, [213, 217, 221, 255]),
            (MenuTextHighlighted, [244, 247, 250, 255]),
            (MenuHover, [55, 58, 62, 255]),
            (MenuSelected, [57, 75, 105, 255]),
            (TreeViewNodeBorder, [68, 71, 75, 255]),
            (TreeViewNode, [31, 33, 36, 255]),
            (TreeViewNodeSelectedBorder, [83, 151, 207, 255]),
            (TreeViewNodeSelected, [57, 75, 105, 255]),
            (TreeViewNodeText, [238, 240, 242, 255]),
            (TreeViewNodePlusMinus, [213, 217, 221, 255]),
            (LayoutSeparator, [69, 72, 76, 255]),
        ] {
            overrides.insert(role, color);
        }

        Self {
            base: TheDarkTheme::new(),
            overrides,
            palette_overrides: FxHashMap::default(),
            temp_color: BLACK,
        }
    }

    fn color(&self, of: TheThemeColors) -> &RGBA {
        self.overrides
            .get(&of)
            .unwrap_or_else(|| self.base.color(of))
    }

    fn paint(&self, of: TheThemePaints, bounds: ThePixelRect) -> ThePaint {
        let top = bounds.y as f32;
        let bottom = bounds.y.saturating_add(bounds.height) as f32;
        match of {
            TheThemePaints::MenuBackground => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [55, 58, 62, 255],
                [15, 16, 18, 255],
            ),
            TheThemePaints::MenuItemHover => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [76, 80, 85, 255],
                [34, 36, 39, 255],
            ),
            TheThemePaints::MenuItemSelected => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [75, 94, 127, 255],
                [42, 56, 79, 255],
            ),
            TheThemePaints::MenubarBackground => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [67, 69, 72, 255],
                [8, 9, 10, 255],
            ),
            TheThemePaints::MenubarButtonHoverChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [80, 84, 89, 255],
                [37, 39, 43, 255],
            ),
            TheThemePaints::MenubarButtonPressedChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [75, 94, 127, 255],
                [42, 56, 79, 255],
            ),
            TheThemePaints::NodeHeaderNormalChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [54, 57, 61, 255],
                [24, 26, 29, 255],
            ),
            TheThemePaints::NodeHeaderSelectedChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [66, 85, 116, 255],
                [34, 46, 65, 255],
            ),
            TheThemePaints::NodeBodyNormalChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [34, 36, 39, 255],
                [20, 21, 23, 255],
            ),
            TheThemePaints::NodeBodySelectedChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [39, 49, 66, 255],
                [25, 32, 44, 255],
            ),
            TheThemePaints::NodeFooterNormalChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [32, 34, 37, 255],
                [15, 16, 18, 255],
            ),
            TheThemePaints::NodeFooterSelectedChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [35, 46, 64, 255],
                [20, 27, 38, 255],
            ),
            TheThemePaints::NodePreviewBackground => ThePaint::solid([13, 14, 16, 255]),
            TheThemePaints::ToolbarBackground | TheThemePaints::SwitchbarChrome => {
                ThePaint::linear_gradient(
                    [0.0, top],
                    [0.0, bottom],
                    [74, 77, 81, 255],
                    [7, 8, 10, 255],
                )
            }
            TheThemePaints::StatusbarBackground => ThePaint::solid([26, 28, 31, 255]),
            TheThemePaints::TimeSliderBackgroundChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [43, 46, 50, 255],
                [22, 24, 27, 255],
            ),
            TheThemePaints::TimeSliderMarkerChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [126, 181, 225, 255],
                [57, 75, 105, 255],
            ),
            TheThemePaints::TimeSliderPositionChrome => ThePaint::solid([126, 181, 225, 255]),
            TheThemePaints::SectionbarChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [55, 58, 62, 255],
                [20, 21, 23, 255],
            ),
            TheThemePaints::TabbarChrome => ThePaint::solid([12, 13, 15, 255]),
            TheThemePaints::TabNormalChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [53, 55, 58, 255],
                [20, 21, 23, 255],
            ),
            TheThemePaints::TabHoverChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [76, 80, 85, 255],
                [34, 36, 39, 255],
            ),
            TheThemePaints::TabSelectedChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [75, 91, 119, 255],
                [43, 56, 78, 255],
            ),
            TheThemePaints::SectionButtonNormal => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [62, 65, 69, 255],
                [28, 30, 33, 255],
            ),
            TheThemePaints::SectionButtonHover => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [83, 87, 92, 255],
                [42, 45, 49, 255],
            ),
            TheThemePaints::SectionButtonSelected => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [80, 98, 130, 255],
                [46, 60, 83, 255],
            ),
            TheThemePaints::ToolListBarChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [68, 71, 75, 255],
                [20, 21, 23, 255],
            ),
            TheThemePaints::DropdownNormal => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [58, 61, 65, 255],
                [24, 26, 29, 255],
            ),
            TheThemePaints::DropdownHover => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [79, 83, 89, 255],
                [38, 41, 45, 255],
            ),
            TheThemePaints::DropdownFocus => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [68, 76, 88, 255],
                [29, 35, 44, 255],
            ),
            TheThemePaints::DropdownPressed => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [69, 87, 118, 255],
                [39, 52, 74, 255],
            ),
            TheThemePaints::DropdownDisabled => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [39, 41, 44, 255],
                [23, 24, 26, 255],
            ),
            TheThemePaints::DropdownMarker => ThePaint::solid([190, 195, 201, 255]),
            TheThemePaints::TextInputNormal => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [58, 60, 63, 255],
                [43, 45, 48, 255],
            ),
            TheThemePaints::TextInputFocused => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [61, 64, 68, 255],
                [43, 46, 50, 255],
            ),
            TheThemePaints::TextInputDisabled => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [45, 46, 48, 255],
                [34, 35, 37, 255],
            ),
            TheThemePaints::CheckboxNormal => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [67, 70, 74, 255],
                [28, 30, 33, 255],
            ),
            TheThemePaints::CheckboxHover => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [78, 84, 91, 255],
                [39, 44, 50, 255],
            ),
            TheThemePaints::CheckboxSelected => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [78, 103, 142, 255],
                [45, 61, 87, 255],
            ),
            TheThemePaints::CheckboxMark => ThePaint::solid([238, 242, 246, 255]),
            TheThemePaints::ScrollbarTrack => ThePaint::solid([17, 18, 20, 255]),
            TheThemePaints::ScrollbarThumbNormal => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [106, 110, 115, 255],
                [58, 61, 65, 255],
            ),
            TheThemePaints::ScrollbarThumbHover => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [135, 141, 148, 255],
                [75, 80, 86, 255],
            ),
            TheThemePaints::ScrollbarThumbPressed => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [91, 120, 163, 255],
                [49, 66, 94, 255],
            ),
            TheThemePaints::SnapperNormal => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [61, 64, 68, 255],
                [25, 27, 30, 255],
            ),
            TheThemePaints::SnapperHover => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [82, 86, 91, 255],
                [39, 42, 46, 255],
            ),
            TheThemePaints::SnapperPressed => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [75, 94, 127, 255],
                [42, 56, 79, 255],
            ),
            TheThemePaints::SnapperSelected => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [72, 93, 128, 255],
                [45, 60, 85, 255],
            ),
            TheThemePaints::SnapperMarker => ThePaint::solid([229, 233, 237, 255]),
            TheThemePaints::SliderTrackChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [52, 55, 59, 255],
                [24, 26, 29, 255],
            ),
            TheThemePaints::SliderTrackAccent => ThePaint::solid([83, 88, 94, 255]),
            TheThemePaints::SliderThumbNormal => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [116, 121, 127, 255],
                [57, 60, 64, 255],
            ),
            TheThemePaints::SliderThumbHover => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [126, 181, 225, 255],
                [57, 75, 105, 255],
            ),
            TheThemePaints::SliderThumbPressed => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [91, 120, 163, 255],
                [42, 56, 79, 255],
            ),
            TheThemePaints::TrayButtonNormal => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [58, 61, 65, 255],
                [28, 30, 33, 255],
            ),
            TheThemePaints::TrayButtonHover => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [80, 84, 89, 255],
                [40, 43, 47, 255],
            ),
            TheThemePaints::TrayButtonPressed => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [75, 94, 127, 255],
                [42, 56, 79, 255],
            ),
            TheThemePaints::TrayButtonDisabled => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [39, 41, 44, 255],
                [23, 24, 26, 255],
            ),
            TheThemePaints::ControlNormal => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [61, 64, 68, 255],
                [29, 31, 34, 255],
            ),
            TheThemePaints::ControlHover => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [85, 89, 94, 255],
                [44, 47, 51, 255],
            ),
            TheThemePaints::ControlPressed => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [75, 94, 127, 255],
                [42, 56, 79, 255],
            ),
            TheThemePaints::PopupFrame => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [129, 133, 139, 255],
                [45, 47, 50, 255],
            ),
            TheThemePaints::PopupBody => ThePaint::solid([18, 19, 21, 255]),
            TheThemePaints::PopupHeader => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [67, 70, 74, 255],
                [24, 26, 29, 255],
            ),
            TheThemePaints::PopupSeparator => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [126, 181, 225, 255],
                [57, 75, 105, 255],
            ),
            TheThemePaints::PopoverShadow => ThePaint::solid([0, 0, 0, 128]),
            TheThemePaints::PopoverFrame => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [78, 81, 86, 255],
                [42, 44, 47, 255],
            ),
            TheThemePaints::PopoverBody => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                [34, 36, 39, 255],
                [18, 19, 21, 255],
            ),
            _ => default_paint(self, of, bounds),
        }
    }

    fn palette_color(&self, of: TheThemePalettes, index: usize) -> RGBA {
        const ACTION_GROUPS: [RGBA; 12] = [
            [57, 75, 105, 255],
            [70, 52, 48, 255],
            [48, 51, 55, 255],
            [43, 67, 56, 255],
            [65, 52, 78, 255],
            [75, 63, 40, 255],
            [42, 65, 75, 255],
            [76, 47, 54, 255],
            [57, 65, 42, 255],
            [47, 54, 78, 255],
            [78, 56, 40, 255],
            [61, 45, 72, 255],
        ];
        self.palette_overrides
            .get(&(of, index))
            .copied()
            .unwrap_or_else(|| match of {
                TheThemePalettes::ActionGroups => ACTION_GROUPS[index % ACTION_GROUPS.len()],
            })
    }

    fn color_disabled_switch(&mut self, of: TheThemeColors, disabled: bool) -> &RGBA {
        if disabled {
            self.color_disabled(of)
        } else {
            self.color(of)
        }
    }

    fn color_disabled(&mut self, of: TheThemeColors) -> &RGBA {
        let mut color = *self.color(of);
        for channel in &mut color[..3] {
            *channel = (*channel as f32 * 0.65).round() as u8;
        }
        self.temp_color = color;
        &self.temp_color
    }

    fn color_disabled_t(&mut self, of: TheThemeColors) -> &RGBA {
        let mut color = *self.color(of);
        color[3] = (color[3] as f32 * 0.65).round() as u8;
        self.temp_color = color;
        &self.temp_color
    }
}

fn default_paint(theme: &dyn TheTheme, role: TheThemePaints, bounds: ThePixelRect) -> ThePaint {
    let top = bounds.y as f32;
    let bottom = bounds.y.saturating_add(bounds.height) as f32;
    match role {
        TheThemePaints::MenuBackground | TheThemePaints::MenubarBackground => {
            ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                *theme.color(DefaultWidgetBackground),
                *theme.color(DefaultWidgetDarkBackground),
            )
        }
        TheThemePaints::MenuItemHover | TheThemePaints::MenubarButtonHoverChrome => {
            ThePaint::solid(*theme.color(MenubarButtonHover))
        }
        TheThemePaints::MenuItemSelected | TheThemePaints::MenubarButtonPressedChrome => {
            ThePaint::solid(*theme.color(MenubarButtonClicked))
        }
        TheThemePaints::NodeHeaderNormalChrome
        | TheThemePaints::NodeBodyNormalChrome
        | TheThemePaints::NodeFooterNormalChrome => ThePaint::solid(*theme.color(NodeBody)),
        TheThemePaints::NodeHeaderSelectedChrome
        | TheThemePaints::NodeBodySelectedChrome
        | TheThemePaints::NodeFooterSelectedChrome => {
            ThePaint::solid(*theme.color(NodeBodySelected))
        }
        TheThemePaints::NodePreviewBackground => {
            ThePaint::solid(*theme.color(DefaultWidgetDarkBackground))
        }
        TheThemePaints::ToolbarBackground => ThePaint::linear_gradient(
            [0.0, top],
            [0.0, bottom],
            *theme.color(DefaultWidgetBackground),
            *theme.color(DefaultWidgetDarkBackground),
        ),
        TheThemePaints::StatusbarBackground => ThePaint::solid(*theme.color(StatusbarEnd)),
        TheThemePaints::SwitchbarChrome => ThePaint::linear_gradient(
            [0.0, top],
            [0.0, bottom],
            *theme.color(DefaultWidgetBackground),
            *theme.color(DefaultWidgetDarkBackground),
        ),
        TheThemePaints::SwitchbarMarker => ThePaint::solid(*theme.color(SectionbarNormalTextColor)),
        TheThemePaints::SectionbarChrome => ThePaint::solid(*theme.color(SectionbarBackground)),
        TheThemePaints::TabbarChrome => ThePaint::solid(*theme.color(TabbarBackground)),
        TheThemePaints::TabNormalChrome => {
            ThePaint::solid(*theme.color(DefaultWidgetDarkBackground))
        }
        TheThemePaints::TabHoverChrome => ThePaint::solid(*theme.color(ToolbarButtonHover)),
        TheThemePaints::TabSelectedChrome => ThePaint::solid(*theme.color(DefaultSelection)),
        TheThemePaints::SectionButtonNormal => ThePaint::solid(*theme.color(SectionbarBackground)),
        TheThemePaints::SectionButtonHover => {
            ThePaint::solid(*theme.color(ToolListButtonHoverBackground))
        }
        TheThemePaints::SectionButtonSelected => ThePaint::solid(*theme.color(DefaultSelection)),
        TheThemePaints::ToolListBarChrome => ThePaint::linear_gradient(
            [0.0, top],
            [0.0, bottom],
            *theme.color(DefaultWidgetBackground),
            *theme.color(DefaultWidgetDarkBackground),
        ),
        TheThemePaints::DropdownNormal => ThePaint::solid(*theme.color(TextEditBackground)),
        TheThemePaints::DropdownHover => ThePaint::solid(*theme.color(ToolbarButtonHover)),
        TheThemePaints::DropdownFocus => ThePaint::solid(*theme.color(TextEditBackground)),
        TheThemePaints::DropdownPressed => ThePaint::solid(*theme.color(ToolbarButtonClicked)),
        TheThemePaints::DropdownDisabled => {
            ThePaint::solid(*theme.color(DefaultWidgetDarkBackground))
        }
        TheThemePaints::DropdownMarker => ThePaint::solid(*theme.color(SectionbarNormalTextColor)),
        TheThemePaints::TextInputNormal => ThePaint::solid(*theme.color(TextEditBackground)),
        TheThemePaints::TextInputFocused => ThePaint::solid(*theme.color(TextEditBackground)),
        TheThemePaints::TextInputDisabled => {
            ThePaint::solid(*theme.color(DefaultWidgetDarkBackground))
        }
        TheThemePaints::CheckboxNormal => {
            ThePaint::solid(*theme.color(DefaultWidgetDarkBackground))
        }
        TheThemePaints::CheckboxHover => ThePaint::solid(*theme.color(ToolbarButtonHover)),
        TheThemePaints::CheckboxSelected => ThePaint::solid(*theme.color(DefaultSelection)),
        TheThemePaints::CheckboxMark => ThePaint::solid(*theme.color(SectionbarSelectedTextColor)),
        TheThemePaints::ScrollbarTrack => ThePaint::solid(*theme.color(ScrollbarBackground)),
        TheThemePaints::ScrollbarThumbNormal => ThePaint::solid(*theme.color(ToolbarButtonNormal)),
        TheThemePaints::ScrollbarThumbHover => ThePaint::solid(*theme.color(ToolbarButtonHover)),
        TheThemePaints::ScrollbarThumbPressed => {
            ThePaint::solid(*theme.color(ToolbarButtonClicked))
        }
        TheThemePaints::SnapperNormal => ThePaint::solid(*theme.color(SectionbarBackground)),
        TheThemePaints::SnapperHover => ThePaint::solid(*theme.color(ToolbarButtonHover)),
        TheThemePaints::SnapperPressed => ThePaint::solid(*theme.color(ToolbarButtonClicked)),
        TheThemePaints::SnapperSelected => ThePaint::solid(*theme.color(DefaultSelection)),
        TheThemePaints::SnapperMarker => ThePaint::solid(*theme.color(SectionbarNormalTextColor)),
        TheThemePaints::SliderTrackChrome => ThePaint::solid(*theme.color(SliderSmallColor4)),
        TheThemePaints::SliderTrackAccent => ThePaint::solid(*theme.color(SliderSmallColor2)),
        TheThemePaints::SliderThumbNormal => ThePaint::solid(*theme.color(SliderSmallColor1)),
        TheThemePaints::SliderThumbHover | TheThemePaints::SliderThumbPressed => {
            ThePaint::solid(*theme.color(SliderSmallColor3))
        }
        TheThemePaints::TimeSliderBackgroundChrome => {
            ThePaint::solid(*theme.color(TimeSliderBackground))
        }
        TheThemePaints::TimeSliderMarkerChrome => ThePaint::solid(*theme.color(TimeSliderMarker)),
        TheThemePaints::TimeSliderPositionChrome => {
            ThePaint::solid(*theme.color(TimeSliderPosition))
        }
        TheThemePaints::TrayButtonNormal => ThePaint::solid(*theme.color(TraybarButtonNormal)),
        TheThemePaints::TrayButtonHover => ThePaint::solid(*theme.color(TraybarButtonHover)),
        TheThemePaints::TrayButtonPressed => ThePaint::solid(*theme.color(TraybarButtonClicked)),
        TheThemePaints::TrayButtonDisabled => {
            ThePaint::solid(*theme.color(TraybarButtonDisabledBackground))
        }
        TheThemePaints::ControlNormal => ThePaint::solid(*theme.color(ToolbarButtonNormal)),
        TheThemePaints::ControlHover => ThePaint::solid(*theme.color(ToolbarButtonHover)),
        TheThemePaints::ControlPressed => ThePaint::solid(*theme.color(ToolbarButtonClicked)),
        TheThemePaints::PopupFrame => ThePaint::solid(*theme.color(WindowBorderOuter)),
        TheThemePaints::PopupBody => ThePaint::solid(*theme.color(ListLayoutBackground)),
        TheThemePaints::PopupHeader => ThePaint::linear_gradient(
            [0.0, top],
            [0.0, bottom],
            *theme.color(WindowHeaderBackground),
            *theme.color(DefaultWidgetDarkBackground),
        ),
        TheThemePaints::PopupSeparator => ThePaint::solid(*theme.color(WindowHeaderBorder1)),
        TheThemePaints::PopoverShadow => ThePaint::solid([0, 0, 0, 96]),
        TheThemePaints::PopoverFrame => ThePaint::solid(*theme.color(WindowBorderOuter)),
        TheThemePaints::PopoverBody => ThePaint::solid(*theme.color(ListLayoutBackground)),
        TheThemePaints::Selection => ThePaint::solid(*theme.color(DefaultSelection)),
        TheThemePaints::Focus => ThePaint::solid(*theme.color(SelectedTextEditBorder1)),
        TheThemePaints::Accent => ThePaint::solid(*theme.color(Green)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_focus_and_accent_are_independent_roles() {
        let theme = TheBlackBlueTheme::new();
        assert_eq!(*theme.color(DefaultSelection), [57, 75, 105, 255]);
        assert_eq!(*theme.color(SelectedTextEditBorder1), [83, 151, 207, 255]);
        assert_eq!(*theme.color(Green), [220, 255, 0, 255]);
        assert_ne!(
            theme.color(DefaultSelection),
            theme.color(SelectedTextEditBorder1)
        );
    }

    #[test]
    fn individual_legacy_roles_remain_customizable() {
        let mut theme = TheBlackBlueTheme::new();
        theme.set_color(ListItemSelected, [1, 2, 3, 255]);
        assert_eq!(*theme.color(ListItemSelected), [1, 2, 3, 255]);
        assert_eq!(theme.metric(ControlCornerRadius), 2.0);
    }

    #[test]
    fn creator_facing_legacy_controls_do_not_fall_back_to_light_gray() {
        let theme = TheBlackBlueTheme::new();
        for role in [
            TraybarBackground,
            TraybarButtonNormal,
            GroupButtonNormalBackground,
            TimeSliderBackground,
            TreeViewNode,
            CodeGridBackground,
        ] {
            let color = theme.color(role);
            assert!(
                color[0] < 64 && color[1] < 64 && color[2] < 64,
                "{role:?} unexpectedly remained light: {color:?}"
            );
        }
    }

    #[test]
    fn action_group_palette_is_extensible_and_customizable() {
        let mut theme = TheBlackBlueTheme::new();
        let original = theme.palette_color(ActionGroups, 6);
        assert_ne!(original, theme.palette_color(ActionGroups, 0));

        theme.set_palette_color(ActionGroups, 12, [1, 2, 3, 255]);
        assert_eq!(theme.palette_color(ActionGroups, 12), [1, 2, 3, 255]);

        // Unconfigured future slots remain safe and deterministic.
        let future = theme.palette_color(ActionGroups, 21);
        assert_eq!(future, theme.palette_color(ActionGroups, 21));
        assert_eq!(future[3], 255);
    }

    #[test]
    fn statusbar_uses_a_flat_paint() {
        let theme = TheBlackBlueTheme::new();
        assert_eq!(
            theme.paint(StatusbarBackground, ThePixelRect::new(0, 0, 200, 21)),
            ThePaint::solid([26, 28, 31, 255])
        );
    }
}
