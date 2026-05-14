use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutAction {
    ToolObject,
    ToolVertex,
    ToolEdge,
    ToolFace,
}

impl ShortcutAction {
    pub fn id(self) -> &'static str {
        match self {
            Self::ToolObject => "tool.object",
            Self::ToolVertex => "tool.vertex",
            Self::ToolEdge => "tool.edge",
            Self::ToolFace => "tool.face",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "tool.object" => Some(Self::ToolObject),
            "tool.vertex" => Some(Self::ToolVertex),
            "tool.edge" => Some(Self::ToolEdge),
            "tool.face" => Some(Self::ToolFace),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutResolution {
    Run(ShortcutAction),
    PreserveInTool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShortcutScope {
    Geometry3D,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortcutBinding {
    pub action: ShortcutAction,
    pub key: char,
    scope: ShortcutScope,
    pub legacy: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ShortcutContext {
    pub editor_view_mode: EditorViewMode,
    pub current_tool: MapToolType,
    pub has_geometry_objects: bool,
    pub has_geometry_vertices: bool,
    pub has_geometry_faces: bool,
    pub has_surface_detail: bool,
}

pub struct ShortcutResolver {
    bindings: Vec<ShortcutBinding>,
}

impl Default for ShortcutResolver {
    fn default() -> Self {
        Self {
            bindings: Self::default_bindings(),
        }
    }
}

impl ShortcutResolver {
    pub fn with_bindings(bindings: Vec<ShortcutBinding>) -> Self {
        Self { bindings }
    }

    pub fn geometry_tool_binding(
        action: ShortcutAction,
        key: char,
        legacy: bool,
    ) -> ShortcutBinding {
        ShortcutBinding {
            action,
            key,
            scope: ShortcutScope::Geometry3D,
            legacy,
        }
    }

    pub fn bindings(&self) -> &[ShortcutBinding] {
        &self.bindings
    }

    pub fn default_bindings() -> Vec<ShortcutBinding> {
        vec![
            Self::geometry_tool_binding(ShortcutAction::ToolObject, 'O', false),
            Self::geometry_tool_binding(ShortcutAction::ToolVertex, 'V', false),
            Self::geometry_tool_binding(ShortcutAction::ToolEdge, 'E', false),
            Self::geometry_tool_binding(ShortcutAction::ToolFace, 'F', false),
            // Transitional aliases: keep old 3D muscle memory working while the documented
            // defaults move to Object/Vertex/Edge/Face = O/V/E/F.
            Self::geometry_tool_binding(ShortcutAction::ToolObject, 'G', true),
            Self::geometry_tool_binding(ShortcutAction::ToolEdge, 'L', true),
        ]
    }

    pub fn from_toml(src: &str) -> Self {
        let mut bindings = Self::default_bindings();
        let Ok(root) = src.parse::<toml::Value>() else {
            return Self::with_bindings(bindings);
        };
        let Some(shortcuts) = root.get("shortcuts").and_then(toml::Value::as_table) else {
            return Self::with_bindings(bindings);
        };

        for (id, value) in shortcuts {
            let Some(action) = ShortcutAction::from_id(id.as_str()) else {
                continue;
            };
            let Some(key) = value.as_str().and_then(|value| value.trim().chars().next()) else {
                continue;
            };

            if let Some(binding) = bindings
                .iter_mut()
                .find(|binding| binding.action == action && !binding.legacy)
            {
                binding.key = key;
            } else {
                bindings.push(Self::geometry_tool_binding(action, key, false));
            }
        }

        Self::with_bindings(bindings)
    }

    pub fn resolve(&self, key: char, context: ShortcutContext) -> Option<ShortcutResolution> {
        if context.editor_view_mode == EditorViewMode::D2 {
            return None;
        }

        if Self::should_preserve_in_tool(key, context) {
            return Some(ShortcutResolution::PreserveInTool);
        }

        let key = key.to_ascii_uppercase();
        self.bindings
            .iter()
            .find(|binding| {
                binding.scope == ShortcutScope::Geometry3D
                    && binding.key.to_ascii_uppercase() == key
            })
            .map(|binding| ShortcutResolution::Run(binding.action))
    }

    fn should_preserve_in_tool(key: char, context: ShortcutContext) -> bool {
        let key = key.to_ascii_uppercase();
        match key {
            'R' => {
                context.current_tool == MapToolType::Selection
                    && context.has_geometry_objects
                    && !context.has_geometry_faces
                    && !context.has_geometry_vertices
                    && !context.has_surface_detail
            }
            'X' | 'M' | 'L' => context.has_geometry_vertices,
            'F' => context.current_tool == MapToolType::Vertex && context.has_geometry_vertices,
            'T' => context.has_geometry_faces || context.has_geometry_objects,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ShortcutContext {
        ShortcutContext {
            editor_view_mode: EditorViewMode::Orbit,
            current_tool: MapToolType::Selection,
            has_geometry_objects: false,
            has_geometry_vertices: false,
            has_geometry_faces: false,
            has_surface_detail: false,
        }
    }

    #[test]
    fn geometry_3d_defaults_use_object_vertex_edge_face_keys() {
        let resolver = ShortcutResolver::default();

        assert_eq!(
            resolver.resolve('o', ctx()),
            Some(ShortcutResolution::Run(ShortcutAction::ToolObject))
        );
        assert_eq!(
            resolver.resolve('v', ctx()),
            Some(ShortcutResolution::Run(ShortcutAction::ToolVertex))
        );
        assert_eq!(
            resolver.resolve('e', ctx()),
            Some(ShortcutResolution::Run(ShortcutAction::ToolEdge))
        );
        assert_eq!(
            resolver.resolve('f', ctx()),
            Some(ShortcutResolution::Run(ShortcutAction::ToolFace))
        );
    }

    #[test]
    fn vertex_fill_preserves_f_in_tool() {
        let mut context = ctx();
        context.current_tool = MapToolType::Vertex;
        context.has_geometry_vertices = true;

        assert_eq!(
            ShortcutResolver::default().resolve('f', context),
            Some(ShortcutResolution::PreserveInTool)
        );
    }

    #[test]
    fn two_d_keeps_legacy_tool_accelerators() {
        let mut context = ctx();
        context.editor_view_mode = EditorViewMode::D2;

        assert_eq!(ShortcutResolver::default().resolve('f', context), None);
    }

    #[test]
    fn custom_bindings_can_override_defaults() {
        let resolver =
            ShortcutResolver::with_bindings(vec![ShortcutResolver::geometry_tool_binding(
                ShortcutAction::ToolObject,
                'Q',
                false,
            )]);

        assert_eq!(
            resolver.resolve('q', ctx()),
            Some(ShortcutResolution::Run(ShortcutAction::ToolObject))
        );
        assert_eq!(resolver.resolve('o', ctx()), None);
    }

    #[test]
    fn toml_overrides_default_tool_bindings() {
        let resolver = ShortcutResolver::from_toml(
            r#"
[shortcuts]
"tool.object" = "Q"
"tool.face" = "A"
"#,
        );

        assert_eq!(
            resolver.resolve('q', ctx()),
            Some(ShortcutResolution::Run(ShortcutAction::ToolObject))
        );
        assert_eq!(
            resolver.resolve('a', ctx()),
            Some(ShortcutResolution::Run(ShortcutAction::ToolFace))
        );
        assert_eq!(resolver.resolve('o', ctx()), None);
    }
}
