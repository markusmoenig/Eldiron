use crate::prelude::*;

pub mod black_blue;
pub mod dark;

pub mod prelude {
    pub use crate::theui::thetheme::black_blue::TheBlackBlueTheme;
    pub use crate::theui::thetheme::dark::TheDarkTheme;
}

/// TheTheme defines all colors and other attributes of a theme.
#[allow(unused)]
pub trait TheTheme: Send {
    fn new() -> Self
    where
        Self: Sized;

    /// Returns the color of the given theme color.
    fn color(&self, of: TheThemeColors) -> &RGBA;

    /// Returns a semantic paint sized for the supplied target rectangle.
    /// Existing themes inherit sensible color-based defaults while newer themes can provide
    /// gradients without making widgets aware of their implementation.
    fn paint(&self, of: TheThemePaints, bounds: ThePixelRect) -> ThePaint {
        let top = bounds.y as f32;
        let bottom = bounds.y.saturating_add(bounds.height) as f32;
        match of {
            TheThemePaints::MenuBackground | TheThemePaints::MenubarBackground => {
                ThePaint::linear_gradient(
                    [0.0, top],
                    [0.0, bottom],
                    *self.color(DefaultWidgetBackground),
                    *self.color(DefaultWidgetDarkBackground),
                )
            }
            TheThemePaints::MenuItemHover | TheThemePaints::MenubarButtonHoverChrome => {
                ThePaint::solid(*self.color(MenubarButtonHover))
            }
            TheThemePaints::MenuItemSelected | TheThemePaints::MenubarButtonPressedChrome => {
                ThePaint::solid(*self.color(MenubarButtonClicked))
            }
            TheThemePaints::NodeHeaderNormalChrome
            | TheThemePaints::NodeBodyNormalChrome
            | TheThemePaints::NodeFooterNormalChrome => ThePaint::solid(*self.color(NodeBody)),
            TheThemePaints::NodeHeaderSelectedChrome
            | TheThemePaints::NodeBodySelectedChrome
            | TheThemePaints::NodeFooterSelectedChrome => {
                ThePaint::solid(*self.color(NodeBodySelected))
            }
            TheThemePaints::NodePreviewBackground => {
                ThePaint::solid(*self.color(DefaultWidgetDarkBackground))
            }
            TheThemePaints::ToolbarBackground => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                *self.color(DefaultWidgetBackground),
                *self.color(DefaultWidgetDarkBackground),
            ),
            TheThemePaints::StatusbarBackground => ThePaint::solid(*self.color(StatusbarEnd)),
            TheThemePaints::SwitchbarChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                *self.color(DefaultWidgetBackground),
                *self.color(DefaultWidgetDarkBackground),
            ),
            TheThemePaints::SwitchbarMarker => {
                ThePaint::solid(*self.color(SectionbarNormalTextColor))
            }
            TheThemePaints::SectionbarChrome => ThePaint::solid(*self.color(SectionbarBackground)),
            TheThemePaints::TabbarChrome => ThePaint::solid(*self.color(TabbarBackground)),
            TheThemePaints::TabNormalChrome => {
                ThePaint::solid(*self.color(DefaultWidgetDarkBackground))
            }
            TheThemePaints::TabHoverChrome => ThePaint::solid(*self.color(ToolbarButtonHover)),
            TheThemePaints::TabSelectedChrome => ThePaint::solid(*self.color(DefaultSelection)),
            TheThemePaints::SectionButtonNormal => {
                ThePaint::solid(*self.color(SectionbarBackground))
            }
            TheThemePaints::SectionButtonHover => {
                ThePaint::solid(*self.color(ToolListButtonHoverBackground))
            }
            TheThemePaints::SectionButtonSelected => ThePaint::solid(*self.color(DefaultSelection)),
            TheThemePaints::ToolListBarChrome => ThePaint::linear_gradient(
                [0.0, top],
                [0.0, bottom],
                *self.color(DefaultWidgetBackground),
                *self.color(DefaultWidgetDarkBackground),
            ),
            TheThemePaints::DropdownNormal => ThePaint::solid(*self.color(TextEditBackground)),
            TheThemePaints::DropdownHover => ThePaint::solid(*self.color(ToolbarButtonHover)),
            TheThemePaints::DropdownFocus => ThePaint::solid(*self.color(TextEditBackground)),
            TheThemePaints::DropdownPressed => ThePaint::solid(*self.color(ToolbarButtonClicked)),
            TheThemePaints::DropdownDisabled => {
                ThePaint::solid(*self.color(DefaultWidgetDarkBackground))
            }
            TheThemePaints::DropdownMarker => {
                ThePaint::solid(*self.color(SectionbarNormalTextColor))
            }
            TheThemePaints::TextInputNormal => ThePaint::solid(*self.color(TextEditBackground)),
            TheThemePaints::TextInputFocused => ThePaint::solid(*self.color(TextEditBackground)),
            TheThemePaints::TextInputDisabled => {
                ThePaint::solid(*self.color(DefaultWidgetDarkBackground))
            }
            TheThemePaints::CheckboxNormal => {
                ThePaint::solid(*self.color(DefaultWidgetDarkBackground))
            }
            TheThemePaints::CheckboxHover => ThePaint::solid(*self.color(ToolbarButtonHover)),
            TheThemePaints::CheckboxSelected => ThePaint::solid(*self.color(DefaultSelection)),
            TheThemePaints::CheckboxMark => {
                ThePaint::solid(*self.color(SectionbarSelectedTextColor))
            }
            TheThemePaints::ScrollbarTrack => ThePaint::solid(*self.color(ScrollbarBackground)),
            TheThemePaints::ScrollbarThumbNormal => {
                ThePaint::solid(*self.color(ToolbarButtonNormal))
            }
            TheThemePaints::ScrollbarThumbHover => ThePaint::solid(*self.color(ToolbarButtonHover)),
            TheThemePaints::ScrollbarThumbPressed => {
                ThePaint::solid(*self.color(ToolbarButtonClicked))
            }
            TheThemePaints::SnapperNormal => ThePaint::solid(*self.color(SectionbarBackground)),
            TheThemePaints::SnapperHover => ThePaint::solid(*self.color(ToolbarButtonHover)),
            TheThemePaints::SnapperPressed => ThePaint::solid(*self.color(ToolbarButtonClicked)),
            TheThemePaints::SnapperSelected => ThePaint::solid(*self.color(DefaultSelection)),
            TheThemePaints::SnapperMarker => {
                ThePaint::solid(*self.color(SectionbarNormalTextColor))
            }
            TheThemePaints::SliderTrackChrome => ThePaint::solid(*self.color(SliderSmallColor4)),
            TheThemePaints::SliderTrackAccent => ThePaint::solid(*self.color(SliderSmallColor2)),
            TheThemePaints::SliderThumbNormal => ThePaint::solid(*self.color(SliderSmallColor1)),
            TheThemePaints::SliderThumbHover | TheThemePaints::SliderThumbPressed => {
                ThePaint::solid(*self.color(SliderSmallColor3))
            }
            TheThemePaints::TimeSliderBackgroundChrome => {
                ThePaint::solid(*self.color(TimeSliderBackground))
            }
            TheThemePaints::TimeSliderMarkerChrome => {
                ThePaint::solid(*self.color(TimeSliderMarker))
            }
            TheThemePaints::TimeSliderPositionChrome => {
                ThePaint::solid(*self.color(TimeSliderPosition))
            }
            TheThemePaints::TrayButtonNormal => ThePaint::solid(*self.color(TraybarButtonNormal)),
            TheThemePaints::TrayButtonHover => ThePaint::solid(*self.color(TraybarButtonHover)),
            TheThemePaints::TrayButtonPressed => ThePaint::solid(*self.color(TraybarButtonClicked)),
            TheThemePaints::TrayButtonDisabled => {
                ThePaint::solid(*self.color(TraybarButtonDisabledBackground))
            }
            TheThemePaints::ControlNormal => ThePaint::solid(*self.color(ToolbarButtonNormal)),
            TheThemePaints::ControlHover => ThePaint::solid(*self.color(ToolbarButtonHover)),
            TheThemePaints::ControlPressed => ThePaint::solid(*self.color(ToolbarButtonClicked)),
            TheThemePaints::Selection => ThePaint::solid(*self.color(DefaultSelection)),
            TheThemePaints::Focus => ThePaint::solid(*self.color(SelectedTextEditBorder1)),
            TheThemePaints::Accent => ThePaint::solid(*self.color(Green)),
        }
    }

    /// Returns geometry tokens shared by painting and, later, layout.
    fn metric(&self, of: TheThemeMetrics) -> f32 {
        match of {
            TheThemeMetrics::ToolbarHeight => 22.0,
            TheThemeMetrics::StatusbarHeight => 21.0,
            TheThemeMetrics::ControlCornerRadius => 2.0,
            TheThemeMetrics::ControlBorderWidth => 1.0,
            TheThemeMetrics::FocusRingWidth => 1.5,
        }
    }

    /// Returns one color from an extensible semantic palette. Palette slots are intentionally
    /// indexed so applications can add new action groups without expanding the widget API.
    fn palette_color(&self, of: TheThemePalettes, index: usize) -> RGBA {
        const ACTION_GROUPS: [RGBA; 12] = [
            [160, 175, 190, 255],
            [195, 170, 150, 255],
            [200, 195, 150, 255],
            [160, 185, 160, 255],
            [176, 158, 192, 255],
            [198, 172, 112, 255],
            [135, 180, 190, 255],
            [190, 145, 150, 255],
            [170, 182, 130, 255],
            [145, 155, 194, 255],
            [196, 151, 116, 255],
            [172, 143, 188, 255],
        ];
        match of {
            TheThemePalettes::ActionGroups => ACTION_GROUPS[index % ACTION_GROUPS.len()],
        }
    }

    /// Returns the given color or its disabled version.
    fn color_disabled_switch(&mut self, of: TheThemeColors, disabled: bool) -> &RGBA;

    /// Returns the disabled color value for the given color
    fn color_disabled(&mut self, of: TheThemeColors) -> &RGBA;

    /// Returns the disabled color value for the given color
    fn color_disabled_t(&mut self, of: TheThemeColors) -> &RGBA;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TheThemePaints {
    MenuBackground,
    MenuItemHover,
    MenuItemSelected,
    MenubarBackground,
    MenubarButtonHoverChrome,
    MenubarButtonPressedChrome,
    NodeHeaderNormalChrome,
    NodeHeaderSelectedChrome,
    NodeBodyNormalChrome,
    NodeBodySelectedChrome,
    NodeFooterNormalChrome,
    NodeFooterSelectedChrome,
    NodePreviewBackground,
    ToolbarBackground,
    StatusbarBackground,
    SwitchbarChrome,
    SwitchbarMarker,
    SectionbarChrome,
    TabbarChrome,
    TabNormalChrome,
    TabHoverChrome,
    TabSelectedChrome,
    SectionButtonNormal,
    SectionButtonHover,
    SectionButtonSelected,
    ToolListBarChrome,
    DropdownNormal,
    DropdownHover,
    DropdownFocus,
    DropdownPressed,
    DropdownDisabled,
    DropdownMarker,
    TextInputNormal,
    TextInputFocused,
    TextInputDisabled,
    CheckboxNormal,
    CheckboxHover,
    CheckboxSelected,
    CheckboxMark,
    ScrollbarTrack,
    ScrollbarThumbNormal,
    ScrollbarThumbHover,
    ScrollbarThumbPressed,
    SnapperNormal,
    SnapperHover,
    SnapperPressed,
    SnapperSelected,
    SnapperMarker,
    SliderTrackChrome,
    SliderTrackAccent,
    SliderThumbNormal,
    SliderThumbHover,
    SliderThumbPressed,
    TimeSliderBackgroundChrome,
    TimeSliderMarkerChrome,
    TimeSliderPositionChrome,
    TrayButtonNormal,
    TrayButtonHover,
    TrayButtonPressed,
    TrayButtonDisabled,
    ControlNormal,
    ControlHover,
    ControlPressed,
    Selection,
    Focus,
    Accent,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TheThemeMetrics {
    ToolbarHeight,
    StatusbarHeight,
    ControlCornerRadius,
    ControlBorderWidth,
    FocusRingWidth,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TheThemePalettes {
    ActionGroups,
}

/// The
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TheThemeColors {
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
}
