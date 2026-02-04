use crate::prelude::*;

// Some code taken from https://github.com/ceronman/loxido/blob/master/src/compiler.rs
// Licensed under the MIT license of Manuel Cerón.

#[derive(PartialOrd, PartialEq, Copy, Clone, Debug)]
enum ThePrecedence {
    None,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl ThePrecedence {
    fn next_higher(&self) -> ThePrecedence {
        match self {
            ThePrecedence::None => ThePrecedence::Assignment,
            ThePrecedence::Assignment => ThePrecedence::Or,
            ThePrecedence::Or => ThePrecedence::And,
            ThePrecedence::And => ThePrecedence::Equality,
            ThePrecedence::Equality => ThePrecedence::Comparison,
            ThePrecedence::Comparison => ThePrecedence::Term,
            ThePrecedence::Term => ThePrecedence::Factor,
            ThePrecedence::Factor => ThePrecedence::Unary,
            ThePrecedence::Unary => ThePrecedence::Call,
            ThePrecedence::Call => ThePrecedence::Primary,
            ThePrecedence::Primary => ThePrecedence::None,
        }
    }
}

type ParseFn = fn(&mut TheCompiler, can_assign: bool) -> ();

#[derive(Copy, Clone, Debug)]
struct TheParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: ThePrecedence,
}

impl TheParseRule {
    fn new(
        prefix: Option<ParseFn>,
        infix: Option<ParseFn>,
        precedence: ThePrecedence,
    ) -> TheParseRule {
        TheParseRule {
            prefix,
            infix,
            precedence,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TheCompilerError {
    // This stack is only used for verification during compilation.
    pub location: (u16, u16),
    pub message: String,
}

impl TheCompilerError {
    pub fn new(location: (u16, u16), message: String) -> Self {
        Self { location, message }
    }
}

#[derive(Clone, Debug)]
pub struct TheCompilerContext {
    // This stack is only used for verification during compilation.
    pub stack: Vec<TheValue>,
    pub local: Vec<TheCodeObject>,

    pub previous_location: (u16, u16),
    pub current_location: (u16, u16),
    pub node_location: (u16, u16),

    pub blocks: Vec<TheCodeNode>,

    pub current: TheCodeAtom,
    pub previous: TheCodeAtom,

    pub module: TheCodeModule,
    pub functions: Vec<TheCodeFunction>,
    pub curr_function_index: usize,

    // Needed for multi comparison support
    pub last_comparison_indent: Option<u16>,
    pub last_comparison_to: Option<TheCodeAtom>,

    pub error: Option<TheCompilerError>,
    pub external_call: Option<(TheCodeNodeCall, Vec<TheValue>)>,
}

impl Default for TheCompilerContext {
    fn default() -> Self {
        TheCompilerContext::new()
    }
}

impl TheCompilerContext {
    pub fn new() -> Self {
        Self {
            stack: vec![],
            local: vec![TheCodeObject::default()],

            blocks: vec![],

            previous_location: (0, 0),
            current_location: (0, 0),
            node_location: (0, 0),

            current: TheCodeAtom::EndOfCode,
            previous: TheCodeAtom::EndOfCode,

            module: TheCodeModule::default(),
            functions: vec![TheCodeFunction::default()],
            curr_function_index: 0,

            last_comparison_indent: None,
            last_comparison_to: None,

            error: None,
            external_call: None,
        }
    }

    /// Returns the current function.
    pub fn get_current_function(&mut self) -> &mut TheCodeFunction {
        if self.functions.is_empty() {
            println!("No function found.");
            self.functions.push(TheCodeFunction::default());
        }
        &mut self.functions[self.curr_function_index]
    }

    /// Add a function.
    pub fn add_function(&mut self, function: TheCodeFunction) {
        self.functions.push(function);
        self.curr_function_index += 1;
    }

    /// Removes the last function from the stack and returns it.
    pub fn remove_function(&mut self) -> Option<TheCodeFunction> {
        if self.curr_function_index > 0 {
            self.curr_function_index -= 1
        }
        self.functions.pop()
    }
}

#[derive(Clone, Debug)]
pub struct TheCompiler {
    rules: FxHashMap<TheCodeAtomKind, TheParseRule>,
    grid: TheCodeGrid,

    external_call: FxHashMap<String, (TheCodeNodeCall, Vec<TheValue>)>,
    packages: FxHashMap<Uuid, TheCodePackage>,

    ctx: TheCompilerContext,
}

impl Default for TheCompiler {
    fn default() -> Self {
        TheCompiler::new()
    }
}

impl TheCompiler {
    pub fn new() -> Self {
        let mut rules = FxHashMap::default();

        let mut rule = |kind, prefix, infix, precedence| {
            rules.insert(kind, TheParseRule::new(prefix, infix, precedence));
        };

        use TheCodeAtomKind::*;
        use ThePrecedence as P;

        rule(Number, Some(TheCompiler::number), None, P::None);
        rule(Plus, None, Some(TheCompiler::binary), P::Term);
        rule(Minus, None, Some(TheCompiler::binary), P::Term);
        rule(Star, None, Some(TheCompiler::binary), P::Factor);
        rule(Slash, None, Some(TheCompiler::binary), P::Factor);
        rule(Percent, None, Some(TheCompiler::binary), P::Factor);
        rule(Eof, None, None, P::None);
        rule(Return, None, None, P::None);
        rule(Semicolon, None, None, P::None);
        rule(Identifier, Some(TheCompiler::variable), None, P::None);

        Self {
            rules,
            grid: TheCodeGrid::default(),
            external_call: FxHashMap::default(),
            packages: FxHashMap::default(),
            ctx: TheCompilerContext::default(),
        }
    }

    /// Add an external node (a function provided by the host) to the compiler.
    pub fn add_external_call(
        &mut self,
        name: String,
        call: TheCodeNodeCall,
        values: Vec<TheValue>,
    ) {
        self.external_call.insert(name, (call, values));
    }

    /// Add the code packages to the compiler. These are used to verify module calls.
    pub fn set_packages(&mut self, packages: FxHashMap<Uuid, TheCodePackage>) {
        self.packages = packages;
    }

    /// Updates a code package.
    pub fn update_package(&mut self, package: TheCodePackage) {
        self.packages.insert(package.id, package);
    }

    /// Compile the given code grid and returns either a module or an error.
    pub fn compile(&mut self, grid: &mut TheCodeGrid) -> Result<TheCodeModule, TheCompilerError> {
        self.ctx = TheCompilerContext::default();

        grid.clear_messages();
        self.grid = grid.clone();

        self.advance();

        while !self.matches(TheCodeAtomKind::Eof) && self.ctx.error.is_none() {
            if self.ctx.current_location.1 % 2 == 1 {
                self.advance();
                continue;
            }

            // Handling indention. Close blocks if needed.
            let indent = self.ctx.blocks.len();
            if indent > 0
                && self.ctx.current_location.0 == 0
                && self.ctx.current_location.1 > self.ctx.previous_location.1
            {
                // We are at the start of a new line, check if we have a block to close.
                #[allow(clippy::collapsible_if)]
                for code_index in 0..=indent * 2 {
                    if self
                        .grid
                        .code
                        .contains_key(&(code_index as u16, self.ctx.current_location.1))
                    {
                        // On even indents check for a block(s) to close.
                        if code_index % 2 == 0 {
                            // Amount of blocks we have to close due to the indentation.
                            let closing = indent - code_index / 2;
                            for _ in 0..closing {
                                // Closing the block.
                                if let Some(function) = self.ctx.remove_function() {
                                    if let Some(mut node) = self.ctx.blocks.pop() {
                                        node.data.sub_functions.push(function);
                                        self.ctx.get_current_function().add_node(node);
                                    }
                                }
                                // Clear the last comparison meta data.
                                self.ctx.last_comparison_indent = None;
                                self.ctx.last_comparison_to = None;
                            }
                            break;
                        }
                        let mut cond: Option<TheValueComparison> = None;

                        // On uneven lines check for multi comparisons.
                        if let Some(TheCodeAtom::Comparison(op)) = self
                            .grid
                            .code
                            .get(&(code_index as u16, self.ctx.current_location.1))
                        {
                            if let Some(last_comparison_indent) = self.ctx.last_comparison_indent {
                                if let Some(last_comparison_to) =
                                    self.ctx.last_comparison_to.clone()
                                {
                                    if code_index == last_comparison_indent as usize {
                                        // Fist, close the current comparison block
                                        if let Some(function) = self.ctx.remove_function() {
                                            if let Some(mut node) = self.ctx.blocks.pop() {
                                                node.data.sub_functions.push(function);
                                                self.ctx.get_current_function().add_node(node);
                                            }
                                        }

                                        // Write the node we compare to, to the stack again.
                                        if let Some(node) =
                                            last_comparison_to.clone().to_node(&mut self.ctx)
                                        {
                                            self.ctx.get_current_function().add_node(node);
                                        }

                                        cond = Some(*op);
                                    }
                                }
                            }

                            // Write out the new conditional block.
                            if let Some(cond) = cond {
                                // Load the conditional value

                                let func = TheCodeFunction::default();
                                self.ctx.add_function(func);

                                self.ctx.node_location =
                                    (code_index as u16, self.ctx.current_location.1);

                                self.advance();
                                self.advance();
                                self.expression();
                                // Write the comparison function which will take the current function as a sub.
                                if let Some(node) =
                                    TheCodeAtom::Comparison(cond).to_node(&mut self.ctx)
                                {
                                    let func = TheCodeFunction::default();
                                    self.ctx.add_function(func);

                                    // We indent one
                                    self.ctx.blocks.push(node);
                                }
                            }
                        }
                    }
                }
            }

            self.declaration();
        }

        // Close all open blocks.
        let mut indent = self.ctx.blocks.len();
        while indent > 0 {
            if let Some(function) = self.ctx.remove_function() {
                if let Some(mut node) = self.ctx.blocks.pop() {
                    node.data.sub_functions.push(function);
                    self.ctx.get_current_function().add_node(node);
                }
            } else {
                // TODO ERROR MESSAGE: Too many open blocks at the end of the code.
            }
            indent -= 1;
        }

        if let Some(error) = &self.ctx.error {
            println!("Error: {:?}", error);
            grid.add_message(
                error.location,
                TheCodeGridMessage {
                    message_type: TheCodeGridMessageType::Error,
                    message: error.message.clone(),
                },
            );
            Err(error.clone())
        } else {
            if !self.ctx.get_current_function().is_empty() {
                let f = self.ctx.get_current_function().clone();
                self.ctx.module.set_function(f);
            }

            self.ctx.module.name.clone_from(&grid.name);
            self.ctx.module.codegrid_id = grid.id;
            Ok(self.ctx.module.clone())
        }
    }

    fn declaration(&mut self) {
        //println!("declaration {:?}", self.ctx.current);

        match self.ctx.current.clone() {
            /*
            TheCodeAtom::FuncDef(name) => {
                self.advance();
                let mut func = TheCodeFunction::named(name);
                let mut arguments = vec![];
                while let TheCodeAtom::FuncArg(arg_name) = self.ctx.current.clone() {
                    if let Some(local) = self.ctx.local.last_mut() {
                        local.set(arg_name.clone(), TheValue::Int(0));
                    }
                    arguments.push(arg_name.clone());
                    self.advance();
                }
                func.arguments = arguments;
                self.ctx.add_function(func);
            }*/
            TheCodeAtom::LocalSet(name, _) => {
                self.advance();
                let var; // = self.ctx.previous.clone();
                let location = self.ctx.previous_location;

                match &self.ctx.current {
                    TheCodeAtom::Assignment(op) => {
                        var = TheCodeAtom::LocalSet(name, *op);
                        self.advance();
                    }
                    _ => {
                        self.error_at(
                            (
                                self.ctx.previous_location.0 + 1,
                                self.ctx.previous_location.1,
                            ),
                            "Expected assignment operator.",
                        );
                        return;
                    }
                }

                self.expression();
                self.ctx.node_location = location;
                if let Some(node) = var.to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtom::ObjectSet(_, _, _) => {
                self.advance();
                let var = self.ctx.previous.clone();
                let location = self.ctx.previous_location;

                match &self.ctx.current {
                    TheCodeAtom::Assignment(_op) => {
                        self.advance();
                    }
                    _ => {
                        self.error_at(
                            (
                                self.ctx.previous_location.0 + 1,
                                self.ctx.previous_location.1,
                            ),
                            "Expected assignment operator.",
                        );
                        return;
                    }
                }

                self.expression();
                self.ctx.node_location = location;
                if let Some(node) = var.to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtom::Set(path, _) => {
                self.advance();
                let var;
                let location = self.ctx.previous_location;

                match &self.ctx.current {
                    TheCodeAtom::Assignment(op) => {
                        var = TheCodeAtom::Set(path, *op);
                        self.advance();
                    }
                    _ => {
                        self.error_at(
                            (
                                self.ctx.previous_location.0 + 1,
                                self.ctx.previous_location.1,
                            ),
                            "Expected assignment operator.",
                        );
                        return;
                    }
                }

                self.expression();
                self.ctx.node_location = location;
                if let Some(node) = var.to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            _ => {
                self.statement();
            }
        }
    }

    fn statement(&mut self) {
        match self.ctx.current.clone() {
            TheCodeAtom::Argument(name) => {
                // Add the argument to the current function.
                self.ctx.get_current_function().arguments.push(name);
            }
            TheCodeAtom::ExternalCall(_name, _, _, arg_values, _) => {
                self.advance();
                let external_call = self.ctx.previous.clone();
                let location: (u16, u16) = self.ctx.previous_location;
                self.ctx.node_location = location;

                for (index, _) in arg_values.iter().enumerate() {
                    let off = location.0 + (index + 1) as u16 * 2;

                    if !matches!(self.ctx.current, TheCodeAtom::EndOfExpression) {
                        self.error_at(
                            (self.ctx.current_location.0, self.ctx.current_location.1),
                            "Unexpected code inside function call.",
                        );
                        return;
                    }

                    self.advance();

                    // Check if function argument value at the right position.
                    if self.ctx.current_location.0 != off
                        || self.ctx.current_location.1 != location.1
                    {
                        self.error_at((off, location.1), "Expected value at this position.");
                        return;
                    }

                    match &self.ctx.current {
                        TheCodeAtom::Value(_)
                        | TheCodeAtom::Get(_)
                        | TheCodeAtom::LocalGet(_)
                        | TheCodeAtom::ObjectGet(_, _)
                        | TheCodeAtom::RandInt(_)
                        | TheCodeAtom::RandFloat(_) => {
                            // Add the function argument to the stack.
                            if let Some(node) = self.ctx.current.clone().to_node(&mut self.ctx) {
                                self.ctx.get_current_function().add_node(node);
                            }
                        }
                        _ => {
                            self.error_at(
                                (self.ctx.current_location.0, self.ctx.current_location.1),
                                "Expected Value.",
                            );
                            return;
                        }
                    }
                    self.advance();
                }

                if let TheCodeAtom::ExternalCall(name, _, _, _, _) = &external_call {
                    if let Some(call) = self.external_call.get(name) {
                        self.ctx.external_call = Some(call.clone());
                        if let Some(node) = external_call.to_node(&mut self.ctx) {
                            self.ctx.get_current_function().add_node(node);
                        }
                        self.ctx.external_call = None;
                    } else {
                        self.error_at(
                            (location.0, location.1),
                            format!("Unknown external call ({}).", name).as_str(),
                        );
                    }
                }
            }
            TheCodeAtom::ModuleCall(_bundle_name, bundle_id, _module_name, codegrid_id) => {
                self.advance();
                let module_call = self.ctx.previous.clone();
                let location: (u16, u16) = self.ctx.previous_location;
                self.ctx.node_location = location;

                /*
                for (index, _) in arg_values.iter().enumerate() {
                    let off = location.0 + (index + 1) as u16 * 2;

                    if !matches!(self.ctx.current, TheCodeAtom::EndOfExpression) {
                        self.error_at(
                            (self.ctx.current_location.0, self.ctx.current_location.1),
                            "Unexpected code inside function call.",
                        );
                        return;
                    }

                    self.advance();

                    // Check if function argument value at the right position.
                    if self.ctx.current_location.0 != off
                        || self.ctx.current_location.1 != location.1
                    {
                        self.error_at((off, location.1), "Expected value at this position.");
                        return;
                    }

                    match &self.ctx.current {
                        TheCodeAtom::Value(_)
                        | TheCodeAtom::LocalGet(_)
                        | TheCodeAtom::ObjectGet(_, _) => {
                            // Add the function argument to the stack.
                            if let Some(node) = self.ctx.current.clone().to_node(&mut self.ctx) {
                                self.ctx.get_current_function().add_node(node);
                            }
                        }
                        _ => {
                            self.error_at(
                                (self.ctx.current_location.0, self.ctx.current_location.1),
                                "Expected Value.",
                            );
                            return;
                        }
                    }
                    self.advance();
                }*/

                let mut found_module = false;
                if let TheCodeAtom::ModuleCall(_, _, _, module_name) = &module_call {
                    //self.ctx.external_call = Some(call.clone());

                    if let Some(package) = self.packages.get(&bundle_id) {
                        if let Some(_module) = package.get_function_codegrid(&codegrid_id) {
                            found_module = true;
                            if let Some(node) = module_call.to_node(&mut self.ctx) {
                                self.ctx.get_current_function().add_node(node);
                            }
                        }
                    }

                    if !found_module {
                        self.error_at(
                            (location.0, location.1),
                            format!("Unknown module call ({}).", module_name).as_str(),
                        );
                    }
                }
            }
            TheCodeAtom::Value(_)
            | TheCodeAtom::Get(_)
            | TheCodeAtom::LocalGet(_)
            | TheCodeAtom::ObjectGet(_, _)
            | TheCodeAtom::RandInt(_)
            | TheCodeAtom::RandFloat(_) => {
                self.advance();
                let mut comparison = TheCodeAtom::Comparison(TheValueComparison::Equal);
                let location: (u16, u16) = self.ctx.current_location;

                match &self.ctx.current.clone() {
                    TheCodeAtom::Comparison(op) => {
                        // Write the node to the stack if the next operation is a comparison
                        if let Some(node) = self.ctx.previous.clone().to_node(&mut self.ctx) {
                            self.ctx.get_current_function().add_node(node);

                            // Save the meta data in case we have a multi comparison.
                            self.ctx.last_comparison_indent = Some(location.0);
                            self.ctx.last_comparison_to = Some(self.ctx.previous.clone());
                        }

                        comparison = TheCodeAtom::Comparison(*op);
                        self.advance();
                    }
                    _ => {
                        if self.ctx.previous_location.0 == self.ctx.blocks.len() as u16 * 2 {
                            self.error_at(
                                (
                                    self.ctx.previous_location.0 + 1,
                                    self.ctx.previous_location.1,
                                ),
                                "Expected comparison operator.",
                            );
                            return;
                        }
                    }
                }

                // Load the conditional value

                let func = TheCodeFunction::default();
                self.ctx.add_function(func);

                self.expression();
                self.ctx.node_location = location;
                // Write the comparison function which will take the current function as a sub.
                if let Some(node) = comparison.to_node(&mut self.ctx) {
                    let func = TheCodeFunction::default();
                    self.ctx.add_function(func);

                    // We indent one
                    self.ctx.blocks.push(node);
                }
            }
            TheCodeAtom::Return => {
                self.advance();
                let location: (u16, u16) = self.ctx.previous_location;
                let return_node = self.ctx.previous.clone();
                self.advance();

                // if self
                //     .grid
                //     .code
                //     .contains_key(&(self.ctx.pre.0 + 2, self.ctx.current_location.1))
                // {
                self.expression();
                //}

                self.ctx.node_location = location;
                if let Some(node) = return_node.to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtom::EndOfExpression => {
                self.advance();
            }
            _ => {
                self.advance();
                self.ctx.error = Some(TheCompilerError::new(
                    self.ctx.current_location,
                    "Unexpected code.".to_string(),
                ));
            }
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(ThePrecedence::Assignment);
    }

    fn variable(&mut self, _can_assing: bool) {
        match self.ctx.previous.clone() {
            TheCodeAtom::Get(_name) => {
                if let Some(node) = self.ctx.previous.clone().to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtom::LocalGet(_name) => {
                if let Some(node) = self.ctx.previous.clone().to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtom::ObjectGet(_, _) => {
                if let Some(node) = self.ctx.previous.clone().to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtom::ExternalCall(_name, _, _, arg_values, _) => {
                let external_call = self.ctx.previous.clone();
                let location: (u16, u16) = self.ctx.previous_location;
                self.ctx.node_location = location;

                for (index, _) in arg_values.iter().enumerate() {
                    let off = location.0 + (index + 1) as u16 * 2;

                    if !matches!(self.ctx.current, TheCodeAtom::EndOfExpression) {
                        self.error_at(
                            (self.ctx.current_location.0, self.ctx.current_location.1),
                            "Unexpected code inside function call.",
                        );
                        return;
                    }

                    self.advance();

                    // Check if function argument value at the right position.
                    if self.ctx.current_location.0 != off
                        || self.ctx.current_location.1 != location.1
                    {
                        self.error_at((off, location.1), "Expected value at this position.");
                        return;
                    }

                    match &self.ctx.current {
                        TheCodeAtom::Value(_)
                        | TheCodeAtom::Get(_)
                        | TheCodeAtom::LocalGet(_)
                        | TheCodeAtom::ObjectGet(_, _)
                        | TheCodeAtom::RandInt(_)
                        | TheCodeAtom::RandFloat(_) => {
                            // Add the function argument to the stack.
                            if let Some(node) = self.ctx.current.clone().to_node(&mut self.ctx) {
                                self.ctx.get_current_function().add_node(node);
                            }
                        }
                        _ => {
                            self.error_at(
                                (self.ctx.current_location.0, self.ctx.current_location.1),
                                "Expected Value.",
                            );
                            return;
                        }
                    }
                    self.advance();
                }

                if let TheCodeAtom::ExternalCall(name, _, _, _, _) = &external_call {
                    if let Some(call) = self.external_call.get(name) {
                        self.ctx.external_call = Some(call.clone());
                        if let Some(node) = external_call.to_node(&mut self.ctx) {
                            self.ctx.get_current_function().add_node(node);
                        }
                        self.ctx.external_call = None;
                    } else {
                        self.error_at(
                            (location.0, location.1),
                            format!("Unknown external call ({}).", name).as_str(),
                        );
                    }
                }
            }
            /*
            TheCodeAtom::FuncCall(_) => {
                let node = self.ctx.previous.clone().to_node(&mut self.ctx);
                //println!("FuncCall {:?}", self.ctx.current_location);

                let arg_loc = (self.ctx.current_location.0, self.ctx.current_location.1 + 1);

                if let Some(arg) = self.grid.code.get(&arg_loc).cloned() {
                    if let Some(arg_node) = arg.clone().to_node(&mut self.ctx) {
                        self.ctx.get_current_function().add_node(arg_node);
                    }

                    self.grid.code.remove(&arg_loc);
                }

                if let Some(node) = node {
                    self.ctx.get_current_function().add_node(node);
                }
            }*/
            _ => {
                self.error_at_current("Unknown identifier.");
            }
        }
    }

    fn number(&mut self, _can_assign: bool) {
        if let Some(node) = self.ctx.previous.clone().to_node(&mut self.ctx) {
            self.ctx.get_current_function().add_node(node);
        }
    }

    fn binary(&mut self, _can_assign: bool) {
        let operator_type = self.ctx.previous.to_kind();

        let rule = self.get_rule(operator_type);
        self.parse_precedence(rule.precedence.next_higher());

        match operator_type {
            TheCodeAtomKind::Plus => {
                if let Some(node) = TheCodeAtom::Add.to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtomKind::Minus => {
                if let Some(node) = TheCodeAtom::Subtract.to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtomKind::Star => {
                if let Some(node) = TheCodeAtom::Multiply.to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtomKind::Slash => {
                if let Some(node) = TheCodeAtom::Divide.to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            TheCodeAtomKind::Percent => {
                if let Some(node) = TheCodeAtom::Modulus.to_node(&mut self.ctx) {
                    self.ctx.get_current_function().add_node(node);
                }
            }
            _ => {}
        }
    }

    fn get_rule(&self, kind: TheCodeAtomKind) -> TheParseRule {
        self.rules.get(&kind).cloned().unwrap()
    }

    fn parse_precedence(&mut self, precedence: ThePrecedence) {
        self.advance();

        let prefix_rule = self.get_rule(self.ctx.previous.to_kind()).prefix;
        let can_assign = precedence <= ThePrecedence::Assignment;

        if let Some(prefix_rule) = prefix_rule {
            prefix_rule(self, can_assign);
        } else {
            //self.error("Expect expression.");
            return;
        }

        while precedence <= self.get_rule(self.ctx.current.to_kind()).precedence {
            if self.ctx.error.is_some() {
                return;
            }

            self.advance();
            let infix_rule = self.get_rule(self.ctx.previous.to_kind()).infix;

            if let Some(infix_rule) = infix_rule {
                infix_rule(self, can_assign);
            }
        }

        if can_assign && self.matches(TheCodeAtomKind::Equal) {
            //self.error("Invalid assignment target.");
        }
    }

    /// Advance one token
    fn advance(&mut self) {
        self.ctx.previous = self.ctx.current.clone();
        self.ctx.previous_location = self.ctx.current_location;

        self.ctx.current = self.grid.get_next(false);

        if let Some(location) = self.grid.current_pos {
            self.ctx.current_location = location;
        }

        //println!("({:?} : {:?}), ({:?} : {:?})", self.ctx.previous, self.ctx.previous_location, self.grid.current_pos, self.ctx.current);
    }

    fn matches(&mut self, kind: TheCodeAtomKind) -> bool {
        if !self.check(kind) {
            false
        } else {
            self.advance();
            true
        }
    }

    fn check(&self, kind: TheCodeAtomKind) -> bool {
        self.ctx.current.to_kind() == kind
    }

    /// Create an error at the current parser location.
    fn error_at_current(&mut self, message: &str) {
        self.ctx.error = Some(TheCompilerError::new(
            self.ctx.current_location,
            message.to_string(),
        ));
    }

    /// Create an error at the given parser location.
    fn error_at(&mut self, location: (u16, u16), message: &str) {
        self.ctx.error = Some(TheCompilerError::new(location, message.to_string()));
    }
    /*
    /// Error at the current token
    fn error_at_current(&mut self, message: &str) {
        self.error_at(self.parser.current.clone(), message)
    }

    /// Error at the previous token
    fn error(&mut self, message: &str) {
        self.error_at(self.parser.previous.clone(), message)
    }

    /// Error at the given token
    fn error_at(&mut self, _token: TheCodeAtom, message: &str) {
        println!("error {}", message);
        if self.parser.panic_mode {
            return;
        }
        self.parser.panic_mode = true;
        self.parser.error_message = message.to_owned();
        //self.parser.error_line = self.parser.previous.line;
        self.parser.had_error = true;
    }*/
}
