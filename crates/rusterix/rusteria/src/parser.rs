use crate::zero_expr_float;
use crate::{
    ASTValue, AssignmentOperator, BinaryOperator, ComparisonOperator, EqualityOperator, Expr,
    IdVerifier, Location, LogicalOperator, Module, ParseError, Scanner, Stmt, Token, TokenType,
    UnaryOperator, objectd::FunctionD,
};
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};

#[derive(PartialEq, Debug)]
enum VariableScope {
    Global,
    Local,
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    current_line: usize,
    path: PathBuf,
    verifier: IdVerifier,

    scope: VariableScope,

    // Store the indices of the global variables.
    globals_map: FxHashMap<String, u32>,

    // Store the indices of the local variables inside a fn.
    locals_map: IndexMap<String, Option<Box<Expr>>>,

    /// User defined function names
    function_names: FxHashSet<String>,

    /// Strings
    strings: Vec<String>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            current: 0,
            current_line: 0,
            path: PathBuf::new(),
            verifier: IdVerifier::default(),

            scope: VariableScope::Global,

            globals_map: FxHashMap::default(),
            locals_map: IndexMap::default(),

            function_names: FxHashSet::default(),
            strings: vec![],
        }
    }

    /// Compile the main source module.
    pub fn compile(&mut self, path: PathBuf) -> Result<Module, ParseError> {
        if let Ok(source) = std::fs::read_to_string(path.clone()) {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                self.compile_module(stem.to_string(), source, path)
            } else {
                Err(ParseError::new("Could not read file", 0, &path))
            }
        } else {
            Err(ParseError::new("Could not read file", 0, &path))
        }
    }

    /// Compile a module with the given name, source code, and path.
    pub fn compile_module(
        &mut self,
        name: String,
        source: String,
        path: PathBuf,
    ) -> Result<Module, ParseError> {
        let mut scanner = Scanner::new(source.clone());
        let mut tokens = vec![];
        loop {
            let token = scanner.scan_token();
            if token.kind == TokenType::Eof {
                //tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        self.tokens = tokens;
        self.path = path.clone();

        // Collect statements
        let mut statements = vec![];

        while !self.is_at_end() {
            let stmt = self.declaration()?;
            statements.push(Box::new(stmt));
        }

        let module = Module::new(
            name,
            source,
            self.path.clone(),
            statements,
            self.globals_map.clone(),
            self.strings.clone(),
        );

        Ok(module)
    }

    fn declaration(&mut self) -> Result<Stmt, ParseError> {
        if self.match_token(vec![TokenType::Import]) {
            return self.import_statement();
        }
        if self.match_token(vec![TokenType::Let]) {
            return self.var_declaration();
        }
        if self.match_token(vec![TokenType::Fn]) {
            return self.fn_declaration();
        }

        self.statement()
    }

    fn var_declaration(&mut self) -> Result<Stmt, ParseError> {
        let line = self.current_line;
        let var_name = self
            .consume(TokenType::Identifier, "Expect variable name", line)?
            .lexeme;

        if self.scope == VariableScope::Global {
            _ = self.verifier.define_var(&var_name, false)?;
            if !self.globals_map.contains_key(&var_name) {
                self.globals_map
                    .insert(var_name.clone(), self.globals_map.len() as u32);
            }
        } else {
            if !self.locals_map.contains_key(&var_name) {
                self.locals_map.insert(var_name.clone(), None);
            }
        }

        let initializer;
        if self.match_token(vec![TokenType::Equal]) {
            initializer = Some(self.expression()?);
        } else {
            return Err(ParseError::new(
                "Expected '=' after variable name",
                line,
                &self.path,
            ));
        }

        let init = if let Some(i) = initializer {
            Box::new(i)
        } else {
            return Err(ParseError::new(
                "Variable declaration cannot be empty",
                line,
                &self.path,
            ));
        };

        /*
        if self.check(TokenType::Comma) {
            self.consume(
                TokenType::Comma,
                &format!(
                    "Expect ',' after variable declaration, found '{}'",
                    self.lexeme(),
                ),
                line,
            )?;
            self.open_var_declaration = Some(static_type.clone());
        } else {
            self.open_var_declaration = None;
            if !self.inside_for_initializer {
                self.consume(
                    TokenType::Semicolon,
                    &format!(
                        "Expect ';' after variable declaration, found '{}'",
                        self.lexeme(),
                    ),
                    line,
                )?;
            }
        }*/

        Ok(Stmt::VarDeclaration(
            var_name,
            ASTValue::None,
            init,
            self.create_loc(line),
        ))
    }

    /// Import statement
    fn import_statement(&mut self) -> Result<Stmt, ParseError> {
        let line = self.current_line;
        self.consume(
            TokenType::String,
            "Expected path string after 'import''",
            self.current_line,
        )?;

        let str = self.previous().unwrap().lexeme.clone().replace("\"", "");
        self.consume(
            TokenType::Semicolon,
            "Expect ';' after import statement",
            line,
        )?;

        // Resolving the path

        fn resolve_import(base_path: &Path, import_path: &str) -> PathBuf {
            let base_dir = base_path.parent().unwrap_or_else(|| Path::new(""));
            base_dir.join(import_path)
        }
        let path = resolve_import(&self.path, &str);

        let mut module = None;

        if let Ok(source) = std::fs::read_to_string(path.clone()) {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let mut parser = Parser::new();
                let m = parser.compile_module(stem.to_string(), source, path)?;

                // Add imported function names to the parser
                for name in parser.function_names {
                    self.function_names.insert(name);
                }

                // Import variables
                for (name, mat) in parser.globals_map {
                    self.globals_map
                        .insert(name, mat + self.globals_map.len() as u32);
                }

                module = Some(m);
            } else {
                return Err(ParseError::new("Could not read import file", 0, &path));
            }
        }

        Ok(Stmt::Import(module, self.create_loc(line)))
    }

    /// Function declaration
    fn fn_declaration(&mut self) -> Result<Stmt, ParseError> {
        let line = self.current_line;
        self.consume(
            TokenType::Identifier,
            "Expected identifier after 'fn''",
            self.current_line,
        )?;

        let id = self.previous().unwrap().lexeme.clone();

        self.consume(
            TokenType::LeftParen,
            "Expected `(` after function name",
            self.current_line,
        )?;

        self.locals_map = IndexMap::default();

        while self.match_token(vec![TokenType::Identifier]) {
            let id = self.previous().unwrap().lexeme.clone();
            let mut param_value: Option<Box<Expr>> = None;

            if self.tokens[self.current].kind == TokenType::Equal {
                self.consume(
                    TokenType::Equal,
                    "Expected '=' after parameter identifier",
                    self.current_line,
                )?;

                param_value = Some(Box::new(self.expression()?));
            }
            self.locals_map.insert(id, param_value);

            if self.tokens[self.current].kind == TokenType::Comma {
                self.advance();
            }
        }

        self.consume(
            TokenType::RightParen,
            "Expected ')' after function parameters",
            self.current_line,
        )?;

        self.consume(
            TokenType::LeftBrace,
            "Expected '{' after function header",
            self.current_line,
        )?;

        let arity = self.locals_map.len();
        self.function_names.insert(id.clone());

        self.scope = VariableScope::Local;
        let block = self.block()?;
        self.scope = VariableScope::Global;

        Ok(Stmt::FunctionDeclaration(
            FunctionD::new(id, arity, self.locals_map.clone(), Box::new(block)),
            self.create_loc(line),
        ))
    }

    fn block(&mut self) -> Result<Stmt, ParseError> {
        let mut statements = vec![];

        self.verifier.begin_scope();

        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            match self.declaration() {
                Ok(stmt) => {
                    statements.push(Box::new(stmt));
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }

        self.verifier.end_scope();

        let line = self.current_line;

        self.consume(TokenType::RightBrace, "Expect '}}' after block", line)?;

        Ok(Stmt::Block(statements, self.create_loc(line)))
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_token(vec![TokenType::If]) {
            self.if_statement()
        } else if self.match_token(vec![TokenType::LeftBrace]) {
            self.block()
        } else if self.match_token(vec![TokenType::Return]) {
            self.return_statement()
        } else if self.match_token(vec![TokenType::For]) {
            self.for_statement()
        } else {
            self.expression_statement()
        }
    }

    fn for_statement(&mut self) -> Result<Stmt, ParseError> {
        let line = self.current_line;
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'", line)?;

        let mut inits: Vec<Box<Stmt>> = vec![];

        // self.inside_for_initializer = true;
        loop {
            let i = self.declaration()?;
            inits.push(Box::new(i));

            if !self.match_token(vec![TokenType::Comma]) {
                break;
            }
        }
        // self.inside_for_initializer = false;

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after loop initializer",
            line,
        )?;

        let mut conditions: Vec<Box<Expr>> = vec![];

        loop {
            let c = self.expression()?;
            conditions.push(Box::new(c));

            if !self.match_token(vec![TokenType::Comma]) {
                break;
            }
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after loop condition",
            line,
        )?;

        let mut incrs: Vec<Box<Expr>> = vec![];

        loop {
            let c = self.expression()?;
            incrs.push(Box::new(c));

            if !self.match_token(vec![TokenType::Comma]) {
                break;
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after for loop", line)?;

        let body = self.statement()?;

        Ok(Stmt::For(
            inits,
            conditions,
            incrs,
            Box::new(body),
            self.create_loc(line),
        ))
    }

    fn expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let value: Expr = self.expression()?;
        let line = self.current_line;
        self.consume(TokenType::Semicolon, "Expect ';' after expression", line)?;
        Ok(Stmt::Expression(Box::new(value), self.create_loc(line)))
    }

    fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        let line = self.current_line;
        let condition = self.expression()?;
        let then_branch = self.statement()?;
        let else_branch = if self.match_token(vec![TokenType::Else]) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };

        Ok(Stmt::If(
            Box::new(condition),
            Box::new(then_branch),
            else_branch,
            self.create_loc(line),
        ))
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.or()?;

        if self.check(TokenType::Plus)
            && self.match_token(vec![TokenType::Plus])
            && self.match_token(vec![TokenType::Equal])
        {
            let equals = self.previous().unwrap();
            let value = self.assignment()?;

            if let Expr::Variable(name, swizzle, field_path, _loc) = expr {
                return Ok(Expr::VariableAssignment(
                    name,
                    AssignmentOperator::AddAssign,
                    swizzle.clone(),
                    field_path.clone(),
                    Box::new(value),
                    self.create_loc(equals.line),
                ));
            }

            return Err(ParseError::new(
                format!("Invalid assignment target: '{:?}'", equals.lexeme),
                equals.line,
                &self.path,
            ));
        } else if self.check(TokenType::Minus)
            && self.match_token(vec![TokenType::Minus])
            && self.match_token(vec![TokenType::Equal])
        {
            let equals = self.previous().unwrap();
            let value = self.assignment()?;

            if let Expr::Variable(name, swizzle, field_path, _loc) = expr {
                return Ok(Expr::VariableAssignment(
                    name,
                    AssignmentOperator::SubtractAssign,
                    swizzle.clone(),
                    field_path.clone(),
                    Box::new(value),
                    self.create_loc(equals.line),
                ));
            }

            return Err(ParseError::new(
                format!("Invalid assignment target: '{:?}'", equals.lexeme),
                equals.line,
                &self.path,
            ));
        } else if self.check(TokenType::Star)
            && self.match_token(vec![TokenType::Star])
            && self.match_token(vec![TokenType::Equal])
        {
            let equals = self.previous().unwrap();
            let value = self.assignment()?;

            if let Expr::Variable(name, swizzle, field_path, _loc) = expr {
                return Ok(Expr::VariableAssignment(
                    name,
                    AssignmentOperator::MultiplyAssign,
                    swizzle.clone(),
                    field_path.clone(),
                    Box::new(value),
                    self.create_loc(equals.line),
                ));
            }

            return Err(ParseError::new(
                format!("Invalid assignment target: '{:?}'", equals.lexeme),
                equals.line,
                &self.path,
            ));
        } else if self.check(TokenType::Slash)
            && self.match_token(vec![TokenType::Slash])
            && self.match_token(vec![TokenType::Equal])
        {
            let equals = self.previous().unwrap();
            let value = self.assignment()?;

            if let Expr::Variable(name, swizzle, field_path, _loc) = expr {
                return Ok(Expr::VariableAssignment(
                    name,
                    AssignmentOperator::DivideAssign,
                    swizzle.clone(),
                    field_path.clone(),
                    Box::new(value),
                    self.create_loc(equals.line),
                ));
            }

            return Err(ParseError::new(
                format!("Invalid assignment target: '{:?}'", equals.lexeme),
                equals.line,
                &self.path,
            ));
        } else if self.match_token(vec![TokenType::Equal]) {
            let equals = self.previous().unwrap();
            let value = self.assignment()?;

            if let Expr::Variable(name, swizzle, field_path, _loc) = expr {
                return Ok(Expr::VariableAssignment(
                    name,
                    AssignmentOperator::Assign,
                    swizzle.clone(),
                    field_path.clone(),
                    Box::new(value),
                    self.create_loc(equals.line),
                ));
            }

            return Err(ParseError::new(
                format!("Invalid assignment target: '{:?}'", equals.lexeme),
                equals.line,
                &self.path,
            ));
        }

        Ok(expr)
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.and()?;

        while self.match_token(vec![TokenType::Or]) {
            let operator = self.previous().unwrap();
            let right = self.and()?;
            expr = Expr::Logical(
                Box::new(expr),
                Self::operator_to_logical(operator.kind),
                Box::new(right),
                self.create_loc(operator.line),
            );
        }

        Ok(expr)
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.ternary()?;

        while self.match_token(vec![TokenType::And]) {
            let operator = self.previous().unwrap();
            let right = self.equality()?;
            expr = Expr::Logical(
                Box::new(expr),
                Self::operator_to_logical(operator.kind),
                Box::new(right),
                self.create_loc(operator.line),
            );
        }

        Ok(expr)
    }

    fn ternary(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;
        let line = self.current_line;

        while self.match_token(vec![TokenType::TernaryOperator]) {
            let then_branch = self.expression()?;

            self.consume(
                TokenType::Colon,
                "Expect ':' after condition for ternary",
                line,
            )?;

            let else_branch = self.expression()?;

            expr = Expr::Ternary(
                Box::new(expr),
                Box::new(then_branch),
                Box::new(else_branch),
                self.create_loc(line),
            );
        }

        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.comparison()?;

        while self.match_token(vec![TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator = self.previous().unwrap();
            let right = self.comparison()?;
            expr = Expr::Equality(
                Box::new(expr),
                Self::operator_to_equality(operator.kind),
                Box::new(right),
                self.create_loc(operator.line),
            );
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.term()?;

        while self.match_token(vec![
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let operator = self.previous().unwrap();
            let right = self.term()?;
            expr = Expr::Comparison(
                Box::new(expr),
                Self::operator_to_comparison(operator.kind),
                Box::new(right),
                self.create_loc(operator.line),
            );
        }

        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.factor()?;

        if (self.check(TokenType::Minus) || self.check(TokenType::Plus))
            && !self.check_next(TokenType::Equal)
        {
            while self.match_token(vec![TokenType::Minus, TokenType::Plus]) {
                let operator = self.previous().unwrap();
                let right = self.factor()?;
                expr = Expr::Binary(
                    Box::new(expr),
                    Self::operator_to_binary(operator.kind),
                    Box::new(right),
                    self.create_loc(operator.line),
                );
            }
        }

        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;

        if (self.check(TokenType::Slash)
            || self.check(TokenType::Star)
            || self.check(TokenType::Percent))
            && !self.check_next(TokenType::Equal)
        {
            while self.match_token(vec![TokenType::Slash, TokenType::Star, TokenType::Percent]) {
                let operator = self.previous().unwrap();
                let right = self.unary()?;
                expr = Expr::Binary(
                    Box::new(expr),
                    Self::operator_to_binary(operator.kind),
                    Box::new(right),
                    self.create_loc(operator.line),
                );
            }
        }

        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_token(vec![TokenType::Bang, TokenType::Minus]) {
            let operator = self.previous().unwrap();
            let right = self.unary()?;
            return Ok(Expr::Unary(
                Self::operator_to_unary(operator.kind),
                Box::new(right),
                self.create_loc(operator.line),
            ));
        }

        self.call()
    }

    fn return_statement(&mut self) -> Result<Stmt, ParseError> {
        let value = self.expression()?;
        let line = self.current_line;
        self.consume(TokenType::Semicolon, "Expect ';' after return value", line)?;
        Ok(Stmt::Return(Box::new(value), self.create_loc(line)))
    }

    fn call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(vec![TokenType::LeftParen]) {
                expr = self.finish_call(expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        let mut arguments = vec![];
        let line = self.current_line;

        if !self.check(TokenType::RightParen) {
            loop {
                if arguments.len() >= 255 {
                    return Err(ParseError::new(
                        "Cannot have more than 255 arguments",
                        line,
                        &self.path,
                    ));
                }

                arguments.push(Box::new(self.expression()?));

                if !self.match_token(vec![TokenType::Comma]) {
                    break;
                }
            }
        }

        let paren = self.consume(
            TokenType::RightParen,
            "Expect ')' after function arguments",
            line,
        )?;

        let mut swizzle = vec![];
        let field_path = vec![];
        if self.check(TokenType::Dot) {
            if self.is_swizzle_valid_at_current() {
                swizzle = self.get_swizzle_at_current();
            }
            // else {
            //     field_path = self.get_field_path_at_current();
            // }
        }

        Ok(Expr::FunctionCall(
            Box::new(callee),
            swizzle,
            field_path,
            arguments,
            self.create_loc(paren.line),
        ))
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.peek();
        match token.kind {
            TokenType::String => {
                self.advance();
                let index = self.strings.len() as f32;
                self.strings.push(token.lexeme.clone().replace("\"", ""));
                Ok(Expr::Value(
                    ASTValue::Float(index),
                    vec![],
                    vec![],
                    self.create_loc(token.line),
                ))
            }
            TokenType::False => {
                self.advance();
                Ok(Expr::Value(
                    ASTValue::Boolean(false),
                    vec![],
                    vec![],
                    self.create_loc(token.line),
                ))
            }
            TokenType::True => {
                self.advance();
                Ok(Expr::Value(
                    ASTValue::Boolean(true),
                    vec![],
                    vec![],
                    self.create_loc(token.line),
                ))
            }
            TokenType::Void => {
                self.advance();
                Ok(Expr::Value(
                    ASTValue::None,
                    vec![],
                    vec![],
                    self.create_loc(token.line),
                ))
            }
            TokenType::Semicolon => Ok(Expr::Value(
                ASTValue::None,
                vec![],
                vec![],
                self.create_loc(token.line),
            )),
            TokenType::IntegerNumber => {
                self.advance();
                if let Ok(number) = token.lexeme.parse::<i32>() {
                    Ok(Expr::Value(
                        ASTValue::Float(number as f32),
                        vec![],
                        vec![],
                        self.create_loc(token.line),
                    ))
                } else {
                    Err(ParseError::new(
                        "Invalid integer number",
                        token.line,
                        &self.path,
                    ))
                }
            }
            TokenType::FloatNumber => {
                self.advance();
                if let Ok(number) = token.lexeme.parse::<f32>() {
                    Ok(Expr::Value(
                        ASTValue::Float(number),
                        vec![],
                        vec![],
                        self.create_loc(token.line),
                    ))
                } else {
                    Err(ParseError::new(
                        "Invalid float number",
                        token.line,
                        &self.path,
                    ))
                }
            }
            TokenType::Float2 => {
                self.advance();
                if self.match_token(vec![TokenType::LeftParen]) {
                    let comps = self.read_vec_components(2, token.line)?;
                    let swizzle: Vec<u8> = self.get_swizzle_at_current();

                    Ok(Expr::Value(
                        ASTValue::Float2(
                            if !comps.is_empty() {
                                Box::new(comps[0].clone())
                            } else {
                                zero_expr_float!()
                            },
                            if comps.len() > 1 {
                                Box::new(comps[1].clone())
                            } else {
                                zero_expr_float!()
                            },
                        ),
                        swizzle,
                        vec![],
                        self.create_loc(token.line),
                    ))
                } else {
                    Err(ParseError::new(
                        "Expected '(' after vec2",
                        token.line,
                        &self.path,
                    ))
                }
            }
            TokenType::Float3 => {
                self.advance();
                if self.match_token(vec![TokenType::LeftParen]) {
                    let comps = self.read_vec_components(3, token.line)?;
                    let swizzle: Vec<u8> = self.get_swizzle_at_current();

                    Ok(Expr::Value(
                        ASTValue::Float3(
                            if !comps.is_empty() {
                                Box::new(comps[0].clone())
                            } else {
                                zero_expr_float!()
                            },
                            if comps.len() > 1 {
                                Box::new(comps[1].clone())
                            } else {
                                if !comps.is_empty() {
                                    Box::new(comps[0].clone())
                                } else {
                                    zero_expr_float!()
                                }
                            },
                            if comps.len() > 2 {
                                Box::new(comps[2].clone())
                            } else {
                                if !comps.is_empty() {
                                    Box::new(comps[0].clone())
                                } else {
                                    zero_expr_float!()
                                }
                            },
                        ),
                        swizzle,
                        vec![],
                        self.create_loc(token.line),
                    ))
                } else {
                    Err(ParseError::new(
                        "Expected '(' after vec3",
                        token.line,
                        &self.path,
                    ))
                }
            }
            TokenType::LeftParen => {
                self.advance();
                let expr = self.expression()?;
                if self.match_token(vec![TokenType::RightParen]) {
                    Ok(Expr::Grouping(Box::new(expr), self.create_loc(token.line)))
                } else {
                    Err(ParseError::new(
                        "Expected ')' after expression",
                        token.line,
                        &self.path,
                    ))
                }
            }
            TokenType::Identifier => {
                self.advance();

                let mut swizzle = vec![];
                let field_path = vec![];
                if self.check(TokenType::Dot) {
                    if self.is_swizzle_valid_at_current() {
                        swizzle = self.get_swizzle_at_current();
                    }
                }
                if token.lexeme == "uv"
                    || token.lexeme == "color"
                    || token.lexeme == "roughness"
                    || token.lexeme == "metallic"
                    || token.lexeme == "emissive"
                    || token.lexeme == "opacity"
                    || token.lexeme == "bump"
                    || token.lexeme == "normal"
                    || token.lexeme == "hitpoint"
                    || token.lexeme == "time"
                {
                    Ok(Expr::Variable(
                        token.lexeme.clone(),
                        swizzle,
                        field_path,
                        self.create_loc(token.line),
                    ))
                } else
                // Local variables in functions
                if self.locals_map.contains_key(&token.lexeme) {
                    Ok(Expr::Variable(
                        token.lexeme,
                        swizzle,
                        field_path,
                        self.create_loc(token.line),
                    ))
                }
                // Verifier contains global variables and global functions
                else if let Some(_) = self.verifier.get_var_name(&token.lexeme) {
                    Ok(Expr::Variable(
                        token.lexeme,
                        swizzle,
                        field_path,
                        self.create_loc(token.line),
                    ))
                } else if self.function_names.contains(&token.lexeme) {
                    Ok(Expr::Variable(
                        token.lexeme,
                        swizzle,
                        field_path,
                        self.create_loc(token.line),
                    ))
                } else {
                    // Check against inbuilt functions
                    Err(ParseError::new(
                        format!("Unknown identifier '{}'", token.lexeme),
                        token.line,
                        &self.path,
                    ))
                }
            }
            _ => Err(ParseError::new(
                format!("Unknown identifier '{}'", token.lexeme),
                token.line,
                &self.path,
            )),
        }
    }

    /// Reads the components of a vector up to `max_comps` components. Can terminate early if closing parenthesis is found.
    /// Check for component validity is done in the compiler.
    fn read_vec_components(
        &mut self,
        max_comps: usize,
        line: usize,
    ) -> Result<Vec<Expr>, ParseError> {
        let mut components = vec![];
        let mut count = 0;

        if self.match_token(vec![TokenType::RightParen]) {
            return Ok(components);
        }

        while count < max_comps {
            let expr = self.expression()?;

            components.push(expr);
            count += 1;

            if !self.match_token(vec![TokenType::Comma]) {
                if !self.match_token(vec![TokenType::RightParen]) {
                    return Err(ParseError::new(
                        "Expected ')' after vector components",
                        line,
                        &self.path,
                    ));
                }
                break;
            }
        }

        Ok(components)
    }

    /// Returns the swizzle at the current token if any.
    pub fn get_swizzle_at_current(&mut self) -> Vec<u8> {
        let mut swizzle: Vec<u8> = vec![];

        if self.current + 2 < self.tokens.len()
            && self.tokens[self.current].kind == TokenType::Dot
            && self.tokens[self.current + 1].kind == TokenType::Identifier
            && self.tokens[self.current + 2].kind != TokenType::Dot
        {
            let swizzle_token = self.tokens[self.current + 1].lexeme.clone();
            if swizzle_token
                .chars()
                .all(|c| matches!(c, 'x' | 'y' | 'z' | 'w'))
            {
                swizzle = swizzle_token
                    .chars()
                    .map(|c| match c {
                        'x' => 0,
                        'y' => 1,
                        'z' => 2,
                        'w' => 3,
                        _ => unreachable!(),
                    })
                    .collect();
                self.current += 2;
            }
        }

        swizzle
    }

    /// Returns true if a swizzle is valid at the current token.
    pub fn is_swizzle_valid_at_current(&self) -> bool {
        if self.current + 1 < self.tokens.len()
            && self.tokens[self.current].kind == TokenType::Dot
            && self.tokens[self.current + 1].kind == TokenType::Identifier
        {
            let swizzle_token = &self.tokens[self.current + 1].lexeme;
            swizzle_token
                .chars()
                .all(|c| matches!(c, 'x' | 'y' | 'z' | 'w'))
        } else {
            false
        }
    }

    /// Extract a potential swizzle from the variable name.
    fn _extract_swizzle(input: &str) -> (&str, Vec<u8>) {
        if let Some(pos) = input.rfind('.') {
            let (base, swizzle) = input.split_at(pos);
            let swizzle = &swizzle[1..]; // Skip the dot

            // Check if all characters in the swizzle are 'x', 'y', 'z', or 'w'
            if swizzle.chars().all(|c| matches!(c, 'x' | 'y' | 'z' | 'w')) {
                // Map 'x', 'y', 'z', 'w' to 0, 1, 2, 3 respectively
                let swizzle_bytes = swizzle
                    .chars()
                    .map(|c| match c {
                        'x' => 0,
                        'y' => 1,
                        'z' => 2,
                        'w' => 3,
                        _ => unreachable!(),
                    })
                    .collect::<Vec<u8>>();

                return (base, swizzle_bytes);
            }
        }
        (input, Vec::new())
    }

    /// For debugging only
    fn _print_current(&self) {
        println!("Current: {:?}", self.tokens[self.current]);
    }

    // Consumes the next token if it matches the expected kind, otherwise returns a parse error.
    fn consume(
        &mut self,
        kind: TokenType,
        message: &str,
        line: usize,
    ) -> Result<Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance().unwrap())
        } else {
            Err(ParseError::new(message, line, &self.path))
        }
    }

    // Advances if the next token matches any in the expected list, returns true if matched.
    fn match_token(&mut self, expected: Vec<TokenType>) -> bool {
        if expected.iter().any(|&kind| self.check(kind)) {
            self.advance();
            true
        } else {
            false
        }
    }

    // Advances and returns the matched token type if any in the expected list matches.
    fn _match_token_and_return(&mut self, expected: Vec<TokenType>) -> Option<TokenType> {
        for &kind in &expected {
            if self.check(kind) {
                self.advance();
                return Some(kind);
            }
        }
        None
    }

    // Returns the lexeme of the current token.
    fn _lexeme(&self) -> String {
        if self.current < self.tokens.len() {
            self.tokens[self.current].lexeme.clone()
        } else {
            "".to_string()
        }
    }

    // Checks if the current token matches the given kind.
    fn check(&self, kind: TokenType) -> bool {
        self.current < self.tokens.len() && self.tokens[self.current].kind == kind
    }

    // Checks if the next token matches the given kind.
    fn check_next(&self, kind: TokenType) -> bool {
        self.current + 1 < self.tokens.len() && self.tokens[self.current + 1].kind == kind
    }

    // Advances to the next token and returns the previous token.
    fn advance(&mut self) -> Option<Token> {
        if !self.is_at_end() {
            self.current_line = self.tokens[self.current].line;
            self.current += 1;
        }
        self.previous()
    }

    // Returns true if all tokens have been consumed.
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    // Returns the current token or an EOF token if at the end.
    fn peek(&self) -> Token {
        if self.is_at_end() {
            Token {
                kind: TokenType::Eof,
                lexeme: "".to_string(),
                line: 0,
            }
        } else {
            self.tokens[self.current].clone()
        }
    }

    // Returns the previous token if available.
    fn previous(&self) -> Option<Token> {
        if self.current > 0 {
            Some(self.tokens[self.current - 1].clone())
        } else {
            None
        }
    }

    fn operator_to_unary(operator: TokenType) -> UnaryOperator {
        match operator {
            TokenType::Bang => UnaryOperator::Negate,
            TokenType::Minus => UnaryOperator::Minus,
            _ => unreachable!(),
        }
    }

    fn operator_to_binary(operator: TokenType) -> BinaryOperator {
        match operator {
            TokenType::Plus => BinaryOperator::Add,
            TokenType::Minus => BinaryOperator::Subtract,
            TokenType::Star => BinaryOperator::Multiply,
            TokenType::Slash => BinaryOperator::Divide,
            TokenType::Percent => BinaryOperator::Mod,
            _ => unreachable!(),
        }
    }

    fn operator_to_comparison(operator: TokenType) -> ComparisonOperator {
        match operator {
            TokenType::Greater => ComparisonOperator::Greater,
            TokenType::GreaterEqual => ComparisonOperator::GreaterEqual,
            TokenType::Less => ComparisonOperator::Less,
            TokenType::LessEqual => ComparisonOperator::LessEqual,
            _ => unreachable!(),
        }
    }

    fn operator_to_equality(operator: TokenType) -> EqualityOperator {
        match operator {
            TokenType::BangEqual => EqualityOperator::NotEqual,
            TokenType::EqualEqual => EqualityOperator::Equal,
            _ => unreachable!(),
        }
    }

    fn operator_to_logical(operator: TokenType) -> LogicalOperator {
        match operator {
            TokenType::And => LogicalOperator::And,
            TokenType::Or => LogicalOperator::Or,
            _ => unreachable!(),
        }
    }

    /// Create a location for the given line number.
    fn create_loc(&self, line: usize) -> Location {
        Location {
            line,
            path: self.path.clone(),
        }
    }
}
