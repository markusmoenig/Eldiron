use rand::Rng;

use crate::prelude::*;

use super::thecodenode::TheCodeNodeData;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TheCodeAtom {
    Assignment(TheValueAssignment),
    Comparison(TheValueComparison),
    Value(TheValue),
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    LocalGet(String),
    LocalSet(String, TheValueAssignment),
    ObjectGet(String, String),
    ObjectSet(String, String, TheValueAssignment),
    Get(String),
    Set(String, TheValueAssignment),
    RandInt(Vec2<i32>),
    RandFloat(Vec2<f32>),
    /// A call into a native function defined by the host.
    ExternalCall(String, String, Vec<String>, Vec<TheValue>, Option<TheValue>),
    /// A call into a module, identified by the bundle id and the codegrid id the module is based on.
    /// We keep the bundle and module names so we can display them in the editor.
    ModuleCall(String, Uuid, String, Uuid),
    Argument(String),
    Return,
    Or,
    And,
    EndOfExpression,
    EndOfCode,
}

impl TheCodeAtom {
    pub fn uneven_slot(&self) -> bool {
        matches!(self, TheCodeAtom::Assignment(_))
            || matches!(self, TheCodeAtom::Comparison(_))
            || matches!(self, TheCodeAtom::Add)
            || matches!(self, TheCodeAtom::Subtract)
            || matches!(self, TheCodeAtom::Multiply)
            || matches!(self, TheCodeAtom::Divide)
            || matches!(self, TheCodeAtom::Modulus)
            || matches!(self, TheCodeAtom::Or)
            || matches!(self, TheCodeAtom::And)
    }

    pub fn can_assign(&self) -> bool {
        matches!(self, TheCodeAtom::LocalSet(_, _))
            || matches!(self, TheCodeAtom::ObjectSet(_, _, _))
    }

    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or(TheCodeAtom::EndOfCode)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }

    pub fn to_node(&self, ctx: &mut TheCompilerContext) -> Option<TheCodeNode> {
        match self {
            TheCodeAtom::Comparison(op) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        if let Some(left) = stack.pop() {
                            if let Some(f) = data.sub_functions.first_mut() {
                                if let Some(right) = f.execute(sandbox).pop() {
                                    //println!("Comparison: left {:?}, right: {:?}", left, right);

                                    if let TheValue::Comparison(op) = data.values[1] {
                                        match op {
                                            TheValueComparison::Equal => {
                                                if left.is_equal(&right)
                                                    && data.sub_functions.len() > 1
                                                {
                                                    if sandbox.debug_mode {
                                                        sandbox.set_debug_executed(data.location);
                                                    }
                                                    _ = data.sub_functions[1]
                                                        .execute(sandbox)
                                                        .pop();
                                                }
                                            }
                                            TheValueComparison::Unequal => {
                                                if !left.is_equal(&right)
                                                    && data.sub_functions.len() > 1
                                                {
                                                    if sandbox.debug_mode {
                                                        sandbox.set_debug_executed(data.location);
                                                    }
                                                    _ = data.sub_functions[1]
                                                        .execute(sandbox)
                                                        .pop();
                                                }
                                            }
                                            TheValueComparison::GreaterThanOrEqual => {
                                                if left.is_greater_than_or_equal(&right)
                                                    && data.sub_functions.len() > 1
                                                {
                                                    if sandbox.debug_mode {
                                                        sandbox.set_debug_executed(data.location);
                                                    }
                                                    _ = data.sub_functions[1]
                                                        .execute(sandbox)
                                                        .pop();
                                                }
                                            }
                                            TheValueComparison::LessThanOrEqual => {
                                                if left.is_less_than_or_equal(&right)
                                                    && data.sub_functions.len() > 1
                                                {
                                                    if sandbox.debug_mode {
                                                        sandbox.set_debug_executed(data.location);
                                                    }
                                                    _ = data.sub_functions[1]
                                                        .execute(sandbox)
                                                        .pop();
                                                }
                                            }
                                            TheValueComparison::GreaterThan => {
                                                if left.is_greater_than(&right)
                                                    && data.sub_functions.len() > 1
                                                {
                                                    if sandbox.debug_mode {
                                                        sandbox.set_debug_executed(data.location);
                                                    }
                                                    _ = data.sub_functions[1]
                                                        .execute(sandbox)
                                                        .pop();
                                                }
                                            }
                                            TheValueComparison::LessThan => {
                                                if left.is_less_than(&right)
                                                    && data.sub_functions.len() > 1
                                                {
                                                    if sandbox.debug_mode {
                                                        sandbox.set_debug_executed(data.location);
                                                    }
                                                    _ = data.sub_functions[1]
                                                        .execute(sandbox)
                                                        .pop();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        TheCodeNodeCallResult::Continue
                    };

                let mut node = TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![TheValue::Int(0), TheValue::Comparison(*op)],
                    ),
                );

                if let Some(function) = ctx.remove_function() {
                    // let mut sandbox = TheCodeSandbox::new();
                    // if let Some(v) = function.execute(&mut sandbox).pop() {
                    //     println!("{:?}", v);//
                    //     node.data.values[1] = v;
                    // }
                    node.data.sub_functions.push(function);
                }

                Some(node)
            }
            TheCodeAtom::Return => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        if sandbox.debug_mode {
                            if let Some(v) = stack.last() {
                                sandbox.set_debug_value(data.location, (None, v.clone()));
                            }
                        }

                        TheCodeNodeCallResult::Break
                    };

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(ctx.node_location, vec![]),
                ))
            }
            TheCodeAtom::ExternalCall(_name, _, _, _, _rc) => {
                if let Some(call) = &ctx.external_call {
                    Some(TheCodeNode::new(
                        call.0,
                        TheCodeNodeData::location_values(ctx.node_location, call.1.clone()),
                    ))
                } else {
                    println!("Did not find external function: {}.", _name);
                    None
                }
            }
            TheCodeAtom::ModuleCall(_, package_id, _, codegrid_id) => {
                let call: TheCodeNodeCall =
                    |_: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        if let TheValue::Id(package_id) = data.values[0] {
                            if let TheValue::Id(codegrid_id) = data.values[1] {
                                if let Some(mut module) =
                                    sandbox.get_package_module_cloned(&package_id, &codegrid_id)
                                {
                                    let rc = module.execute(sandbox);
                                    if sandbox.debug_mode {
                                        if let Some(v) = rc.last() {
                                            sandbox
                                                .set_debug_value(data.location, (None, v.clone()));
                                        } else {
                                            sandbox.set_debug_value(
                                                data.location,
                                                (None, TheValue::Empty),
                                            );
                                        }
                                        sandbox.set_debug_executed(data.location);
                                    }
                                }
                            }
                        }

                        TheCodeNodeCallResult::Continue
                    };

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![TheValue::Id(*package_id), TheValue::Id(*codegrid_id)],
                    ),
                ))
            }
            /*
            TheCodeAtom::FuncCall(name) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        if let Some(id) = sandbox.module_stack.last() {
                            if let Some(mut function) = sandbox
                                .get_function_cloned(*id, &data.values[0].to_string().unwrap())
                            {
                                let mut clone = function.clone();

                                // Insert the arguments (if any) into the clone locals

                                let arguments = clone.arguments.clone();
                                for arg in &arguments {
                                    //}.iter().enumerate() {
                                    if let Some(arg_value) = stack.pop() {
                                        clone.set_local(arg.clone(), arg_value);
                                    }
                                }

                                sandbox.call_stack.push(clone);
                                function.execute(sandbox);
                                if let Some(rc_value) = &sandbox.func_rc {
                                    stack.push(rc_value.clone());
                                }
                                sandbox.call_stack.pop();
                            }
                        }
                        TheCodeNodeCallResult::Continue
                    };
                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![TheValue::Text(name.clone())],
                    ),
                ))
            }*/
            TheCodeAtom::LocalGet(name) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        if let Some(function) = sandbox.call_stack.last_mut() {
                            if let Some(local) =
                                function.get_local(&data.values[0].to_string().unwrap())
                            {
                                stack.push(local.clone());
                            } else {
                                println!(
                                    "Runtime error: Unknown local variable {} at {:?}.",
                                    &data.values[0].to_string().unwrap(),
                                    data.location
                                );
                            }
                        }
                        TheCodeNodeCallResult::Continue
                    };

                /*
                if ctx.error.is_none() {
                    let mut error = true;
                    if let Some(local) = ctx.local.last_mut() {
                        if let Some(local) = local.get(&name.clone()) {
                            ctx.stack.push(local.clone());
                            error = false;
                        }
                    }
                    if error {
                        ctx.error = Some(TheCompilerError::new(
                            ctx.node_location,
                            format!("Unknown local variable {}.", name),
                        ));
                    }
                }*/
                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![TheValue::Text(name.clone())],
                    ),
                ))
            }
            TheCodeAtom::LocalSet(name, op) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        let mut debug_value: Option<TheValue> = None;

                        if let Some(function) = sandbox.call_stack.last_mut() {
                            if let Some(local) = function.local.last_mut() {
                                if let Some(v) = stack.pop() {
                                    if let TheValue::Assignment(op) = data.values[1] {
                                        match op {
                                            TheValueAssignment::Assign => {
                                                if sandbox.debug_mode {
                                                    debug_value = Some(v.clone());
                                                }
                                                local.set(data.values[0].to_string().unwrap(), v);
                                            }
                                            TheValueAssignment::AddAssign => {
                                                let name = data.values[0].to_string().unwrap();
                                                if let Some(left) = local.get(&name) {
                                                    // Handle special String case
                                                    if let TheValue::Text(a) = left {
                                                        let result = TheValue::Text(format!(
                                                            "{} {}",
                                                            a,
                                                            v.describe()
                                                        ));
                                                        if sandbox.debug_mode {
                                                            debug_value = Some(result.clone());
                                                        }
                                                        local.set(name, result);
                                                    } else if let Some(result) =
                                                        TheValue::add(left, &v)
                                                    {
                                                        if sandbox.debug_mode {
                                                            debug_value = Some(result.clone());
                                                        }
                                                        local.set(name, result);
                                                    }
                                                }
                                            }
                                            TheValueAssignment::SubtractAssign => {
                                                let name = data.values[0].to_string().unwrap();
                                                if let Some(left) = local.get(&name) {
                                                    if let Some(result) = TheValue::sub(left, &v) {
                                                        if sandbox.debug_mode {
                                                            debug_value = Some(result.clone());
                                                        }
                                                        local.set(name, result);
                                                    }
                                                }
                                            }
                                            TheValueAssignment::MultiplyAssign => {
                                                let name = data.values[0].to_string().unwrap();
                                                if let Some(left) = local.get(&name) {
                                                    if let Some(result) = TheValue::mul(left, &v) {
                                                        if sandbox.debug_mode {
                                                            debug_value = Some(result.clone());
                                                        }
                                                        local.set(name, result);
                                                    }
                                                }
                                            }
                                            TheValueAssignment::DivideAssign => {
                                                let name = data.values[0].to_string().unwrap();
                                                if let Some(left) = local.get(&name) {
                                                    if let Some(result) = TheValue::div(left, &v) {
                                                        if sandbox.debug_mode {
                                                            debug_value = Some(result.clone());
                                                        }
                                                        local.set(name, result);
                                                    }
                                                }
                                            }
                                            TheValueAssignment::ModulusAssign => {
                                                let name = data.values[0].to_string().unwrap();
                                                if let Some(left) = local.get(&name) {
                                                    if let Some(result) =
                                                        TheValue::modulus(left, &v)
                                                    {
                                                        if sandbox.debug_mode {
                                                            debug_value = Some(result.clone());
                                                        }
                                                        local.set(name, result);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(debug_value) = debug_value {
                            sandbox.set_debug_value(data.location, (None, debug_value));
                        }

                        TheCodeNodeCallResult::Continue
                    };

                /*
                if ctx.error.is_none() {
                    if ctx.stack.is_empty() {
                        ctx.error = Some(TheCompilerError::new(
                            ctx.node_location,
                            "Nothing to assign to local variable.".to_string(),
                        ));
                    } else if let Some(local) = ctx.local.last_mut() {
                        local.set(name.clone(), ctx.stack.pop().unwrap());
                    }
                }*/

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![TheValue::Text(name.clone()), TheValue::Assignment(*op)],
                    ),
                ))
            }
            TheCodeAtom::ObjectGet(object, name) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        if let Some(object) =
                            sandbox.get_object(&data.values[0].to_string().unwrap())
                        {
                            if let Some(v) = object.get(&data.values[1].to_string().unwrap()) {
                                stack.push(v.clone());
                            } else {
                                println!(
                                    "Runtime error: Unknown object variable {}.",
                                    &data.values[1].to_string().unwrap()
                                );
                            }
                        } else {
                            println!(
                                "Runtime error: Unknown object {}.",
                                &data.values[0].to_string().unwrap()
                            );
                        }
                        TheCodeNodeCallResult::Continue
                    };

                ctx.stack.push(TheValue::Int(0));
                // if ctx.error.is_none() {
                //     let mut error = true;
                //     if let Some(local) = ctx.local.last_mut() {
                //         if let Some(local) = local.get(&name.clone()) {
                //             ctx.stack.push(local.clone());
                //             error = false;
                //         }
                //     }
                //     if error {
                //         ctx.error = Some(TheCompilerError::new(
                //             ctx.current_location,
                //             format!("Unknown local variable {}.", name),
                //         ));
                //     }
                // }
                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![TheValue::Text(object.clone()), TheValue::Text(name.clone())],
                    ),
                ))
            }
            TheCodeAtom::ObjectSet(object, name, op) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        let mut debug_value: Option<TheValue> = None;

                        let debug_mode = sandbox.debug_mode;
                        if let Some(object) =
                            sandbox.get_object_mut(&data.values[0].to_string().unwrap())
                        {
                            if let Some(v) = stack.pop() {
                                if debug_mode {
                                    debug_value = Some(v.clone());
                                }
                                if let TheValue::Assignment(op) = data.values[2] {
                                    match op {
                                        TheValueAssignment::Assign => {
                                            if debug_mode {
                                                debug_value = Some(v.clone());
                                            }
                                            object.set(data.values[1].to_string().unwrap(), v);
                                        }
                                        TheValueAssignment::AddAssign => {
                                            let name = data.values[0].to_string().unwrap();
                                            if let Some(left) = object.get(&name) {
                                                // Handle special String case
                                                if let TheValue::Text(a) = left {
                                                    let result = TheValue::Text(format!(
                                                        "{} {}",
                                                        a,
                                                        v.describe()
                                                    ));
                                                    if debug_mode {
                                                        debug_value = Some(result.clone());
                                                    }
                                                    object.set(name, result);
                                                } else if let Some(result) = TheValue::add(left, &v)
                                                {
                                                    if debug_mode {
                                                        debug_value = Some(result.clone());
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                        TheValueAssignment::SubtractAssign => {
                                            let name = data.values[0].to_string().unwrap();
                                            if let Some(left) = object.get(&name) {
                                                if let Some(result) = TheValue::sub(left, &v) {
                                                    if debug_mode {
                                                        debug_value = Some(result.clone());
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                        TheValueAssignment::MultiplyAssign => {
                                            let name = data.values[0].to_string().unwrap();
                                            if let Some(left) = object.get(&name) {
                                                if let Some(result) = TheValue::mul(left, &v) {
                                                    if debug_mode {
                                                        debug_value = Some(result.clone());
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                        TheValueAssignment::DivideAssign => {
                                            let name = data.values[0].to_string().unwrap();
                                            if let Some(left) = object.get(&name) {
                                                if let Some(result) = TheValue::div(left, &v) {
                                                    if debug_mode {
                                                        debug_value = Some(result.clone());
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                        TheValueAssignment::ModulusAssign => {
                                            let name = data.values[0].to_string().unwrap();
                                            if let Some(left) = object.get(&name) {
                                                if let Some(result) = TheValue::modulus(left, &v) {
                                                    if debug_mode {
                                                        debug_value = Some(result.clone());
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                println!("Runtime error: Object Set. Stack is empty.",);
                            }
                        } else {
                            println!(
                                "Runtime error: Object Set. Unknown object {}.",
                                data.values[0].to_string().unwrap()
                            );
                        }

                        if let Some(debug_value) = debug_value {
                            sandbox.set_debug_value(data.location, (None, debug_value));
                        }
                        TheCodeNodeCallResult::Continue
                    };

                if ctx.error.is_none() {
                    if ctx.stack.is_empty() {
                        ctx.error = Some(TheCompilerError::new(
                            ctx.node_location,
                            "Nothing to assign to local variable.".to_string(),
                        ));
                    } else if let Some(local) = ctx.local.last_mut() {
                        local.set(name.clone(), ctx.stack.pop().unwrap());
                    }
                }
                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![
                            TheValue::Text(object.clone()),
                            TheValue::Text(name.clone()),
                            TheValue::Assignment(*op),
                        ],
                    ),
                ))
            }
            TheCodeAtom::Get(path) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        // Get a value of an object by recursively traversing the object tree.
                        fn get_value(
                            object: &TheCodeObject,
                            mut parts: Vec<String>,
                        ) -> Option<TheValue> {
                            match parts.len().cmp(&1) {
                                std::cmp::Ordering::Greater => {
                                    let current_part = parts.remove(0).to_string();
                                    if let Some(TheValue::CodeObject(o)) = object.get(&current_part)
                                    {
                                        get_value(o, parts)
                                    } else {
                                        None
                                    }
                                }
                                std::cmp::Ordering::Equal => object.get(&parts[0]).cloned(),
                                _ => None,
                            }
                        }

                        if data.values[0] == TheValue::Bool(true) {
                            // Object
                            if let TheValue::TextList(_, parts) = &data.values[1] {
                                if let Some(object) = sandbox.get_object_mut(&parts[0]) {
                                    let rest_parts: Vec<String> =
                                        parts.iter().skip(1).cloned().collect();

                                    if let Some(value) = get_value(object, rest_parts) {
                                        stack.push(value);
                                    }
                                }
                            }
                        } else if let Some(function) = sandbox.call_stack.last_mut() {
                            if let Some(object) = function.local.last_mut() {
                                if let TheValue::TextList(_, parts) = &data.values[1] {
                                    if let Some(value) = get_value(object, parts.clone()) {
                                        stack.push(value);
                                    }
                                }
                            }
                        }
                        TheCodeNodeCallResult::Continue
                    };

                let mut is_object = false;
                let mut parts: Vec<String> = path.split('.').map(|s| s.to_string()).collect();

                if let Some(first) = parts.get_mut(0) {
                    // if first.starts_with(':') {
                    //     *first = first.strip_prefix(':').unwrap_or(first).to_string();
                    //     is_object = true;
                    // }
                    if first.starts_with('@') {
                        *first = first.strip_prefix('@').unwrap_or(first).to_string();
                        is_object = true;
                    }
                }

                if ctx.error.is_none() && parts.is_empty() {
                    ctx.error = Some(TheCompilerError::new(
                        ctx.node_location,
                        "Empty variable path for Set().".to_string(),
                    ));
                }

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![TheValue::Bool(is_object), TheValue::TextList(0, parts)],
                    ),
                ))
            }
            TheCodeAtom::Set(path, op) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        // Assign a value to an object by recursively traversing the object tree.
                        fn assign_value(
                            object: &mut TheCodeObject,
                            mut parts: Vec<String>,
                            value: TheValue,
                            op: TheValueAssignment,
                            debug_value: &mut Option<TheValue>,
                        ) {
                            match parts.len().cmp(&1) {
                                std::cmp::Ordering::Greater => {
                                    let current_part = parts.remove(0).to_string();
                                    if let Some(TheValue::CodeObject(o)) =
                                        object.get_mut(&current_part)
                                    {
                                        assign_value(o, parts, value, op, debug_value);
                                    }
                                }
                                std::cmp::Ordering::Equal => {
                                    let name = parts[0].clone();
                                    match op {
                                        TheValueAssignment::Assign => {
                                            if let Some(dv) = debug_value {
                                                *dv = value.clone();
                                            }
                                            object.set(parts[0].clone(), value);
                                        }
                                        TheValueAssignment::AddAssign => {
                                            if let Some(left) = object.get_mut(&name) {
                                                if let TheValue::Text(a) = left {
                                                    // Handle special String case
                                                    let result = TheValue::Text(format!(
                                                        "{} {}",
                                                        a,
                                                        value.describe()
                                                    ));
                                                    if let Some(dv) = debug_value {
                                                        *dv = result.clone();
                                                    }
                                                    object.set(name, result);
                                                } else if let TheValue::List(list) = left {
                                                    // += on a list, add the value.
                                                    list.push(value);
                                                } else if let Some(result) =
                                                    TheValue::add(left, &value)
                                                {
                                                    if let Some(dv) = debug_value {
                                                        *dv = result.clone();
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                        TheValueAssignment::SubtractAssign => {
                                            if let Some(left) = object.get(&name) {
                                                if let Some(result) = TheValue::sub(left, &value) {
                                                    if let Some(dv) = debug_value {
                                                        *dv = result.clone();
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                        TheValueAssignment::MultiplyAssign => {
                                            if let Some(left) = object.get(&name) {
                                                if let Some(result) = TheValue::mul(left, &value) {
                                                    if let Some(dv) = debug_value {
                                                        *dv = result.clone();
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                        TheValueAssignment::DivideAssign => {
                                            if let Some(left) = object.get(&name) {
                                                if let Some(result) = TheValue::div(left, &value) {
                                                    if let Some(dv) = debug_value {
                                                        *dv = result.clone();
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                        TheValueAssignment::ModulusAssign => {
                                            if let Some(left) = object.get(&name) {
                                                if let Some(result) =
                                                    TheValue::modulus(left, &value)
                                                {
                                                    if let Some(dv) = debug_value {
                                                        *dv = result.clone();
                                                    }
                                                    object.set(name, result);
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }

                        if let Some(v) = stack.pop() {
                            let mut debug_value: Option<TheValue> = None;
                            if sandbox.debug_mode {
                                debug_value = Some(TheValue::Empty);
                            }

                            if let TheValue::Assignment(op) = data.values[2] {
                                if data.values[0] == TheValue::Bool(true) {
                                    // Object
                                    if let TheValue::TextList(_, parts) = &data.values[1] {
                                        if let Some(object) = sandbox.get_object_mut(&parts[0]) {
                                            let rest_parts: Vec<String> =
                                                parts.iter().skip(1).cloned().collect();

                                            assign_value(
                                                object,
                                                rest_parts,
                                                v,
                                                op,
                                                &mut debug_value,
                                            );
                                        }
                                    }
                                } else if let Some(function) = sandbox.call_stack.last_mut() {
                                    if let Some(object) = function.local.last_mut() {
                                        if let TheValue::TextList(_, parts) = &data.values[1] {
                                            assign_value(
                                                object,
                                                parts.clone(),
                                                v,
                                                op,
                                                &mut debug_value,
                                            );
                                        }
                                    }
                                }
                            }

                            if let Some(debug_value) = debug_value {
                                sandbox.set_debug_value(data.location, (None, debug_value));
                            }
                        }
                        TheCodeNodeCallResult::Continue
                    };

                let mut is_object = false;
                let mut parts: Vec<String> = path.split('.').map(|s| s.to_string()).collect();

                if let Some(first) = parts.get_mut(0) {
                    // if first.starts_with(':') {
                    //     *first = first.strip_prefix(':').unwrap_or(first).to_string();
                    //     is_object = true;
                    // }
                    if first.starts_with('@') {
                        *first = first.strip_prefix('@').unwrap_or(first).to_string();
                        is_object = true;
                    }
                }

                if ctx.error.is_none() && parts.is_empty() {
                    ctx.error = Some(TheCompilerError::new(
                        ctx.node_location,
                        "Empty variable path for Set().".to_string(),
                    ));
                }

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![
                            TheValue::Bool(is_object),
                            TheValue::TextList(0, parts),
                            TheValue::Assignment(*op),
                        ],
                    ),
                ))
            }
            TheCodeAtom::Value(value) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     _sandbox: &mut TheCodeSandbox| {
                        stack.push(data.values[0].clone());
                        TheCodeNodeCallResult::Continue
                    };

                ctx.stack.push(value.clone());

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(ctx.node_location, vec![value.clone()]),
                ))
            }
            TheCodeAtom::RandInt(value) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        let mut rng = rand::thread_rng();
                        if let Some(range) = data.values[0].to_vec2i() {
                            let v = rng.gen_range(range.x..=range.y);
                            if sandbox.debug_mode {
                                sandbox.set_debug_value(data.location, (None, TheValue::Int(v)));
                            }
                            stack.push(TheValue::Int(v));
                        }
                        TheCodeNodeCallResult::Continue
                    };

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![TheValue::Int2(*value)],
                    ),
                ))
            }
            TheCodeAtom::RandFloat(value) => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     data: &mut TheCodeNodeData,
                     sandbox: &mut TheCodeSandbox| {
                        let mut rng = rand::thread_rng();
                        if let Some(range) = data.values[0].to_vec2f() {
                            let v = rng.gen_range(range.x..=range.y);
                            if sandbox.debug_mode {
                                sandbox.set_debug_value(data.location, (None, TheValue::Float(v)));
                            }
                            stack.push(TheValue::Float(v));
                        }
                        TheCodeNodeCallResult::Continue
                    };

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location_values(
                        ctx.node_location,
                        vec![TheValue::Float2(*value)],
                    ),
                ))
            }
            TheCodeAtom::Add => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     _data: &mut TheCodeNodeData,
                     _sandbox: &mut TheCodeSandbox| {
                        if let Some(b) = stack.pop() {
                            if let Some(a) = stack.pop() {
                                if let TheValue::Text(a) = a {
                                    let result = TheValue::Text(format!("{} {}", a, b.describe()));
                                    stack.push(result);
                                } else if let Some(result) = TheValue::add(&a, &b) {
                                    stack.push(result);
                                } else {
                                    println!("Runtime error: Add. Invalid types.");
                                }
                            }
                        }
                        TheCodeNodeCallResult::Continue
                    };

                if ctx.error.is_none() && ctx.stack.len() < 2 {
                    ctx.error = Some(TheCompilerError::new(
                        ctx.node_location,
                        format!("Invalid stack for Add ({})", ctx.stack.len()),
                    ));
                }

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location(ctx.node_location),
                ))
            }
            TheCodeAtom::Subtract => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     _data: &mut TheCodeNodeData,
                     _sandbox: &mut TheCodeSandbox| {
                        if let Some(b) = stack.pop() {
                            if let Some(a) = stack.pop() {
                                if let Some(result) = TheValue::sub(&a, &b) {
                                    stack.push(result);
                                } else {
                                    println!("Runtime error: Sub. Invalid types.");
                                }
                            }
                        }
                        TheCodeNodeCallResult::Continue
                    };

                if ctx.error.is_none() && ctx.stack.len() < 2 {
                    ctx.error = Some(TheCompilerError::new(
                        ctx.node_location,
                        format!("Invalid stack for Sub ({})", ctx.stack.len()),
                    ));
                }

                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location(ctx.node_location),
                ))
            }
            TheCodeAtom::Multiply => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     _data: &mut TheCodeNodeData,
                     _sandbox: &mut TheCodeSandbox| {
                        if let Some(b) = stack.pop() {
                            if let Some(a) = stack.pop() {
                                if let Some(result) = TheValue::mul(&a, &b) {
                                    stack.push(result);
                                } else {
                                    println!("Runtime error: Multiply. Invalid types.");
                                }
                            }
                        }
                        TheCodeNodeCallResult::Continue
                    };

                if ctx.error.is_none() && ctx.stack.len() < 2 {
                    ctx.error = Some(TheCompilerError::new(
                        ctx.node_location,
                        format!("Invalid stack for Multiply ({})", ctx.stack.len()),
                    ));
                }
                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location(ctx.current_location),
                ))
            }
            TheCodeAtom::Divide => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     _data: &mut TheCodeNodeData,
                     _sandbox: &mut TheCodeSandbox| {
                        if let Some(b) = stack.pop() {
                            if let Some(a) = stack.pop() {
                                if let Some(result) = TheValue::div(&a, &b) {
                                    stack.push(result);
                                } else {
                                    println!("Runtime error: Division. Invalid types.");
                                }
                            }
                        }
                        TheCodeNodeCallResult::Continue
                    };

                if ctx.error.is_none() && ctx.stack.len() < 2 {
                    ctx.error = Some(TheCompilerError::new(
                        ctx.node_location,
                        format!("Invalid stack for Division ({})", ctx.stack.len()),
                    ));
                }
                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location(ctx.current_location),
                ))
            }
            TheCodeAtom::Modulus => {
                let call: TheCodeNodeCall =
                    |stack: &mut Vec<TheValue>,
                     _data: &mut TheCodeNodeData,
                     _sandbox: &mut TheCodeSandbox| {
                        if let Some(b) = stack.pop() {
                            if let Some(a) = stack.pop() {
                                if let Some(result) = TheValue::modulus(&a, &b) {
                                    stack.push(result);
                                } else {
                                    println!("Runtime error: Modulus. Invalid types.");
                                }
                            }
                        }
                        TheCodeNodeCallResult::Continue
                    };

                if ctx.error.is_none() && ctx.stack.len() < 2 {
                    ctx.error = Some(TheCompilerError::new(
                        ctx.node_location,
                        format!("Invalid stack for Modulus ({})", ctx.stack.len()),
                    ));
                }
                Some(TheCodeNode::new(
                    call,
                    TheCodeNodeData::location(ctx.current_location),
                ))
            }
            _ => None,
        }
    }

    pub fn to_sdf(&self, dim: TheDim, zoom: f32) -> TheSDF {
        match self {
            Self::Value(_) | Self::RandInt(_) | Self::RandFloat(_) => TheSDF::Hexagon(dim),
            Self::Add | &Self::Multiply => {
                TheSDF::RoundedRect(dim, (0.0, 0.0, 0.0, 0.0))
                //TheSDF::Rhombus(dim)
            }
            Self::ObjectGet(_, _) | Self::LocalGet(_) | Self::Get(_) => {
                TheSDF::RoundedRect(dim, (10.0 * zoom, 10.0 * zoom, 0.0, 0.0))
            }
            Self::ObjectSet(_, _, _) | Self::LocalSet(_, _) | Self::Set(_, _) => {
                TheSDF::RoundedRect(dim, (0.0, 0.0, 10.0 * zoom, 10.0 * zoom))
            }
            Self::Return => TheSDF::RoundedRect(dim, (0.0, 0.0, 10.0 * zoom, 0.0)),
            Self::ExternalCall(_, _, _, _, _) | Self::ModuleCall(_, _, _, _) => {
                TheSDF::RoundedRect(dim, (10.0 * zoom, 10.0 * zoom, 10.0 * zoom, 10.0 * zoom))
            }
            _ => TheSDF::RoundedRect(dim, (0.0, 0.0, 0.0, 0.0)),
        }
    }

    pub fn to_color(&self) -> [u8; 4] {
        match self {
            Self::ObjectSet(_, _, _) => [36, 61, 92, 255],
            //[87, 112, 143, 255]
            _ => [174, 174, 174, 255],
        }
    }

    pub fn to_kind(&self) -> TheCodeAtomKind {
        match self {
            TheCodeAtom::Value(_) | TheCodeAtom::RandInt(_) | TheCodeAtom::RandFloat(_) => {
                TheCodeAtomKind::Number
            }
            TheCodeAtom::Add => TheCodeAtomKind::Plus,
            TheCodeAtom::Subtract => TheCodeAtomKind::Minus,
            TheCodeAtom::Multiply => TheCodeAtomKind::Star,
            TheCodeAtom::Divide => TheCodeAtomKind::Slash,
            TheCodeAtom::Modulus => TheCodeAtomKind::Percent,
            TheCodeAtom::Or => TheCodeAtomKind::Or,
            TheCodeAtom::And => TheCodeAtomKind::And,
            TheCodeAtom::Return => TheCodeAtomKind::Return,
            TheCodeAtom::EndOfExpression => TheCodeAtomKind::Semicolon,
            TheCodeAtom::EndOfCode => TheCodeAtomKind::Eof,
            _ => TheCodeAtomKind::Identifier,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            TheCodeAtom::Assignment(op) => op.to_string().to_string(),
            TheCodeAtom::Comparison(op) => op.to_string().to_string(),
            TheCodeAtom::Argument(name) => name.clone(),
            TheCodeAtom::ExternalCall(name, _, _, _, _) => name.clone(),
            TheCodeAtom::ModuleCall(_, _, module_name, _) => module_name.clone(),
            TheCodeAtom::LocalGet(name) => name.clone(),
            TheCodeAtom::LocalSet(name, _) => name.clone(),
            TheCodeAtom::ObjectGet(object, name) => format!("{}.{}", object, name),
            TheCodeAtom::ObjectSet(object, name, _) => format!("{}.{}", object, name),
            TheCodeAtom::Get(name) => name.clone(),
            TheCodeAtom::Set(name, _) => name.clone(),
            TheCodeAtom::Value(value) => match value {
                TheValue::Tile(name, _id) => name.clone(),
                _ => value.describe(),
            },
            TheCodeAtom::Add => "+".to_string(),
            TheCodeAtom::Subtract => "-".to_string(),
            TheCodeAtom::Multiply => "*".to_string(),
            TheCodeAtom::Divide => "/".to_string(),
            TheCodeAtom::Modulus => "%".to_string(),
            TheCodeAtom::Or => "Or".to_string(),
            TheCodeAtom::And => "And".to_string(),
            TheCodeAtom::Return => "Return".to_string(),
            TheCodeAtom::EndOfExpression => ";".to_string(),
            TheCodeAtom::EndOfCode => "Stop".to_string(),
            TheCodeAtom::RandInt(_) => "RInt".to_string(),
            TheCodeAtom::RandFloat(_) => "RFloat".to_string(),
        }
    }

    pub fn help(&self) -> String {
        match self {
            TheCodeAtom::Assignment(op) => format!("Assignment ({}).", op.to_string()),
            TheCodeAtom::Comparison(op) => format!("Comparison ({}).", op.to_string()),
            TheCodeAtom::Argument(name) => format!("Function argument ({}).", name),
            TheCodeAtom::ModuleCall(bundle, _, name, _) => format!("{}: {}.", bundle, name),
            TheCodeAtom::ExternalCall(_, description, _, _, _) => description.clone(),
            TheCodeAtom::LocalGet(name) => format!("Get the value of a local variable ({}).", name),
            TheCodeAtom::LocalSet(name, _) => {
                format!("Set a value to a local variable ({}).", name)
            }
            TheCodeAtom::ObjectGet(object, name) => {
                format!("Get the value of an object variable ({}.{}).", object, name)
            }
            TheCodeAtom::ObjectSet(object, name, _) => {
                format!("Set a value to an object variable ({}.{}).", object, name)
            }
            TheCodeAtom::Get(name) => format!("Get the value of a variable ({}).", name),
            TheCodeAtom::Set(name, _) => {
                format!("Set a value to a variable ({}).", name)
            }
            TheCodeAtom::Value(value) => match value {
                TheValue::Assignment(_) => self.describe(),
                TheValue::Comparison(_) => self.describe(),
                TheValue::Bool(_v) => format!("Boolean constant ({}).", self.describe()),
                TheValue::CodeObject(_v) => "An object.".to_string(),
                TheValue::List(_v) => "A list of values.".to_string(),
                TheValue::Int(v) | TheValue::IntRange(v, _) => format!("Integer constant ({}).", v),
                TheValue::Float(_) | TheValue::FloatRange(_, _) => {
                    format!("Float constant ({}).", value.describe())
                }
                TheValue::Text(v) => format!("Text constant ({}).", v),
                TheValue::TextList(index, v) => {
                    format!("Text List ({}).", v[*index as usize].clone())
                }
                TheValue::Char(v) => format!("Char constant ({}).", v),
                TheValue::Int2(v) => format!("Int2 constant ({}).", v),
                TheValue::Float2(v) => format!("Float2 constant ({}).", v),
                TheValue::Int3(v) => format!("Int3 constant ({}).", v),
                TheValue::Float3(v) => format!("Float3 constant ({}).", v),
                TheValue::Int4(v) => format!("Int4 constant ({}).", v),
                TheValue::Float4(v) => format!("Float4 constant ({}).", v),
                TheValue::Position(v) => format!("Position ({}).", v),
                TheValue::Tile(name, _id) => format!("Tile ({}).", name),
                TheValue::KeyCode(_v) => "Key Code value.".to_string(),
                TheValue::RangeI32(_v) => "Range value.".to_string(),
                TheValue::RangeF32(_v) => "Range value.".to_string(),
                TheValue::ColorObject(_) => "Color.".to_string(),
                TheValue::PaletteIndex(_) => "Palette index.".to_string(),
                TheValue::Empty => "Empty value.".to_string(),
                TheValue::Id(id) => format!("Id ({}).", id),
                TheValue::Direction(v) => format!("Direction ({}).", v),
                TheValue::Time(_) => self.describe(),
                TheValue::TimeDuration(_, _) => self.describe(),
                TheValue::TileMask(_) => self.describe(),
                TheValue::Image(_) => self.describe(),
            },
            TheCodeAtom::Add => "Operator ('+')".to_string(),
            TheCodeAtom::Subtract => "Operator ('-')".to_string(),
            TheCodeAtom::Multiply => "Operator ('*')".to_string(),
            TheCodeAtom::Divide => "Operator ('/')".to_string(),
            TheCodeAtom::Modulus => "Operator ('%')".to_string(),
            TheCodeAtom::Or => "Logical Or".to_string(),
            TheCodeAtom::And => "Logical And".to_string(),
            TheCodeAtom::Return => "Return".to_string(),
            TheCodeAtom::EndOfExpression => ";".to_string(),
            TheCodeAtom::EndOfCode => "Stop".to_string(),
            TheCodeAtom::RandInt(_) => {
                "Generates a random Integer value in a given range".to_string()
            }
            TheCodeAtom::RandFloat(_) => {
                "Generates a random Float value in a given range".to_string()
            }
        }
    }

    #[cfg(feature = "ui")]
    /// Generates a text layout to edit the properties of the atom
    pub fn to_layout(&self, layout: &mut dyn TheHLayoutTrait) {
        match self {
            TheCodeAtom::Assignment(op) => {
                let mut text = TheText::new(TheId::empty());
                text.set_text("Assignment".to_string());
                let mut drop_down = TheDropdownMenu::new(TheId::named("Atom Assignment"));
                for dir in TheValueAssignment::iterator() {
                    drop_down.add_option(dir.to_string().to_string());
                }
                drop_down.set_selected_index(*op as i32);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(drop_down));
            }
            TheCodeAtom::Comparison(op) => {
                let mut text = TheText::new(TheId::empty());
                text.set_text("Comparison".to_string());
                let mut drop_down = TheDropdownMenu::new(TheId::named("Atom Comparison"));
                for dir in TheValueComparison::iterator() {
                    drop_down.add_option(dir.to_string().to_string());
                }
                drop_down.set_selected_index(*op as i32);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(drop_down));
            }
            TheCodeAtom::Argument(name) => {
                let mut text = TheText::new(TheId::empty());
                text.set_text("Argument Name".to_string());
                let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Func Arg"));
                name_edit.set_text(name.clone());
                name_edit.set_needs_redraw(true);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(name_edit));
            }
            TheCodeAtom::Get(name) => {
                let mut text = TheText::new(TheId::empty());
                text.set_text("Path".to_string());
                let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Get"));
                name_edit.set_text(name.clone());
                name_edit.limiter_mut().set_max_width(300);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(name_edit));
            }
            TheCodeAtom::Set(name, _) => {
                let mut text = TheText::new(TheId::empty());
                text.set_text("Path".to_string());
                let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Set"));
                name_edit.set_text(name.clone());
                name_edit.limiter_mut().set_max_width(300);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(name_edit));
            }
            TheCodeAtom::LocalGet(name) => {
                let mut text = TheText::new(TheId::empty());
                text.set_text("Variable Name".to_string());
                let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Local Get"));
                name_edit.set_text(name.clone());
                name_edit.set_needs_redraw(true);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(name_edit));
            }
            TheCodeAtom::LocalSet(name, _) => {
                let mut text = TheText::new(TheId::empty());
                text.set_text("Variable Name".to_string());
                let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Local Set"));
                name_edit.set_text(name.clone());
                name_edit.set_needs_redraw(true);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(name_edit));
            }
            TheCodeAtom::ObjectGet(object, name) => {
                let mut text = TheText::new(TheId::empty());
                text.set_text("Object Name".to_string());
                let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Object Get Object"));
                name_edit.set_text(object.clone());
                name_edit.set_needs_redraw(true);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(name_edit));

                let mut text = TheText::new(TheId::empty());
                text.set_text("Variable Name".to_string());
                let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Object Get Variable"));
                name_edit.set_text(name.clone());
                name_edit.set_needs_redraw(true);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(name_edit));
            }
            TheCodeAtom::ObjectSet(object, name, _) => {
                let mut text = TheText::new(TheId::empty());
                text.set_text("Object Name".to_string());
                let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Object Set Object"));
                name_edit.set_text(object.clone());
                name_edit.set_needs_redraw(true);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(name_edit));

                let mut text = TheText::new(TheId::empty());
                text.set_text("Variable Name".to_string());
                let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Object Set Variable"));
                name_edit.set_text(name.clone());
                name_edit.set_needs_redraw(true);
                layout.add_widget(Box::new(text));
                layout.add_widget(Box::new(name_edit));
            }
            TheCodeAtom::RandInt(v) => {
                create_int2_widgets(
                    layout,
                    TheId::named("Atom RandInt"),
                    *v,
                    vec!["Low", "High"],
                );
            }
            TheCodeAtom::RandFloat(v) => {
                create_float2_widgets(
                    layout,
                    TheId::named("Atom RandFloat"),
                    *v,
                    vec!["Low", "High"],
                );
            }
            TheCodeAtom::Value(value) => match value {
                TheValue::ColorObject(color) => {
                    let mut text = TheText::new(TheId::empty());
                    text.set_text("Color #".to_string());

                    let mut name_edit = TheTextLineEdit::new(TheId::named("Atom Color Hex"));
                    name_edit.set_status_text("The color in hex.");
                    name_edit.set_text(color.to_hex());
                    name_edit.set_needs_redraw(true);

                    layout.add_widget(Box::new(text));
                    layout.add_widget(Box::new(name_edit));
                }
                TheValue::Direction(value) => {
                    create_float2_widgets(
                        layout,
                        TheId::named("Atom Direction Float2"),
                        Vec2::new(value.x, value.z),
                        vec!["X", "Y"],
                    );
                }
                TheValue::TextList(index, list) => {
                    let mut drop_down = TheDropdownMenu::new(TheId::named("Atom TextList"));
                    for l in list {
                        drop_down.add_option(l.clone());
                    }
                    drop_down.set_selected_index(*index);
                    layout.add_widget(Box::new(drop_down));
                }
                TheValue::Position(v) => {
                    create_float2_widgets(
                        layout,
                        TheId::named("Atom Position"),
                        Vec2::new(v.x, v.z),
                        vec!["X", "Y"],
                    );
                }
                TheValue::Int2(v) => {
                    create_int2_widgets(layout, TheId::named("Atom Int2"), *v, vec!["X", "Y"]);
                }
                TheValue::Float2(v) => {
                    create_float2_widgets(layout, TheId::named("Atom Float2"), *v, vec!["X", "Y"]);
                }
                TheValue::Int(v) => {
                    let mut text = TheText::new(TheId::empty());
                    text.set_text(value.to_kind());
                    let mut name_edit = TheTextLineEdit::new(TheId::named(
                        format!("Atom {}", value.to_kind()).as_str(),
                    ));
                    // name_edit.set_range(TheValue::RangeI32(core::ops::RangeInclusive::new(
                    //     std::i32::MIN,
                    //     std::i32::MAX,
                    // )));
                    name_edit.set_text(v.to_string());
                    name_edit.set_needs_redraw(true);
                    layout.add_widget(Box::new(text));
                    layout.add_widget(Box::new(name_edit));
                }
                TheValue::Float(v) => {
                    let mut text = TheText::new(TheId::empty());
                    text.set_text(value.to_kind());
                    let mut name_edit = TheTextLineEdit::new(TheId::named(
                        format!("Atom {}", value.to_kind()).as_str(),
                    ));
                    // name_edit.set_range(TheValue::RangeF32(core::ops::RangeInclusive::new(
                    //     std::f32::MIN,
                    //     std::f32::MAX,
                    // )));
                    name_edit.set_text(v.to_string());
                    name_edit.set_needs_redraw(true);
                    layout.add_widget(Box::new(text));
                    layout.add_widget(Box::new(name_edit));
                }
                TheValue::Bool(bool) => {
                    let mut text = TheText::new(TheId::empty());
                    text.set_text("Boolean".to_string());
                    let mut drop_down = TheDropdownMenu::new(TheId::named("Atom Bool"));
                    drop_down.add_option("False".to_string());
                    drop_down.add_option("True".to_string());
                    drop_down.set_selected_index(if *bool { 1 } else { 0 });
                    layout.add_widget(Box::new(text));
                    layout.add_widget(Box::new(drop_down));
                }
                _ => {
                    let mut text = TheText::new(TheId::empty());
                    text.set_text(value.to_kind());
                    let mut name_edit = TheTextLineEdit::new(TheId::named(
                        format!("Atom {}", value.to_kind()).as_str(),
                    ));
                    name_edit.set_text(value.describe());
                    name_edit.set_needs_redraw(true);
                    layout.add_widget(Box::new(text));
                    layout.add_widget(Box::new(name_edit));
                }
            },
            _ => {}
        };
    }

    // #[cfg(feature = "ui")]
    // / Generates a text layout to edit the properties of the atom
    // pub fn process_value_change(&mut self, name: String, value: TheValue) {
    //     match self {
    //         TheCodeAtom::Value(_) => {
    //             //println!("{} {:?}", name, value);
    //             if name == "Atom Integer Edit" {
    //                 *self = TheCodeAtom::Value(value.clone());
    //             }
    //         }
    //         _ => {}
    //     };
    // }
}

#[allow(dead_code)]
#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub enum TheCodeAtomKind {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Dollar,
    Colon,
    Percent,

    LineFeed,
    Space,
    Quotation,
    Unknown,
    SingeLineComment,
    HexColor,

    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    Identifier,
    String,
    Number,

    // Keywords.
    And,
    Class,
    Else,
    False,
    For,
    Fn,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Let,
    While,
    CodeBlock,

    Error,
    Eof,
}
