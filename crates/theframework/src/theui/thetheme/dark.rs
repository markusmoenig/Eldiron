use crate::prelude::*;

use super::TheThemeColors;

pub struct TheDarkTheme {
    temp_color: RGBA,
    colors: FxHashMap<TheThemeColors, RGBA>,
}

/// Implements TheDarkTheme
impl TheTheme for TheDarkTheme {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut colors = FxHashMap::default();

        colors.insert(Green, [10, 245, 5, 255]);
        colors.insert(Red, [209, 42, 42, 255]);

        colors.insert(DefaultWidgetBackground, [116, 116, 116, 255]);
        colors.insert(DefaultWidgetDarkBackground, [82, 82, 82, 255]);
        colors.insert(DefaultWidgetBorder, [146, 146, 146, 255]);
        colors.insert(DefaultSelection, [187, 122, 208, 255]);

        colors.insert(SwitchbarBorder, [86, 86, 86, 255]);

        colors.insert(SectionbarHeaderBorder, [86, 86, 86, 255]);
        colors.insert(SectionbarBackground, [130, 130, 130, 255]);
        colors.insert(SectionbarNormalTextColor, [255, 255, 255, 255]);
        colors.insert(SectionbarSelectedTextColor, [96, 96, 96, 255]);

        colors.insert(TextLayoutBackground, [82, 82, 82, 255]);
        colors.insert(TextLayoutBorder, [139, 139, 139, 255]);

        colors.insert(TextEditBackground, [148, 148, 148, 255]);
        colors.insert(TextEditRange, [178, 178, 178, 255]);
        colors.insert(SelectedTextEditBorder1, [202, 113, 230, 255]);
        colors.insert(SelectedTextEditBorder2, [187, 122, 208, 255]);
        colors.insert(TextEditBorder, [209, 209, 209, 255]);
        colors.insert(TextEditTextColor, [242, 242, 242, 255]);
        colors.insert(TextEditCursorColor, [119, 119, 119, 255]);
        colors.insert(TextEditLineNumberColor, [219, 219, 219, 255]);
        colors.insert(TextEditLineNumberHighlightColor, [242, 242, 242, 255]);
        colors.insert(TextEditLineNumberDebugColor, [255, 214, 102, 255]);
        colors.insert(TextEditDebugLineBackground, [255, 214, 102, 80]);

        colors.insert(TextLinkColor, [0, 0, 238, 255]);
        colors.insert(TextLinkHoveredColor, [0, 0, 170, 255]);

        colors.insert(MenubarPopupBackground, [124, 124, 124, 255]);
        colors.insert(MenubarPopupBorder, [153, 153, 153, 255]);

        colors.insert(SliderSmallColor1, [158, 158, 158, 255]);
        colors.insert(SliderSmallColor2, [174, 174, 174, 255]);
        colors.insert(SliderSmallColor3, [187, 187, 187, 255]);
        colors.insert(SliderSmallColor4, [122, 122, 122, 255]);

        colors.insert(MenubarButtonHover, [157, 157, 157, 255]);
        colors.insert(MenubarButtonHoverBorder, [179, 179, 179, 255]);
        colors.insert(MenubarButtonClicked, [149, 149, 149, 255]);
        colors.insert(MenubarButtonClickedBorder, [204, 204, 204, 255]);

        colors.insert(MenubarButtonSeparator1, [102, 102, 102, 255]);
        colors.insert(MenubarButtonSeparator2, [148, 148, 148, 255]);

        colors.insert(ToolbarButtonNormal, [99, 99, 99, 255]);
        colors.insert(ToolbarButtonNormalBorder, [87, 87, 87, 255]);
        colors.insert(ToolbarButtonHover, [157, 157, 157, 255]);
        colors.insert(ToolbarButtonHoverBorder, [179, 179, 179, 255]);
        colors.insert(ToolbarButtonClicked, [149, 149, 149, 255]);
        colors.insert(ToolbarButtonClickedBorder, [204, 204, 204, 255]);

        colors.insert(TraybarButtonNormal, [123, 123, 123, 255]);
        colors.insert(TraybarButtonNormalBorder, [108, 108, 108, 255]);
        colors.insert(TraybarButtonHover, [157, 157, 157, 255]);
        colors.insert(TraybarButtonHoverBorder, [179, 179, 179, 255]);
        colors.insert(TraybarButtonClicked, [149, 149, 149, 255]);
        colors.insert(TraybarButtonClickedBorder, [204, 204, 204, 255]);
        colors.insert(TraybarButtonDisabledBorder, [111, 111, 111, 255]);
        colors.insert(TraybarButtonDisabledBackground, [119, 119, 119, 255]);

        colors.insert(ListLayoutBackground, [82, 82, 82, 255]);
        colors.insert(ListLayoutBorder, [139, 139, 139, 255]);
        colors.insert(ListItemNormal, [174, 174, 174, 255]);
        colors.insert(ListItemSelected, [187, 122, 208, 255]);
        colors.insert(ListItemSelectedNoFocus, [208, 208, 208, 255]);
        colors.insert(ListItemHover, [237, 237, 237, 255]);
        colors.insert(ListItemText, [85, 81, 85, 255]);
        colors.insert(ListItemIconBorder, [139, 139, 139, 255]);

        colors.insert(ScrollbarBackground, [139, 139, 139, 255]);
        colors.insert(ScrollbarSeparator, [119, 119, 119, 255]);

        colors.insert(TabbarBackground, [82, 82, 82, 255]);
        colors.insert(TabbarConnector, [137, 137, 137, 255]);
        colors.insert(TabbarText, [244, 244, 244, 255]);

        colors.insert(TraybarBorder, [153, 153, 153, 255]);
        colors.insert(TraybarBackground, [118, 118, 118, 255]);
        colors.insert(TraybarBottomBorder, [89, 89, 89, 255]);

        colors.insert(StatusbarStart, [84, 84, 84, 255]);
        colors.insert(StatusbarEnd, [99, 99, 99, 255]);

        colors.insert(DividerStart, [102, 102, 102, 255]);
        colors.insert(DividerEnd, [148, 148, 148, 255]);

        colors.insert(GroupButtonNormalBorder, [108, 108, 108, 255]);
        colors.insert(GroupButtonNormalBackground, [103, 103, 103, 255]);
        colors.insert(GroupButtonHoverBorder, [179, 179, 179, 255]);
        colors.insert(GroupButtonHoverBackground, [157, 157, 157, 255]);
        colors.insert(GroupButtonSelectedBorder, [204, 204, 204, 255]);
        colors.insert(GroupButtonSelectedBackground, [149, 149, 149, 255]);

        colors.insert(CodeGridBackground, [128, 128, 128, 255]);
        colors.insert(CodeGridNormal, [174, 174, 174, 255]);
        colors.insert(CodeGridDark, [74, 74, 74, 255]);
        colors.insert(CodeGridSelected, [187, 122, 208, 255]);
        colors.insert(CodeGridText, [85, 81, 85, 255]);
        colors.insert(CodeGridHover, [237, 237, 237, 255]);

        colors.insert(DropItemBackground, [174, 174, 174, 255]);
        colors.insert(DropItemBorder, [237, 237, 237, 255]);
        colors.insert(DropItemText, [85, 81, 85, 255]);

        colors.insert(ContextMenuBackground, [149, 149, 149, 255]);
        colors.insert(ContextMenuSeparator, [102, 102, 102, 255]);
        colors.insert(ContextMenuBorder, [130, 130, 130, 255]);
        colors.insert(ContextMenuHighlight, [187, 122, 208, 255]);
        colors.insert(ContextMenuTextNormal, [255, 255, 255, 255]);
        colors.insert(ContextMenuTextDisabled, [100, 100, 100, 255]);
        colors.insert(ContextMenuTextHighlight, [82, 82, 82, 255]);

        colors.insert(WindowBorderOuter, [147, 147, 147, 255]);
        colors.insert(WindowBorderInner, [197, 197, 197, 255]);
        colors.insert(WindowHeaderBackground, [148, 148, 148, 255]);
        colors.insert(WindowHeaderBorder1, [163, 163, 163, 255]);
        colors.insert(WindowHeaderBorder2, [139, 139, 139, 255]);

        colors.insert(TimeSliderBorder, [216, 216, 216, 255]);
        colors.insert(TimeSliderBackground, [184, 184, 184, 255]);
        colors.insert(TimeSliderText, [57, 57, 57, 255]);
        colors.insert(TimeSliderLine, [139, 139, 139, 255]);
        colors.insert(TimeSliderMarker, [202, 113, 230, 255]);
        colors.insert(TimeSliderPosition, [240, 240, 240, 255]);

        colors.insert(MenuHover, [40, 40, 40, 255]);
        colors.insert(MenuText, [200, 200, 200, 255]);
        colors.insert(MenuTextHighlighted, [244, 244, 244, 255]);

        colors.insert(NodeBorder, [160, 160, 160, 255]);
        colors.insert(NodeBorderSelected, [230, 230, 230, 255]);
        colors.insert(NodeBody, [84, 84, 84, 255]);
        colors.insert(NodeBodySelected, [65, 65, 65, 255]);

        colors.insert(ToolListButtonNormalBorder, [108, 108, 108, 255]);
        colors.insert(ToolListButtonHoverBorder, [170, 170, 170, 255]);
        colors.insert(ToolListButtonSelectedBorder, [204, 204, 204, 255]);
        colors.insert(ToolListButtonHoverBackground, [157, 157, 157, 255]);
        colors.insert(ToolListButtonSelectedBackground, [149, 149, 149, 255]);

        colors.insert(LayoutSeparator, [160, 160, 160, 255]);

        // colors.insert(TreeViewNodeBorder, [229, 229, 229, 255]);
        // colors.insert(TreeViewNode, [237, 237, 237, 255]);

        colors.insert(TreeViewNodeBorder, [174, 174, 174, 255]);
        colors.insert(TreeViewNode, [174, 174, 174, 255]);

        colors.insert(TreeViewNodeSelectedBorder, [202, 113, 230, 255]);
        colors.insert(TreeViewNodeSelected, [187, 122, 208, 255]);

        colors.insert(TreeViewNodeText, [52, 52, 52, 255]);

        colors.insert(TreeViewNodePlusMinus, [227, 227, 227, 255]);

        Self {
            temp_color: BLACK,
            colors,
        }
    }

    /// Return the given theme color.
    fn color(&self, of: TheThemeColors) -> &RGBA {
        self.colors.get(&of).unwrap_or(&[0, 0, 0, 255])
    }

    fn color_disabled_switch(&mut self, of: TheThemeColors, disabled: bool) -> &RGBA {
        if disabled {
            self.color_disabled(of)
        } else {
            self.color(of)
        }
    }

    /// Returns the disabled color value for the given color
    fn color_disabled(&mut self, of: TheThemeColors) -> &RGBA {
        let mut d = *self.color(of);
        d[0] = (d[0] as f32 * 0.75) as u8;
        d[1] = (d[1] as f32 * 0.75) as u8;
        d[2] = (d[2] as f32 * 0.75) as u8;
        self.temp_color = d;
        &self.temp_color
    }

    /// Returns the disabled color value for the given color
    fn color_disabled_t(&mut self, of: TheThemeColors) -> &RGBA {
        let mut d = *self.color(of);
        d[3] = (d[3] as f32 * 0.75) as u8;
        self.temp_color = d;
        &self.temp_color
    }
}
