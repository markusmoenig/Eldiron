use denrim_materials::ffi::*;
use std::{
    ffi::{CStr, CString},
    ptr,
};

#[test]
fn c_abi_ownership_sampling_and_diagnostics() {
    let source = CString::new(include_str!("../examples/warped-disc.material")).unwrap();
    let mut error = ptr::null_mut();
    unsafe {
        let program = dm_compile(source.as_ptr(), &mut error);
        assert!(!program.is_null());
        assert!(error.is_null());
        let mut values = [-99.; 3];
        assert_eq!(
            dm_sample(
                program,
                c"demo".as_ptr(),
                c"height".as_ptr(),
                0.,
                0.,
                values.as_mut_ptr(),
                &mut error
            ),
            1
        );
        assert!(values[0].is_finite());
        assert!(error.is_null());
        let shader = dm_metal(
            program,
            c"demo".as_ptr(),
            c"base color".as_ptr(),
            &mut error,
        );
        assert!(
            CStr::from_ptr(shader)
                .to_str()
                .unwrap()
                .contains("material_preview")
        );
        dm_string_free(shader);
        let original = values;
        assert_eq!(
            dm_sample(
                program,
                c"absent".as_ptr(),
                c"height".as_ptr(),
                0.,
                0.,
                values.as_mut_ptr(),
                &mut error
            ),
            -1
        );
        assert_eq!(values, original);
        assert!(
            CStr::from_ptr(error)
                .to_str()
                .unwrap()
                .contains("Unknown material")
        );
        dm_string_free(error);
        dm_program_free(program);
        let program = dm_compile(ptr::null(), &mut error);
        assert!(program.is_null());
        assert!(!error.is_null());
        dm_string_free(error);
        dm_program_free(ptr::null_mut());
        dm_string_free(ptr::null_mut());
    }
}

#[test]
fn swift_bridge_exposes_live_edits_catalog_and_buffer_ownership() {
    let source = CString::new(include_str!("../examples/warped-disc.material")).unwrap();
    let mut error = ptr::null_mut();
    unsafe {
        let catalog = dm_catalog_json(&mut error);
        assert!(
            CStr::from_ptr(catalog)
                .to_str()
                .unwrap()
                .contains("\"type\":\"vec2\"")
        );
        dm_string_free(catalog);
        let p = dm_compile(source.as_ptr(), &mut error);
        let s = dm_shader_compile(p, c"demo".as_ptr(), c"base color".as_ptr(), &mut error);
        assert!(!s.is_null());
        let count = dm_shader_float_count(s);
        let mut values = vec![-99.; count];
        assert_eq!(
            dm_shader_copy_parameters(s, values.as_mut_ptr(), count, &mut error),
            count as i32
        );
        let old = values.clone();
        assert_eq!(
            dm_shader_copy_parameters(s, values.as_mut_ptr(), 1, &mut error),
            -1
        );
        assert_eq!(values, old);
        dm_string_free(error);
        let value = [0.1];
        assert_eq!(
            dm_set_constant(
                p,
                c"demo".as_ptr(),
                c"shape".as_ptr(),
                1,
                c"globalValue".as_ptr(),
                value.as_ptr(),
                1,
                &mut error
            ),
            1
        );
        assert_eq!(dm_shader_refresh(s, p, &mut error), 1);
        assert_eq!(
            dm_shader_copy_parameters(s, values.as_mut_ptr(), count, &mut error),
            count as i32
        );
        assert_ne!(values, old);
        let text = dm_project_text(p, &mut error);
        assert!(CStr::from_ptr(text).to_str().unwrap().contains("to 0.1"));
        dm_string_free(text);
        let layout = dm_shader_layout_json(s, &mut error);
        assert!(
            CStr::from_ptr(layout)
                .to_str()
                .unwrap()
                .contains("\"stride_bytes\":16")
        );
        dm_string_free(layout);
        // Shader data remains valid after its originating document is destroyed.
        dm_program_free(p);
        let source = dm_shader_source(s, &mut error);
        assert!(
            CStr::from_ptr(source)
                .to_str()
                .unwrap()
                .contains("buffer(2)")
        );
        dm_string_free(source);
        dm_shader_free(s);
        assert!(error.is_null());
    }
}

#[test]
fn tile_bridge_exposes_optional_distance_and_rgba_thumbnails() {
    let text = CString::new(include_str!("../examples/icon.material")).unwrap();
    let mut error = ptr::null_mut();
    unsafe {
        let p = dm_compile(text.as_ptr(), &mut error);
        assert!(!p.is_null());
        let mut output = [-99.; 5];
        assert_eq!(
            dm_tile_sample(
                p,
                c"icon".as_ptr(),
                c"styled".as_ptr(),
                1,
                0.,
                0.,
                0.001,
                output.as_mut_ptr(),
                &mut error
            ),
            1
        );
        assert!(output[4] < 0.);
        assert_eq!(output[3], 1.);
        let shader = dm_tile_shader_compile(
            p,
            c"icon".as_ptr(),
            c"styled".as_ptr(),
            1,
            c"image".as_ptr(),
            &mut error,
        );
        assert!(!shader.is_null());
        dm_shader_free(shader);
        dm_program_free(p);
        let text = CString::new(include_str!("../examples/warped-disc.material")).unwrap();
        let p = dm_compile(text.as_ptr(), &mut error);
        assert_eq!(
            dm_tile_sample(
                p,
                c"demo".as_ptr(),
                c"noise".as_ptr(),
                0,
                0.,
                0.,
                0.001,
                output.as_mut_ptr(),
                &mut error
            ),
            0
        );
        assert_eq!(output[4], 0.);
        let before = output;
        assert_eq!(
            dm_tile_sample(
                p,
                c"demo".as_ptr(),
                c"noise".as_ptr(),
                99,
                0.,
                0.,
                0.001,
                output.as_mut_ptr(),
                &mut error
            ),
            -1
        );
        assert_eq!(before, output);
        dm_string_free(error);
        dm_program_free(p);
    }
}
