use crate::actionscript::{EDITOR_ACTION_FUNCTION, EDITOR_TOOL_FUNCTION, EditorActionRequest};
use crate::editor::{ACTIONLIST, RUSTERIX, TOOLLIST};
use crate::prelude::*;
use rusterix::{
    Entity, Item, Value,
    server::ServerState,
    vm::{Execution, HostHandler, VM, VMValue},
};
use theframework::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConsoleFocus {
    Root,
    Entity(u32),
    Item(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleListScope {
    All,
    Characters,
    Items,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsoleRequest {
    Help,
    List(ConsoleListScope),
    Focus(String),
    Show,
    Get(String),
    Pwd,
    Up,
    Clear,
    ListActions {
        group: Option<String>,
        include_unavailable: bool,
    },
    RunAction(EditorActionRequest),
    ListTools {
        include_unavailable: bool,
    },
    SelectTool {
        command_id: String,
    },
}

const CONSOLE_LIST_FUNCTION: &str = "console_list";
const CONSOLE_FOCUS_FUNCTION: &str = "console_focus";
const CONSOLE_SHOW_FUNCTION: &str = "console_show";
const CONSOLE_GET_FUNCTION: &str = "console_get";
const CONSOLE_PWD_FUNCTION: &str = "console_pwd";
const CONSOLE_UP_FUNCTION: &str = "console_up";
const CONSOLE_FEEDBACK_ACTION: &str = "Console Feedback Action";
const CONSOLE_BACK: &str = "Console Back";
const CONSOLE_FORWARD: &str = "Console Forward";
const CONSOLE_OUTPUT: &str = "Console Output";
const CONSOLE_INPUT: &str = "Console Input";

#[derive(Default)]
struct ConsoleScriptHost {
    requests: Vec<ConsoleRequest>,
    error: Option<String>,
}

impl ConsoleScriptHost {
    fn string_arg<'a>(&mut self, args: &'a [VMValue], index: usize, name: &str) -> Option<&'a str> {
        let value = args.get(index).and_then(VMValue::as_string);
        if value.is_none() {
            self.error = Some(format!("{name} argument {} must be a string", index + 1));
        }
        value
    }
}

impl HostHandler for ConsoleScriptHost {
    fn on_host_call(&mut self, name: &str, args: &[VMValue]) -> Option<VMValue> {
        let request = match name {
            CONSOLE_LIST_FUNCTION => {
                let scope = self.string_arg(args, 0, name)?;
                let scope = match scope.trim().to_ascii_lowercase().as_str() {
                    "" | "all" => ConsoleListScope::All,
                    "characters" | "character" | "chars" => ConsoleListScope::Characters,
                    "items" | "item" => ConsoleListScope::Items,
                    _ => {
                        self.error =
                            Some(format!("{name} scope must be all, characters, or items"));
                        return Some(VMValue::from_bool(false));
                    }
                };
                ConsoleRequest::List(scope)
            }
            CONSOLE_FOCUS_FUNCTION => {
                let selector = self.string_arg(args, 0, name)?;
                ConsoleRequest::Focus(selector.to_string())
            }
            CONSOLE_SHOW_FUNCTION => ConsoleRequest::Show,
            CONSOLE_GET_FUNCTION => {
                let key = self.string_arg(args, 0, name)?;
                ConsoleRequest::Get(key.to_string())
            }
            CONSOLE_PWD_FUNCTION => ConsoleRequest::Pwd,
            CONSOLE_UP_FUNCTION => ConsoleRequest::Up,
            EDITOR_ACTION_FUNCTION => {
                let command_id = self.string_arg(args, 0, name)?;
                let parameters_toml = self.string_arg(args, 1, name)?;
                ConsoleRequest::RunAction(EditorActionRequest {
                    command_id: command_id.to_string(),
                    parameters_toml: parameters_toml.to_string(),
                })
            }
            EDITOR_TOOL_FUNCTION => {
                let command_id = self.string_arg(args, 0, name)?;
                ConsoleRequest::SelectTool {
                    command_id: command_id.to_string(),
                }
            }
            _ => return None,
        };
        self.requests.push(request);
        Some(VMValue::from_bool(true))
    }
}

fn collect_console_script_requests(source: &str) -> Result<Vec<ConsoleRequest>, String> {
    let mut vm = VM::default();
    for (name, arity) in [
        (CONSOLE_LIST_FUNCTION, 1),
        (CONSOLE_FOCUS_FUNCTION, 1),
        (CONSOLE_SHOW_FUNCTION, 0),
        (CONSOLE_GET_FUNCTION, 1),
        (CONSOLE_PWD_FUNCTION, 0),
        (CONSOLE_UP_FUNCTION, 0),
        (EDITOR_ACTION_FUNCTION, 2),
        (EDITOR_TOOL_FUNCTION, 1),
    ] {
        vm.register_host_function(name, arity)?;
    }
    let program = vm.prepare_str(source).map_err(|error| error.to_string())?;
    let mut execution = Execution::new(program.globals);
    let mut host = ConsoleScriptHost::default();
    execution.execute_host(&program.body, &program, &mut host);
    if let Some(error) = host.error {
        Err(error)
    } else {
        Ok(host.requests)
    }
}

pub struct ConsoleDock {
    document: TheFeedbackDocument,
    focus: ConsoleFocus,
    pending_requests: Vec<ConsoleRequest>,
    history: Vec<ConsoleHistoryEntry>,
    history_index: usize,
}

#[derive(Clone)]
struct ConsoleHistoryEntry {
    document: TheFeedbackDocument,
    focus: ConsoleFocus,
}

#[derive(Clone)]
struct RuntimeEntity {
    entity: Entity,
}

#[derive(Clone)]
struct RuntimeItem {
    item: Item,
}

impl ConsoleDock {
    fn console_input_id(ui: &mut TheUI) -> Option<TheId> {
        ui.get_widget(CONSOLE_INPUT)
            .map(|widget| widget.id().clone())
    }

    fn set_output(&mut self, document: TheFeedbackDocument, ui: &mut TheUI, ctx: &mut TheContext) {
        self.document = document;
        self.sync_output(ui, ctx);
    }

    fn sync_output(&self, ui: &mut TheUI, _ctx: &mut TheContext) {
        if let Some(output) = ui.get_text_view(CONSOLE_OUTPUT) {
            output.set_blocks(
                self.document
                    .to_text_view_blocks(&TheFeedbackPalette::default()),
            );
        }
    }

    fn history_entry(&self) -> ConsoleHistoryEntry {
        ConsoleHistoryEntry {
            document: self.document.clone(),
            focus: self.focus,
        }
    }

    fn sync_history_controls(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(back) = ui.get_widget(CONSOLE_BACK) {
            back.set_disabled(self.history_index == 0);
        }
        if let Some(forward) = ui.get_widget(CONSOLE_FORWARD) {
            forward.set_disabled(self.history_index + 1 >= self.history.len());
        }
        ctx.ui.redraw_all = true;
    }

    fn reset_history(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.history.clear();
        self.history.push(self.history_entry());
        self.history_index = 0;
        self.sync_history_controls(ui, ctx);
    }

    fn record_history(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.history.truncate(self.history_index.saturating_add(1));
        self.history.push(self.history_entry());
        self.history_index = self.history.len() - 1;
        self.sync_history_controls(ui, ctx);
    }

    fn refresh_current_history(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        let entry = self.history_entry();
        if let Some(current) = self.history.get_mut(self.history_index) {
            *current = entry;
        } else {
            self.history.push(entry);
            self.history_index = self.history.len() - 1;
        }
        self.sync_history_controls(ui, ctx);
    }

    fn navigate_history(&mut self, offset: isize, ui: &mut TheUI, ctx: &mut TheContext) {
        let target = self.history_index as isize + offset;
        if target < 0 || target >= self.history.len() as isize {
            return;
        }
        self.history_index = target as usize;
        let entry = self.history[self.history_index].clone();
        self.document = entry.document;
        self.focus = entry.focus;
        self.pending_requests.clear();
        self.sync_output(ui, ctx);
        if let Some(output) = ui.get_text_view(CONSOLE_OUTPUT) {
            output.scroll_to_top();
        }
        self.sync_history_controls(ui, ctx);
    }

    fn set_input(&self, ui: &mut TheUI, ctx: &mut TheContext, text: &str) {
        ui.set_widget_value(CONSOLE_INPUT, ctx, TheValue::Text(text.to_string()));
    }

    fn clear_input(&self, ui: &mut TheUI) {
        if let Some(widget) = ui.get_widget(CONSOLE_INPUT)
            && let Some(edit) = widget.as_text_line_edit()
        {
            edit.set_text(String::new());
        }
    }

    fn prompt(&self, project: &Project, server_ctx: &ServerContext) -> String {
        let region_name = project
            .get_region_ctx(server_ctx)
            .map(|region| region.name.clone())
            .unwrap_or_else(|| "Region".to_string());
        match self.focus {
            ConsoleFocus::Root => region_name,
            ConsoleFocus::Entity(id) => {
                let name = Self::runtime_snapshot(project, server_ctx)
                    .ok()
                    .and_then(|(entities, _)| {
                        entities
                            .iter()
                            .find(|entity| entity.entity.id == id)
                            .map(|entity| Self::entity_name(&entity.entity))
                    })
                    .unwrap_or_else(|| "Character".to_string());
                format!("{} / {}", region_name, name)
            }
            ConsoleFocus::Item(id) => {
                let name = Self::runtime_snapshot(project, server_ctx)
                    .ok()
                    .and_then(|(_, items)| {
                        items
                            .iter()
                            .find(|item| item.item.id == id)
                            .map(|item| Self::item_name(&item.item))
                    })
                    .unwrap_or_else(|| "Item".to_string());
                format!("{} / {}", region_name, name)
            }
        }
    }

    fn entity_name(entity: &Entity) -> String {
        entity
            .get_attr_string("name")
            .unwrap_or_else(|| format!("Entity {}", entity.id))
    }

    fn item_name(item: &Item) -> String {
        item.get_attr_string("name")
            .unwrap_or_else(|| format!("Item {}", item.id))
    }

    fn quoted(text: &str) -> String {
        format!("\"{}\"", text.replace('"', "'"))
    }

    fn format_value(value: &Value) -> String {
        value.to_string()
    }

    fn command(command: &str, description: &str) -> TheFeedbackCommand {
        TheFeedbackCommand::new(command, description)
            .interactive(CONSOLE_FEEDBACK_ACTION, TheValue::Text(command.to_string()))
    }

    fn command_with_input(command: String, description: String) -> TheFeedbackCommand {
        TheFeedbackCommand::new(command.clone(), description)
            .interactive(CONSOLE_FEEDBACK_ACTION, TheValue::Text(command))
    }

    fn value_span(value: impl Into<String>) -> TheFeedbackSpan {
        let value = value.into();
        let trimmed = value.trim();
        let role = if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false")
        {
            TheFeedbackRole::BoolValue
        } else if trimmed.parse::<f64>().is_ok()
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            TheFeedbackRole::NumberValue
        } else {
            TheFeedbackRole::StringValue
        };
        TheFeedbackSpan::new(value, role)
    }

    fn plain_document(text: impl Into<String>) -> TheFeedbackDocument {
        TheFeedbackDocument::new(vec![TheFeedbackBlock::paragraph(text)])
    }

    pub fn success_document(text: impl Into<String>) -> TheFeedbackDocument {
        TheFeedbackDocument::new(vec![TheFeedbackBlock::notice(
            TheFeedbackNoticeKind::Success,
            text,
        )])
    }

    pub fn action_success_document(request: &EditorActionRequest) -> TheFeedbackDocument {
        let mut document =
            Self::success_document(format!("Executed action `{}`.", request.command_id));
        let source = request.parameters_toml.trim();
        if source.is_empty() {
            return document;
        }

        if let Ok(table) = source.parse::<toml::Table>() {
            document.push(TheFeedbackBlock::KeyValueList {
                title: Some("Parameters".to_string()),
                entries: table
                    .into_iter()
                    .map(|(key, value)| {
                        TheFeedbackKeyValue::new(key, vec![Self::value_span(value.to_string())])
                    })
                    .collect(),
            });
        } else {
            document.push(TheFeedbackBlock::Code {
                language: Some("TOML".to_string()),
                source: source.to_string(),
            });
        }
        document
    }

    pub fn error_document(text: impl Into<String>) -> TheFeedbackDocument {
        TheFeedbackDocument::new(vec![TheFeedbackBlock::notice(
            TheFeedbackNoticeKind::Error,
            text,
        )])
    }

    fn intro() -> TheFeedbackDocument {
        TheFeedbackDocument::new(vec![
            TheFeedbackBlock::heading("Console"),
            TheFeedbackBlock::notice(
                TheFeedbackNoticeKind::Success,
                "Ready for game inspection and editor operations.",
            ),
            TheFeedbackBlock::CommandList {
                title: Some("Quick commands".to_string()),
                entries: vec![
                    Self::command("list", "List live characters and items"),
                    Self::command("actions", "List actions applicable now"),
                    Self::command("tools", "List available editor tools"),
                    Self::command("help", "Show all Console and Eldrin commands"),
                ],
            },
            TheFeedbackBlock::Paragraph(vec![
                TheFeedbackSpan::new("Tip  ", TheFeedbackRole::Info),
                TheFeedbackSpan::new(
                    "Select an underlined command to put it in the input field.",
                    TheFeedbackRole::Muted,
                ),
            ]),
        ])
    }

    fn parse_id(text: &str) -> Option<u32> {
        text.trim().parse::<u32>().ok()
    }

    fn entity_matches(entity: &Entity, needle: &str) -> bool {
        entity.id.to_string() == needle || Self::entity_name(entity).eq_ignore_ascii_case(needle)
    }

    fn item_matches(item: &Item, needle: &str) -> bool {
        item.id.to_string() == needle || Self::item_name(item).eq_ignore_ascii_case(needle)
    }

    fn collect_nested_items(items: &[Item], out: &mut Vec<RuntimeItem>) {
        for item in items {
            out.push(RuntimeItem { item: item.clone() });
            if let Some(container) = &item.container {
                Self::collect_nested_items(container, out);
            }
        }
    }

    fn collect_nested_items_from_entity(entity: &Entity, out: &mut Vec<RuntimeItem>) {
        for item in entity.inventory.iter().flatten() {
            out.push(RuntimeItem { item: item.clone() });
            if let Some(container) = &item.container {
                Self::collect_nested_items(container, out);
            }
        }
        for item in entity.equipped.values() {
            out.push(RuntimeItem { item: item.clone() });
            if let Some(container) = &item.container {
                Self::collect_nested_items(container, out);
            }
        }
    }

    fn collect_focusable_items(
        entities: &[RuntimeEntity],
        items: &[RuntimeItem],
    ) -> Vec<RuntimeItem> {
        let mut collected = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for item in items {
            if seen.insert(item.item.id) {
                collected.push(item.clone());
            }
            if let Some(container) = &item.item.container {
                let mut nested = Vec::new();
                Self::collect_nested_items(container, &mut nested);
                for child in nested {
                    if seen.insert(child.item.id) {
                        collected.push(child);
                    }
                }
            }
        }

        for entity in entities {
            let mut nested = Vec::new();
            Self::collect_nested_items_from_entity(&entity.entity, &mut nested);
            for child in nested {
                if seen.insert(child.item.id) {
                    collected.push(child);
                }
            }
        }

        collected
    }

    fn focused_entity<'a>(&self, entities: &'a [RuntimeEntity]) -> Option<&'a RuntimeEntity> {
        match self.focus {
            ConsoleFocus::Entity(id) => entities.iter().find(|entity| entity.entity.id == id),
            _ => None,
        }
    }

    fn list_root(&self, entities: &[RuntimeEntity], items: &[RuntimeItem]) -> TheFeedbackDocument {
        let mut document = Self::list_characters(entities);
        document.extend(Self::list_items(items));
        document
    }

    fn item_tree_feedback(items: &mut Vec<Vec<TheFeedbackSpan>>, item: &Item, depth: usize) {
        let command = format!("focus {}", item.id);
        items.push(vec![
            TheFeedbackSpan::new("  ".repeat(depth), TheFeedbackRole::Muted),
            TheFeedbackSpan::new(Self::item_name(item), TheFeedbackRole::StringValue)
                .interactive(CONSOLE_FEEDBACK_ACTION, TheValue::Text(command)),
            TheFeedbackSpan::new(format!("  #{}", item.id), TheFeedbackRole::StableId),
        ]);
        if let Some(container) = &item.container {
            for child in container {
                Self::item_tree_feedback(items, child, depth + 1);
            }
        }
    }

    fn list_entity(&self, entity: &RuntimeEntity) -> TheFeedbackDocument {
        let mut attributes = Vec::new();
        for key in entity.entity.attributes.keys_sorted() {
            if key == "setup" || key == "_source_seq" {
                continue;
            }
            if let Some(value) = entity.entity.attributes.get(key) {
                attributes.push(TheFeedbackKeyValue::new(
                    key.clone(),
                    vec![Self::value_span(Self::format_value(value))],
                ));
            }
        }

        let mut inventory = Vec::new();
        for item in entity.entity.inventory.iter().flatten() {
            Self::item_tree_feedback(&mut inventory, item, 0);
        }

        let equipped = entity
            .entity
            .equipped
            .iter()
            .map(|(slot, item)| {
                TheFeedbackKeyValue::new(
                    slot.clone(),
                    vec![
                        TheFeedbackSpan::new(Self::item_name(item), TheFeedbackRole::StringValue)
                            .interactive(
                                CONSOLE_FEEDBACK_ACTION,
                                TheValue::Text(format!("focus {}", item.id)),
                            ),
                        TheFeedbackSpan::new(format!("  #{}", item.id), TheFeedbackRole::StableId),
                    ],
                )
            })
            .collect();

        TheFeedbackDocument::new(vec![
            TheFeedbackBlock::Heading {
                level: 1,
                spans: vec![
                    TheFeedbackSpan::new("Character  ", TheFeedbackRole::Heading),
                    TheFeedbackSpan::new(
                        Self::entity_name(&entity.entity),
                        TheFeedbackRole::StringValue,
                    ),
                    TheFeedbackSpan::new(
                        format!("  #{}", entity.entity.id),
                        TheFeedbackRole::StableId,
                    ),
                ],
            },
            TheFeedbackBlock::KeyValueList {
                title: Some("Transform".to_string()),
                entries: vec![
                    TheFeedbackKeyValue::new(
                        "position",
                        vec![Self::value_span(format!(
                            "[{:.2}, {:.2}, {:.2}]",
                            entity.entity.position.x,
                            entity.entity.position.y,
                            entity.entity.position.z
                        ))],
                    ),
                    TheFeedbackKeyValue::new(
                        "orientation",
                        vec![Self::value_span(format!(
                            "[{:.2}, {:.2}]",
                            entity.entity.orientation.x, entity.entity.orientation.y
                        ))],
                    ),
                ],
            },
            TheFeedbackBlock::KeyValueList {
                title: Some("Attributes".to_string()),
                entries: attributes,
            },
            TheFeedbackBlock::List {
                title: Some("Inventory".to_string()),
                items: inventory,
            },
            TheFeedbackBlock::KeyValueList {
                title: Some("Equipped".to_string()),
                entries: equipped,
            },
        ])
    }

    fn list_item(&self, item: &RuntimeItem) -> TheFeedbackDocument {
        let mut attributes = Vec::new();
        for key in item.item.attributes.keys_sorted() {
            if key == "setup" || key == "_source_seq" {
                continue;
            }
            if let Some(value) = item.item.attributes.get(key) {
                attributes.push(TheFeedbackKeyValue::new(
                    key.clone(),
                    vec![Self::value_span(Self::format_value(value))],
                ));
            }
        }
        let mut container = Vec::new();
        if let Some(items) = &item.item.container {
            for child in items {
                Self::item_tree_feedback(&mut container, child, 0);
            }
        }

        TheFeedbackDocument::new(vec![
            TheFeedbackBlock::Heading {
                level: 1,
                spans: vec![
                    TheFeedbackSpan::new("Item  ", TheFeedbackRole::Heading),
                    TheFeedbackSpan::new(Self::item_name(&item.item), TheFeedbackRole::StringValue),
                    TheFeedbackSpan::new(format!("  #{}", item.item.id), TheFeedbackRole::StableId),
                ],
            },
            TheFeedbackBlock::KeyValueList {
                title: Some("Transform".to_string()),
                entries: vec![TheFeedbackKeyValue::new(
                    "position",
                    vec![Self::value_span(format!(
                        "[{:.2}, {:.2}, {:.2}]",
                        item.item.position.x, item.item.position.y, item.item.position.z
                    ))],
                )],
            },
            TheFeedbackBlock::KeyValueList {
                title: Some("Attributes".to_string()),
                entries: attributes,
            },
            TheFeedbackBlock::List {
                title: Some("Container".to_string()),
                items: container,
            },
        ])
    }

    fn runtime_snapshot(
        project: &Project,
        server_ctx: &ServerContext,
    ) -> Result<(Vec<RuntimeEntity>, Vec<RuntimeItem>), String> {
        let rusterix = RUSTERIX.read().unwrap();
        if rusterix.server.state != ServerState::Running {
            return Err("Game is not running.".to_string());
        }

        let mut runtime_entities = Vec::new();
        let mut runtime_items = Vec::new();

        let (entities, items) = rusterix.server.get_entities_items(&server_ctx.curr_region);
        if let Some(entities) = entities {
            for entity in entities {
                runtime_entities.push(RuntimeEntity {
                    entity: entity.clone(),
                });
            }
        }
        if let Some(items) = items {
            for item in items {
                runtime_items.push(RuntimeItem { item: item.clone() });
            }
        }

        if runtime_entities.is_empty()
            && runtime_items.is_empty()
            && let Some(region) = project.get_region_ctx(server_ctx)
        {
            for entity in &region.map.entities {
                runtime_entities.push(RuntimeEntity {
                    entity: entity.clone(),
                });
            }
            for item in &region.map.items {
                runtime_items.push(RuntimeItem { item: item.clone() });
            }
        }

        Ok((runtime_entities, runtime_items))
    }

    fn focus_label(&self, entities: &[RuntimeEntity], items: &[RuntimeItem]) -> String {
        match self.focus {
            ConsoleFocus::Root => "root".to_string(),
            ConsoleFocus::Entity(id) => entities
                .iter()
                .find(|entity| entity.entity.id == id)
                .map(|entity| {
                    format!(
                        "character {} {}",
                        Self::quoted(&Self::entity_name(&entity.entity)),
                        id
                    )
                })
                .unwrap_or_else(|| format!("character {}", id)),
            ConsoleFocus::Item(id) => items
                .iter()
                .find(|item| item.item.id == id)
                .map(|item| format!("item {} {}", Self::quoted(&Self::item_name(&item.item)), id))
                .unwrap_or_else(|| format!("item {}", id)),
        }
    }

    fn help() -> TheFeedbackDocument {
        TheFeedbackDocument::new(vec![
            TheFeedbackBlock::heading("Console commands"),
            TheFeedbackBlock::CommandList {
                title: Some("Game information".to_string()),
                entries: vec![
                    Self::command("list [characters|items]", "List the current runtime scope"),
                    Self::command("focus <name|id>", "Focus a live character or item"),
                    Self::command("show", "Show the focused object"),
                    Self::command("get <key>", "Show one focused attribute"),
                    Self::command("pwd", "Show the current focus"),
                    Self::command("up", "Reset focus to the region"),
                ],
            },
            TheFeedbackBlock::CommandList {
                title: Some("Editor operations".to_string()),
                entries: vec![
                    Self::command(
                        "actions [all] [group]",
                        "List applicable actions or the full catalog",
                    ),
                    Self::command("action <id> [TOML]", "Run an action by stable ID"),
                    Self::command("tools [all]", "List available tools or the full catalog"),
                    Self::command("tool <id>", "Select a tool by stable ID"),
                    Self::command("clear", "Clear the Console output"),
                ],
            },
            TheFeedbackBlock::CommandList {
                title: Some("Eldrin automation".to_string()),
                entries: vec![Self::command(
                    "eldrin <source>",
                    "Run ordered Console automation",
                )],
            },
            TheFeedbackBlock::Code {
                language: Some("Eldrin".to_string()),
                source: [
                    "console_list(\"all|characters|items\");",
                    "console_focus(\"name|id\");",
                    "console_show(); console_get(\"key\");",
                    "console_pwd(); console_up();",
                    "editor_action(\"face.extrude\", \"amount = 2\");",
                    "editor_tool(\"tool.geometry\");",
                ]
                .join("\n"),
            },
        ])
    }

    fn unquote(text: &str) -> String {
        let trimmed = text.trim();
        if trimmed.len() >= 2
            && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn parse_command(command: &str) -> Result<Vec<ConsoleRequest>, String> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let (head, tail) = trimmed
            .split_once(char::is_whitespace)
            .map(|(head, tail)| (head.to_ascii_lowercase(), tail.trim()))
            .unwrap_or_else(|| (trimmed.to_ascii_lowercase(), ""));

        let request = match head.as_str() {
            "help" | "commands" => ConsoleRequest::Help,
            "list" | "ls" => {
                let scope = match tail.to_ascii_lowercase().as_str() {
                    "" | "all" => ConsoleListScope::All,
                    "character" | "characters" | "char" | "chars" => ConsoleListScope::Characters,
                    "item" | "items" => ConsoleListScope::Items,
                    _ => return Err("Usage: list [characters|items]".to_string()),
                };
                ConsoleRequest::List(scope)
            }
            "focus" | "cd" => {
                if tail.is_empty() {
                    return Err(format!("Usage: {head} <name|id>"));
                }
                if tail == ".." || tail == "/" || tail.eq_ignore_ascii_case("root") {
                    ConsoleRequest::Up
                } else {
                    ConsoleRequest::Focus(Self::unquote(tail))
                }
            }
            "show" | "info" => ConsoleRequest::Show,
            "get" => {
                if tail.is_empty() {
                    return Err("Usage: get <key>".to_string());
                }
                ConsoleRequest::Get(Self::unquote(tail))
            }
            "pwd" => ConsoleRequest::Pwd,
            "up" => ConsoleRequest::Up,
            "clear" => ConsoleRequest::Clear,
            "actions" => {
                let mut group = None;
                let mut include_unavailable = false;
                for argument in tail.split_whitespace() {
                    if argument.eq_ignore_ascii_case("all") {
                        include_unavailable = true;
                    } else if group.is_none() {
                        group = Some(argument.to_ascii_lowercase());
                    } else {
                        return Err("Usage: actions [all] [group]".to_string());
                    }
                }
                ConsoleRequest::ListActions {
                    group,
                    include_unavailable,
                }
            }
            "action" => {
                let (id, parameters_toml) = tail
                    .split_once(char::is_whitespace)
                    .map(|(id, params)| (id.trim(), params.trim()))
                    .unwrap_or((tail, ""));
                if id.is_empty() {
                    return Err("Usage: action <stable.id> [TOML parameters]".to_string());
                }
                ConsoleRequest::RunAction(EditorActionRequest {
                    command_id: id.to_string(),
                    parameters_toml: parameters_toml.to_string(),
                })
            }
            "tools" => match tail.to_ascii_lowercase().as_str() {
                "" => ConsoleRequest::ListTools {
                    include_unavailable: false,
                },
                "all" => ConsoleRequest::ListTools {
                    include_unavailable: true,
                },
                _ => return Err("Usage: tools [all]".to_string()),
            },
            "tool" => {
                if tail.is_empty() || tail.split_whitespace().count() != 1 {
                    return Err("Usage: tool <stable.id>".to_string());
                }
                let command_id = if tail.contains('.') {
                    tail.to_string()
                } else {
                    format!("tool.{tail}")
                };
                ConsoleRequest::SelectTool { command_id }
            }
            "eldrin" => {
                if tail.is_empty() {
                    return Err("Usage: eldrin <source>".to_string());
                }
                return collect_console_script_requests(tail);
            }
            _ => return Err(format!("Unknown command `{trimmed}`. Type `help`.")),
        };
        Ok(vec![request])
    }

    fn list_characters(entities: &[RuntimeEntity]) -> TheFeedbackDocument {
        TheFeedbackDocument::new(vec![TheFeedbackBlock::List {
            title: Some(format!("Characters  {}", entities.len())),
            items: entities
                .iter()
                .map(|entity| {
                    vec![
                        TheFeedbackSpan::new(
                            Self::entity_name(&entity.entity),
                            TheFeedbackRole::StringValue,
                        )
                        .interactive(
                            CONSOLE_FEEDBACK_ACTION,
                            TheValue::Text(format!("focus {}", entity.entity.id)),
                        ),
                        TheFeedbackSpan::new(
                            format!("  #{}", entity.entity.id),
                            TheFeedbackRole::StableId,
                        ),
                    ]
                })
                .collect(),
        }])
    }

    fn list_items(items: &[RuntimeItem]) -> TheFeedbackDocument {
        TheFeedbackDocument::new(vec![TheFeedbackBlock::List {
            title: Some(format!("Items  {}", items.len())),
            items: items
                .iter()
                .map(|item| {
                    vec![
                        TheFeedbackSpan::new(
                            Self::item_name(&item.item),
                            TheFeedbackRole::StringValue,
                        )
                        .interactive(
                            CONSOLE_FEEDBACK_ACTION,
                            TheValue::Text(format!("focus {}", item.item.id)),
                        ),
                        TheFeedbackSpan::new(
                            format!("  #{}", item.item.id),
                            TheFeedbackRole::StableId,
                        ),
                    ]
                })
                .collect(),
        }])
    }

    fn list_actions(
        group: Option<&str>,
        include_unavailable: bool,
        project: &Project,
        server_ctx: &ServerContext,
        ctx: &mut TheContext,
    ) -> Result<TheFeedbackDocument, String> {
        let valid_groups = ActionGroup::ALL.map(ActionGroup::id);
        if let Some(group) = group
            && !valid_groups.contains(&group)
        {
            return Err(format!(
                "Unknown action group `{group}`. Groups: {}",
                valid_groups.join(", ")
            ));
        }

        let default_map = Map::default();
        let map = project.get_map(server_ctx).unwrap_or(&default_map);
        let actions = ACTIONLIST.read().unwrap();
        let mut entries = actions
            .actions
            .iter()
            .enumerate()
            .filter_map(|(index, action)| {
                let descriptor = actions.descriptor_by_id(action.id().uuid)?;
                if group.is_some_and(|group| group != descriptor.group.id()) {
                    return None;
                }
                let applicable = action.is_applicable(map, ctx, server_ctx);
                if !include_unavailable && !applicable {
                    return None;
                }
                Some((
                    descriptor.group.palette_slot(),
                    index,
                    descriptor.command_id.clone(),
                    descriptor.group.qualified_name(&action.id().name),
                    applicable,
                ))
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|(slot, index, _, _, _)| (*slot, *index));

        let count = entries.len();
        Ok(TheFeedbackDocument::new(vec![
            TheFeedbackBlock::CommandList {
                title: Some(format!("Actions  {count}")),
                entries: entries
                    .into_iter()
                    .map(|(_, _, id, name, applicable)| {
                        Self::command_with_input(format!("action {id}"), name).available(applicable)
                    })
                    .collect(),
            },
        ]))
    }

    fn list_tools(include_unavailable: bool) -> TheFeedbackDocument {
        let tools = TOOLLIST.read().unwrap();
        let mut entries = Vec::new();
        for tool in &tools.game_tools {
            let Some(descriptor) = tools.game_tool_descriptor_by_id(tool.id().uuid) else {
                continue;
            };
            let available = tools.game_tool_is_available(&descriptor.command_id);
            if !include_unavailable && !available {
                continue;
            }
            let selected =
                tools.current_game_tool_command_id() == Some(descriptor.command_id.as_str());
            let description = if selected {
                format!("{}  • selected", tool.id().name)
            } else {
                tool.id().name.clone()
            };
            entries.push(
                Self::command_with_input(format!("tool {}", descriptor.command_id), description)
                    .available(available),
            );
        }
        TheFeedbackDocument::new(vec![TheFeedbackBlock::CommandList {
            title: Some(format!("Tools  {}", entries.len())),
            entries,
        }])
    }

    pub fn execute_local_request(
        &mut self,
        request: &ConsoleRequest,
        project: &Project,
        server_ctx: &ServerContext,
        ctx: &mut TheContext,
    ) -> Result<TheFeedbackDocument, String> {
        match request {
            ConsoleRequest::Help => Ok(Self::help()),
            ConsoleRequest::ListActions {
                group,
                include_unavailable,
            } => Self::list_actions(
                group.as_deref(),
                *include_unavailable,
                project,
                server_ctx,
                ctx,
            ),
            ConsoleRequest::ListTools {
                include_unavailable,
            } => Ok(Self::list_tools(*include_unavailable)),
            ConsoleRequest::Clear => Ok(TheFeedbackDocument::default()),
            ConsoleRequest::RunAction(_) | ConsoleRequest::SelectTool { .. } => {
                Err("Editor request was not dispatched by the sidebar.".to_string())
            }
            request => {
                let (entities, items) = Self::runtime_snapshot(project, server_ctx)?;
                let focusable_items = Self::collect_focusable_items(&entities, &items);
                match request {
                    ConsoleRequest::List(ConsoleListScope::Characters) => {
                        Ok(Self::list_characters(&entities))
                    }
                    ConsoleRequest::List(ConsoleListScope::Items) => Ok(Self::list_items(&items)),
                    ConsoleRequest::List(ConsoleListScope::All) | ConsoleRequest::Show => {
                        match self.focus {
                            ConsoleFocus::Root => Ok(self.list_root(&entities, &items)),
                            ConsoleFocus::Entity(_) => {
                                if let Some(entity) = self.focused_entity(&entities) {
                                    Ok(self.list_entity(entity))
                                } else {
                                    self.focus = ConsoleFocus::Root;
                                    Err("Focused character no longer exists.".to_string())
                                }
                            }
                            ConsoleFocus::Item(id) => {
                                if let Some(item) =
                                    focusable_items.iter().find(|item| item.item.id == id)
                                {
                                    Ok(self.list_item(item))
                                } else {
                                    self.focus = ConsoleFocus::Root;
                                    Err("Focused item no longer exists.".to_string())
                                }
                            }
                        }
                    }
                    ConsoleRequest::Focus(selector) => {
                        let needle = selector.trim();
                        if let Some(id) = Self::parse_id(needle) {
                            if let Some(entity) =
                                entities.iter().find(|entity| entity.entity.id == id)
                            {
                                self.focus = ConsoleFocus::Entity(id);
                                return Ok(self.list_entity(entity));
                            }
                            if let Some(item) =
                                focusable_items.iter().find(|item| item.item.id == id)
                            {
                                self.focus = ConsoleFocus::Item(id);
                                return Ok(self.list_item(item));
                            }
                        }

                        let matching_entities = entities
                            .iter()
                            .filter(|entity| Self::entity_matches(&entity.entity, needle))
                            .collect::<Vec<_>>();
                        let matching_items = focusable_items
                            .iter()
                            .filter(|item| Self::item_matches(&item.item, needle))
                            .collect::<Vec<_>>();
                        if matching_entities.len() + matching_items.len() > 1 {
                            let mut matches = Vec::new();
                            for entity in &matching_entities {
                                matches.push(vec![
                                    TheFeedbackSpan::new("Character  ", TheFeedbackRole::Muted),
                                    TheFeedbackSpan::new(
                                        Self::entity_name(&entity.entity),
                                        TheFeedbackRole::StringValue,
                                    )
                                    .interactive(
                                        CONSOLE_FEEDBACK_ACTION,
                                        TheValue::Text(format!("focus {}", entity.entity.id)),
                                    ),
                                    TheFeedbackSpan::new(
                                        format!("  #{}", entity.entity.id),
                                        TheFeedbackRole::StableId,
                                    ),
                                ]);
                            }
                            for item in &matching_items {
                                matches.push(vec![
                                    TheFeedbackSpan::new("Item  ", TheFeedbackRole::Muted),
                                    TheFeedbackSpan::new(
                                        Self::item_name(&item.item),
                                        TheFeedbackRole::StringValue,
                                    )
                                    .interactive(
                                        CONSOLE_FEEDBACK_ACTION,
                                        TheValue::Text(format!("focus {}", item.item.id)),
                                    ),
                                    TheFeedbackSpan::new(
                                        format!("  #{}", item.item.id),
                                        TheFeedbackRole::StableId,
                                    ),
                                ]);
                            }
                            return Ok(TheFeedbackDocument::new(vec![
                                TheFeedbackBlock::notice(
                                    TheFeedbackNoticeKind::Warning,
                                    "Several runtime objects matched. Select one to focus it.",
                                ),
                                TheFeedbackBlock::List {
                                    title: Some("Matches".to_string()),
                                    items: matches,
                                },
                            ]));
                        }
                        if let Some(entity) = matching_entities.first() {
                            self.focus = ConsoleFocus::Entity(entity.entity.id);
                            return Ok(self.list_entity(entity));
                        }
                        if let Some(item) = matching_items.first() {
                            self.focus = ConsoleFocus::Item(item.item.id);
                            return Ok(self.list_item(item));
                        }
                        Err(format!("No runtime character or item matched `{needle}`."))
                    }
                    ConsoleRequest::Get(key) => match self.focus {
                        ConsoleFocus::Entity(_) => self
                            .focused_entity(&entities)
                            .ok_or_else(|| "Focused character no longer exists.".to_string())
                            .and_then(|entity| {
                                entity
                                    .entity
                                    .attributes
                                    .get(key)
                                    .map(|value| {
                                        TheFeedbackDocument::new(vec![
                                            TheFeedbackBlock::KeyValueList {
                                                title: None,
                                                entries: vec![TheFeedbackKeyValue::new(
                                                    key.clone(),
                                                    vec![Self::value_span(Self::format_value(
                                                        value,
                                                    ))],
                                                )],
                                            },
                                        ])
                                    })
                                    .ok_or_else(|| format!("Attribute `{key}` not found."))
                            }),
                        ConsoleFocus::Item(id) => focusable_items
                            .iter()
                            .find(|item| item.item.id == id)
                            .ok_or_else(|| "Focused item no longer exists.".to_string())
                            .and_then(|item| {
                                item.item
                                    .attributes
                                    .get(key)
                                    .map(|value| {
                                        TheFeedbackDocument::new(vec![
                                            TheFeedbackBlock::KeyValueList {
                                                title: None,
                                                entries: vec![TheFeedbackKeyValue::new(
                                                    key.clone(),
                                                    vec![Self::value_span(Self::format_value(
                                                        value,
                                                    ))],
                                                )],
                                            },
                                        ])
                                    })
                                    .ok_or_else(|| format!("Attribute `{key}` not found."))
                            }),
                        ConsoleFocus::Root => Err("Focus a character or item first.".to_string()),
                    },
                    ConsoleRequest::Pwd => Ok(Self::plain_document(
                        self.focus_label(&entities, &focusable_items),
                    )),
                    ConsoleRequest::Up => {
                        self.focus = ConsoleFocus::Root;
                        Ok(self.list_root(&entities, &items))
                    }
                    _ => unreachable!("non-runtime console request handled above"),
                }
            }
        }
    }

    pub fn take_pending_requests(&mut self) -> Vec<ConsoleRequest> {
        std::mem::take(&mut self.pending_requests)
    }

    pub fn complete_requests(
        &mut self,
        results: &[TheFeedbackDocument],
        clear: bool,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if clear {
            self.document = TheFeedbackDocument::default();
        } else {
            for result in results.iter().filter(|result| !result.is_empty()) {
                self.document.extend(result.clone());
            }
        }
        self.sync_output(ui, ctx);
        self.refresh_current_history(ui, ctx);
    }
}

impl Dock for ConsoleDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        let document = Self::intro();
        Self {
            history: vec![ConsoleHistoryEntry {
                document: document.clone(),
                focus: ConsoleFocus::Root,
            }],
            history_index: 0,
            document,
            focus: ConsoleFocus::Root,
            pending_requests: Vec::new(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut output = TheTextView::new(TheId::named(CONSOLE_OUTPUT));
        output.set_font_size(13.0);
        output.set_font_preference(TheFontPreference::Code);
        output.set_word_wrap(true);
        output.set_padding((10, 8, 10, 8));
        output.set_selectable(true);
        output.draw_background(true);
        canvas.set_widget(output);

        let mut navigation_canvas = TheCanvas::default();
        navigation_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut navigation = TheHLayout::new(TheId::named("Console Navigation"));
        navigation.set_background_color(None);
        navigation.set_margin(Vec4::new(5, 1, 5, 1));
        navigation.set_padding(3);

        let mut back = TheTraybarButton::new(TheId::named(CONSOLE_BACK));
        back.set_text("<".to_string());
        back.set_fixed_size(true);
        back.set_status_text(&fl!("status_console_back"));
        back.limiter_mut().set_max_size(Vec2::new(24, 20));
        navigation.add_widget(Box::new(back));

        let mut forward = TheTraybarButton::new(TheId::named(CONSOLE_FORWARD));
        forward.set_text(">".to_string());
        forward.set_fixed_size(true);
        forward.set_status_text(&fl!("status_console_forward"));
        forward.limiter_mut().set_max_size(Vec2::new(24, 20));
        navigation.add_widget(Box::new(forward));

        navigation_canvas.set_layout(navigation);
        canvas.set_top(navigation_canvas);

        let mut input_canvas = TheCanvas::default();
        let mut input = TheTextLineEdit::new(TheId::named(CONSOLE_INPUT));
        input.set_status_text(&fl!("status_console_input"));
        input.set_font_size(12.5);
        input.limiter_mut().set_max_height(24);
        input_canvas.set_widget(input);
        canvas.set_bottom(input_canvas);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
        if self.document.is_empty() {
            self.document = Self::intro();
            self.focus = ConsoleFocus::Root;
            self.reset_history(ui, ctx);
        }
        self.sync_output(ui, ctx);
        self.sync_history_controls(ui, ctx);
        self.set_input(ui, ctx, "");
        if let Some(id) = Self::console_input_id(ui) {
            ctx.ui.set_focus(&id);
        }
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let TheEvent::StateChanged(id, TheWidgetState::Clicked) = event {
            if id.name == CONSOLE_BACK {
                self.navigate_history(-1, ui, ctx);
                self.clear_input(ui);
                return true;
            }
            if id.name == CONSOLE_FORWARD {
                self.navigate_history(1, ui, ctx);
                self.clear_input(ui);
                return true;
            }
        }

        if let TheEvent::Custom(id, TheValue::Text(command)) = event
            && id.name == CONSOLE_FEEDBACK_ACTION
        {
            self.set_input(ui, ctx, command);
            if let Some(input_id) = Self::console_input_id(ui) {
                ctx.ui.set_focus(&input_id);
            }
            return true;
        }

        if let TheEvent::ValueChanged(id, value) = event
            && id.name == CONSOLE_INPUT
        {
            let command = value.to_string().unwrap_or_default();
            let command = command.trim().to_string();
            if command.is_empty() {
                self.set_input(ui, ctx, "");
                return false;
            }

            let prompt = self.prompt(project, server_ctx);
            let mut output = TheFeedbackDocument::new(vec![TheFeedbackBlock::Paragraph(vec![
                TheFeedbackSpan::new(format!("{prompt}  ›  "), TheFeedbackRole::Muted),
                TheFeedbackSpan::new(command.clone(), TheFeedbackRole::Command),
            ])]);
            match Self::parse_command(&command) {
                Ok(requests) => {
                    self.pending_requests = requests;
                    self.set_output(output, ui, ctx);
                    self.record_history(ui, ctx);
                }
                Err(error) => {
                    self.pending_requests.clear();
                    output.extend(Self::error_document(error));
                    self.set_output(output, ui, ctx);
                    self.record_history(ui, ctx);
                }
            }
            self.clear_input(ui);
            if let Some(focus_id) = Self::console_input_id(ui) {
                ctx.ui.focus = Some(focus_id.clone());
                ctx.ui.keyboard_focus = Some(focus_id.clone());
                ctx.ui.send(TheEvent::GainedFocus(focus_id));
                ui.process_events(ctx);
            }
            return true;
        }

        false
    }

    fn supports_actions(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concise_editor_commands_use_stable_ids() {
        assert_eq!(
            ConsoleDock::parse_command("action face.extrude amount = 2").unwrap(),
            vec![ConsoleRequest::RunAction(EditorActionRequest {
                command_id: "face.extrude".to_string(),
                parameters_toml: "amount = 2".to_string(),
            })]
        );
        assert_eq!(
            ConsoleDock::parse_command("tool geometry").unwrap(),
            vec![ConsoleRequest::SelectTool {
                command_id: "tool.geometry".to_string(),
            }]
        );
    }

    #[test]
    fn concise_game_queries_remain_small() {
        assert_eq!(
            ConsoleDock::parse_command("list chars").unwrap(),
            vec![ConsoleRequest::List(ConsoleListScope::Characters)]
        );
        assert_eq!(
            ConsoleDock::parse_command("focus \"Old Smuggler\"").unwrap(),
            vec![ConsoleRequest::Focus("Old Smuggler".to_string())]
        );
        assert!(ConsoleDock::parse_command("rules overview").is_err());
    }

    #[test]
    fn eldrin_console_scripts_emit_ordered_typed_requests() {
        let requests = ConsoleDock::parse_command(
            r#"eldrin console_list("characters"); editor_tool("tool.geometry"); editor_action("camera.isometric", "");"#,
        )
        .unwrap();
        assert_eq!(
            requests,
            vec![
                ConsoleRequest::List(ConsoleListScope::Characters),
                ConsoleRequest::SelectTool {
                    command_id: "tool.geometry".to_string(),
                },
                ConsoleRequest::RunAction(EditorActionRequest {
                    command_id: "camera.isometric".to_string(),
                    parameters_toml: String::new(),
                }),
            ]
        );
    }
}
