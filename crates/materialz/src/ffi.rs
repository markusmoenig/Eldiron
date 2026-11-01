//! Minimal C ABI for Swift. All pointers follow include/denrim_materials.h.
use crate::{
    Error, Result,
    cpu::{Evaluator, PreviewMode},
    ir::Channel,
    live::Document,
    metal,
};
use std::{
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
};

unsafe fn text<'a>(value: *const c_char) -> Result<&'a str> {
    if value.is_null() {
        return Err(Error("Null string pointer".into()));
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|_| Error("String must be UTF-8".into()))
}
fn owned(s: String) -> *mut c_char {
    CString::new(s.replace('\0', "\\0"))
        .expect("NULs escaped")
        .into_raw()
}
unsafe fn boundary<T>(error: *mut *mut c_char, fallback: T, f: impl FnOnce() -> Result<T>) -> T {
    if !error.is_null() {
        unsafe {
            *error = ptr::null_mut();
        }
    }
    let result = catch_unwind(AssertUnwindSafe(f))
        .unwrap_or_else(|_| Err(Error("Internal engine panic".into())));
    match result {
        Ok(v) => v,
        Err(e) => {
            if !error.is_null() {
                unsafe {
                    *error = owned(e.to_string());
                }
            }
            fallback
        }
    }
}

/// Compile an owned opaque program; free it with dm_program_free.
/// # Safety
/// `source` must be a valid NUL-terminated string; `error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_compile(
    source: *const c_char,
    error: *mut *mut c_char,
) -> *mut Document {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            Ok(Box::into_raw(Box::new(Document::parse(text(source)?)?)))
        })
    }
}
/// # Safety
/// `program` must be null or an unfreed pointer returned by dm_compile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_program_free(program: *mut Document) {
    if !program.is_null() {
        unsafe {
            drop(Box::from_raw(program));
        }
    }
}
/// # Safety
/// `string` must be null or an unfreed string returned by this API.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_string_free(string: *mut c_char) {
    if !string.is_null() {
        unsafe {
            drop(CString::from_raw(string));
        }
    }
}
/// Generate an owned Metal source string.
/// # Safety
/// The program must be live, strings valid and `error` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_metal(
    program: *const Document,
    material: *const c_char,
    channel: *const c_char,
    error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            let program = program
                .as_ref()
                .ok_or_else(|| Error("Null program".into()))?;
            Ok(owned(metal::generate(
                program.program(),
                text(material)?,
                Channel::parse(text(channel)?)?,
            )?))
        })
    }
}
/// Sample linear floats. Returns component count (1 or 3), or -1 on error.
/// # Safety
/// The program must be live, strings valid, `output` writable for 3 floats,
/// and `error` null or writable. Output storage must not alias the program.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_sample(
    program: *const Document,
    material: *const c_char,
    channel: *const c_char,
    u: f32,
    v: f32,
    output: *mut f32,
    error: *mut *mut c_char,
) -> i32 {
    unsafe {
        boundary(error, -1, || {
            let program = program
                .as_ref()
                .ok_or_else(|| Error("Null program".into()))?;
            if output.is_null() {
                return Err(Error("Null output buffer".into()));
            }
            let values = Evaluator::new(program.program()).sample(
                text(material)?,
                Channel::parse(text(channel)?)?,
                [u, v],
            )?;
            ptr::copy_nonoverlapping(values.as_ptr(), output, values.len());
            Ok(values.len() as i32)
        })
    }
}

/// Versioned JSON catalog of the original source controls.
/// # Safety
/// `error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_catalog_json(error: *mut *mut c_char) -> *mut c_char {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            Ok(owned(crate::metadata::catalog_json()))
        })
    }
}

/// # Safety
/// `error` must be null or writable pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_bsdf_catalog_json(error: *mut *mut c_char) -> *mut c_char {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            Ok(owned(crate::bsdf::catalog_json()))
        })
    }
}

/// # Safety
/// `program` must be live and strings valid; `error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_project_text(
    program: *const Document,
    error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            let program = program
                .as_ref()
                .ok_or_else(|| Error("Null program".into()))?;
            Ok(owned(program.text()))
        })
    }
}

/// Transactional constant edit. Returns 1 on success, 0 on error.
/// # Safety
/// Exclusive access to a live program is required. Strings must be valid, values
/// readable for `count` floats, and `error` null or writable. No input may alias
/// the program or error storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_set_constant(
    program: *mut Document,
    material: *const c_char,
    row: *const c_char,
    tile: usize,
    parameter: *const c_char,
    values: *const f32,
    count: usize,
    error: *mut *mut c_char,
) -> i32 {
    unsafe {
        boundary(error, 0, || {
            if values.is_null() || !(1..=4).contains(&count) {
                return Err(Error("Expected 1..4 parameter values".into()));
            }
            let program = program
                .as_mut()
                .ok_or_else(|| Error("Null program".into()))?;
            program.set_constant(
                text(material)?,
                text(row)?,
                tile,
                text(parameter)?,
                std::slice::from_raw_parts(values, count),
            )?;
            Ok(1)
        })
    }
}

/// Compile buffered Metal. The returned handle owns its source, layout and values.
/// # Safety
/// The program must be live, strings valid, and `error` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_compile(
    program: *const Document,
    material: *const c_char,
    channel: *const c_char,
    error: *mut *mut c_char,
) -> *mut metal::Shader {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            let program = program
                .as_ref()
                .ok_or_else(|| Error("Null program".into()))?;
            Ok(Box::into_raw(Box::new(metal::compile(
                program.program(),
                text(material)?,
                Channel::parse(text(channel)?)?,
            )?)))
        })
    }
}
/// # Safety
/// `shader` must be null or an unfreed shader handle from this API.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_free(shader: *mut metal::Shader) {
    if !shader.is_null() {
        unsafe {
            drop(Box::from_raw(shader));
        }
    }
}
/// # Safety
/// `shader` must be live and `error` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_source(
    shader: *const metal::Shader,
    error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            Ok(owned(
                shader
                    .as_ref()
                    .ok_or_else(|| Error("Null shader".into()))?
                    .source
                    .clone(),
            ))
        })
    }
}
/// # Safety
/// `shader` must be live and `error` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_layout_json(
    shader: *const metal::Shader,
    error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            Ok(owned(
                shader
                    .as_ref()
                    .ok_or_else(|| Error("Null shader".into()))?
                    .layout_json(),
            ))
        })
    }
}
/// # Safety
/// `shader` must be null or a live shader handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_float_count(shader: *const metal::Shader) -> usize {
    unsafe { shader.as_ref() }.map_or(0, |s| s.parameters().len() * 4)
}
/// Copy packed float4 values. Returns floats written or -1; failure writes nothing.
/// # Safety
/// `shader` must be live, output writable for capacity floats and not alias any
/// input, and `error` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_copy_parameters(
    shader: *const metal::Shader,
    output: *mut f32,
    capacity: usize,
    error: *mut *mut c_char,
) -> i32 {
    unsafe {
        boundary(error, -1, || {
            let shader = shader.as_ref().ok_or_else(|| Error("Null shader".into()))?;
            let count = shader.parameters().len() * 4;
            if output.is_null() || capacity < count {
                return Err(Error("Parameter buffer is null or too small".into()));
            }
            for (i, value) in shader.parameters().iter().flatten().enumerate() {
                *output.add(i) = *value;
            }
            Ok(count as i32)
        })
    }
}
/// Refresh constant data. No generated source changes, no GPU compilation.
/// # Safety
/// Exclusive access to `shader` and shared access to a live program are required;
/// `error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_refresh(
    shader: *mut metal::Shader,
    program: *const Document,
    error: *mut *mut c_char,
) -> i32 {
    unsafe {
        boundary(error, 0, || {
            let shader = shader.as_mut().ok_or_else(|| Error("Null shader".into()))?;
            let program = program
                .as_ref()
                .ok_or_else(|| Error("Null program".into()))?;
            shader.refresh(program.program())?;
            Ok(1)
        })
    }
}

/// RGBA plus optional distance. Returns 1 with distance, 0 for image only,
/// or -1 on error. Absent distance is written as zero.
/// # Safety
/// Handles/strings must be live, output writable for five floats and not alias
/// inputs, and error null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_tile_sample(
    program: *const Document,
    material: *const c_char,
    row: *const c_char,
    tile: usize,
    u: f32,
    v: f32,
    pixel_width: f32,
    output: *mut f32,
    error: *mut *mut c_char,
) -> i32 {
    unsafe {
        boundary(error, -1, || {
            let program = program
                .as_ref()
                .ok_or_else(|| Error("Null program".into()))?;
            if output.is_null() {
                return Err(Error("Null output buffer".into()));
            }
            let sample = Evaluator::new(program.program()).sample_tile(
                text(material)?,
                text(row)?,
                tile,
                [u, v],
                pixel_width,
            )?;
            for (i, value) in sample
                .color
                .into_iter()
                .chain([sample.distance.unwrap_or(0.)])
                .enumerate()
            {
                *output.add(i) = value;
            }
            Ok(i32::from(sample.distance.is_some()))
        })
    }
}

/// Compile a tile thumbnail in image, shape or distance mode.
/// # Safety
/// Handles/strings must be live and error null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_tile_shader_compile(
    program: *const Document,
    material: *const c_char,
    row: *const c_char,
    tile: usize,
    mode: *const c_char,
    error: *mut *mut c_char,
) -> *mut metal::Shader {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            let program = program
                .as_ref()
                .ok_or_else(|| Error("Null program".into()))?;
            Ok(Box::into_raw(Box::new(metal::compile_tile_preview(
                program.program(),
                text(material)?,
                text(row)?,
                tile,
                PreviewMode::parse(text(mode)?)?,
            )?)))
        })
    }
}

/// Structured authoring data for native editors.
/// # Safety
/// program must be live and error null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_project_json(
    program: *const Document,
    error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        boundary(error, std::ptr::null_mut(), || {
            let program = program
                .as_ref()
                .ok_or_else(|| Error("Null program".into()))?;
            Ok(owned(program.json()))
        })
    }
}

/// Selected tile's definition and implementation, not the whole material shader.
/// # Safety
/// The program and strings must be live; error must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_tile_source(
    program: *const Document,
    material: *const c_char,
    row: *const c_char,
    tile: usize,
    error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            let program = program
                .as_ref()
                .ok_or_else(|| Error("Null program".into()))?;
            Ok(owned(metal::tile_source(
                program.program(),
                text(material)?,
                text(row)?,
                tile,
            )?))
        })
    }
}

/// Number of prerequisite image passes. The final shader is obtained normally.
/// # Safety
/// shader must be null or a live shader handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_pass_count(shader: *const metal::Shader) -> usize {
    unsafe { shader.as_ref() }.map_or(0, |s| s.passes.len())
}
/// # Safety
/// shader must be live, index in range, and error null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_pass_source(
    shader: *const metal::Shader,
    index: usize,
    error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            let s = shader.as_ref().ok_or_else(|| Error("Null shader".into()))?;
            Ok(owned(
                s.passes
                    .get(index)
                    .ok_or_else(|| Error("Unknown image pass".into()))?
                    .source
                    .clone(),
            ))
        })
    }
}
/// Key includes input parameters and earlier passes, excluding downstream edits.
/// # Safety
/// shader must be live and error null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_pass_key(
    shader: *const metal::Shader,
    index: usize,
    error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        boundary(error, ptr::null_mut(), || {
            let s = shader.as_ref().ok_or_else(|| Error("Null shader".into()))?;
            Ok(owned(s.pass_key(index)?))
        })
    }
}
/// # Safety
/// shader must be null or a live shader handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_pixel_padding(shader: *const metal::Shader) -> f32 {
    unsafe { shader.as_ref() }.map_or(0., |s| s.pixel_padding)
}
/// # Safety
/// shader must be null or a live shader handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_uv_padding(shader: *const metal::Shader) -> f32 {
    unsafe { shader.as_ref() }.map_or(0., |s| s.uv_padding)
}

/// # Safety
/// shader must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_domain_count(shader: *const metal::Shader) -> usize {
    unsafe { shader.as_ref() }.map_or(0, |s| s.domains.len())
}
/// # Safety
/// shader must be live; output writable for six floats; error null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_domain(
    shader: *const metal::Shader,
    index: usize,
    output: *mut f32,
    error: *mut *mut c_char,
) -> i32 {
    unsafe {
        boundary(error, 0, || {
            let s = shader.as_ref().ok_or_else(|| Error("Null shader".into()))?;
            let d = s
                .domains
                .get(index)
                .ok_or_else(|| Error("Unknown cache domain".into()))?;
            if output.is_null() {
                return Err(Error("Null domain output".into()));
            }
            ptr::copy_nonoverlapping(d.as_ptr(), output, 6);
            Ok(1)
        })
    }
}

/// # Safety
/// shader must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_fixed_domain_count(shader: *const metal::Shader) -> usize {
    unsafe { shader.as_ref() }.map_or(0, |s| s.fixed_domains.len())
}
/// # Safety
/// shader must be live; output writable for four floats; error null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_fixed_domain(
    shader: *const metal::Shader,
    index: usize,
    output: *mut f32,
    error: *mut *mut c_char,
) -> i32 {
    unsafe {
        boundary(error, 0, || {
            let s = shader.as_ref().ok_or_else(|| Error("Null shader".into()))?;
            let d = s
                .fixed_domains
                .get(index)
                .ok_or_else(|| Error("Unknown fixed cache domain".into()))?;
            if output.is_null() {
                return Err(Error("Null domain output".into()));
            }
            ptr::copy_nonoverlapping(d.as_ptr(), output, 4);
            Ok(1)
        })
    }
}

/// # Safety
/// shader must be null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dm_shader_domain_is_fixed(
    shader: *const metal::Shader,
    index: usize,
) -> i32 {
    unsafe { shader.as_ref() }
        .and_then(|s| s.domain_fixed.get(index))
        .copied()
        .unwrap_or(false) as i32
}
