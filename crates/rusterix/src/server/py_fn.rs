// use crate::RegionCtx;
// use crate::server::region::{add_debug_value, with_regionctx};
use rand::*;
use vek::Vec2;

/// Find a random poition max_distance away from pos.
pub fn find_random_position(pos: Vec2<f32>, max_distance: f32) -> Vec2<f32> {
    let mut rng = rand::rng();
    let angle = rng.random_range(0.0..std::f32::consts::TAU);
    let dx = max_distance * angle.cos();
    let dy = max_distance * angle.sin();
    Vec2::new(pos.x + dx, pos.y + dy)
}
/*
use rustpython::vm::*;
use theframework::prelude::TheValue;

use rustpython_vm::builtins::PyDict;

/// Extracts a dictionary to a string from a Python object.
pub fn extract_dictionary(py_dict: PyObjectRef, vm: &VirtualMachine) -> PyResult<String> {
    // Helper function to format Python values correctly
    fn format_python_value(value: PyObjectRef, vm: &VirtualMachine) -> PyResult<String> {
        if value.class().is(vm.ctx.types.bool_type) {
            let val: bool = bool::try_from_object(vm, value)?;
            Ok(if val {
                "True".to_string()
            } else {
                "False".to_string()
            }) // Convert to Python True/False
        } else if value.class().is(vm.ctx.types.int_type) {
            let val: i32 = i32::try_from_object(vm, value)?;
            Ok(val.to_string()) // Convert integer to string
        } else if value.class().is(vm.ctx.types.float_type) {
            let val: f64 = f64::try_from_object(vm, value)?;
            Ok(format!("{}", val)) // Convert float to string
        } else if value.class().is(vm.ctx.types.str_type) {
            let val: String = String::try_from_object(vm, value)?;
            Ok(format!("\"{}\"", val)) // Strings need quotes
        } else if value.class().is(vm.ctx.types.dict_type) {
            extract_dictionary(value, vm) // Recursive conversion for nested dicts
        } else if value.class().is(vm.ctx.types.tuple_type) {
            let tuple: Vec<PyObjectRef> = Vec::<PyObjectRef>::try_from_object(vm, value)?;
            let elements: Vec<String> = tuple
                .into_iter()
                .map(|item| format_python_value(item, vm))
                .collect::<PyResult<Vec<String>>>()?;
            Ok(format!("({})", elements.join(", "))) // Format as a Python tuple
        } else {
            Ok(format!("{}", value.str(vm)?)) // Default: Use Python str() representation
        }
    }

    let dict = py_dict
        .downcast::<PyDict>()
        .map_err(|_| vm.new_type_error("Expected a dictionary".to_string()))?;

    let mut output = String::from("{\n");

    // Iterate over dictionary items and format as Python dictionary syntax
    for (key, value) in dict.into_iter() {
        let key_str = key.str(vm)?.to_string(); // Convert key to a Python-valid string
        let value_str = format_python_value(value, vm)?; // Convert value properly

        output.push_str(&format!("    \"{}\": {},\n", key_str, value_str));
    }

    output.push('}');

    Ok(output) // Return as Python-formatted string
}

/// Generate an i32 or f32 random number within the given range.
pub fn random_in_range(
    from: PyObjectRef,
    to: PyObjectRef,
    vm: &VirtualMachine,
) -> PyResult<PyObjectRef> {
    if from.class().is(vm.ctx.types.int_type) && to.class().is(vm.ctx.types.int_type) {
        // Extract integers
        let start: i32 = from.try_into_value(vm)?;
        let end: i32 = to.try_into_value(vm)?;

        if start <= end {
            // Generate a random i32 within the range
            let mut rng = rand::rng();
            let result = rng.random_range(start..=end);

            with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Int(result), false);
                }
            });

            Ok(vm.ctx.new_int(result).into())
        } else {
            with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Text("Invalid Range".into()), true);
                }
            });
            Err(vm.new_type_error("Start > End".to_string()))
        }
    } else if from.class().is(vm.ctx.types.float_type) && to.class().is(vm.ctx.types.float_type) {
        // Extract floats
        let start: f64 = from.try_into_value(vm)?;
        let end: f64 = to.try_into_value(vm)?;

        if start <= end {
            // Generate a random f64 within the range
            let mut rng = rand::rng();
            let result = rng.random_range(start..=end);

            with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Float(result as f32), false);
                }
            });

            Ok(vm.ctx.new_float(result).into())
        } else {
            with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Text("Invalid Range".into()), true);
                }
            });
            Err(vm.new_type_error("Start > End".to_string()))
        }
    } else {
        with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
            if ctx.debug_mode {
                add_debug_value(ctx, TheValue::Text("Invalid Range".into()), true);
            }
        });
        // If the inputs are not valid numbers, raise a TypeError
        Err(vm.new_type_error("Both from and to must be integers or floats".to_string()))
    }
}

/// Get an i32 value from an Python object with a default fallback.
pub fn get_i32(value: PyObjectRef, default: i32, vm: &VirtualMachine) -> i32 {
    if value.class().is(vm.ctx.types.int_type) {
        value.try_into_value::<i32>(vm).unwrap_or(default)
    } else if value.class().is(vm.ctx.types.float_type) {
        value
            .try_into_value::<f32>(vm)
            .map(|v| v as i32)
            .unwrap_or(default) // Convert f32 to i32
    } else {
        default
    }
}

/// Get an f32 value from an Python object with a default fallback.
pub fn get_f32(value: PyObjectRef, default: f32, vm: &VirtualMachine) -> f32 {
    if value.class().is(vm.ctx.types.int_type) {
        value
            .try_into_value::<i32>(vm)
            .map(|v| v as f32)
            .unwrap_or(default)
    } else {
        value.try_into_value::<f32>(vm).unwrap_or(default)
    }
}
*/
