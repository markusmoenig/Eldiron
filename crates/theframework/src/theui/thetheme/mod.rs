use crate::prelude::*;

pub mod dark;

pub mod prelude {
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

    /// Returns the given color or its disabled version.
    fn color_disabled_switch(&mut self, of: TheThemeColors, disabled: bool) -> &RGBA;

    /// Returns the disabled color value for the given color
    fn color_disabled(&mut self, of: TheThemeColors) -> &RGBA;

    /// Returns the disabled color value for the given color
    fn color_disabled_t(&mut self, of: TheThemeColors) -> &RGBA;
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
