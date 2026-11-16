use super::*;
use std::ptr;
unsafe fn read_output(f: impl FnOnce(*mut *const u8, *mut usize) -> i32) -> Vec<u8> {
    let mut data = ptr::null();
    let mut len = 0;
    assert_eq!(f(&mut data, &mut len), 0);
    let result = if len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
    };
    unsafe { unified_app_runner_free_data(data, len) };
    result
}
#[test]
fn apple_abi_validates_nulls_keys_and_utf8_without_creating_a_gpu() {
    assert!(unsafe { unified_app_runner_create(ptr::null_mut(), 1, 1, 1.) }.is_null());
    assert_eq!(unsafe { unified_app_runner_render(ptr::null_mut()) }, -1);
    let mut output = ptr::dangling();
    let mut len = 123;
    assert_eq!(
        unsafe { unified_app_runner_save_project(ptr::null_mut(), &mut output, &mut len) },
        -1
    );
    assert!(output.is_null());
    assert_eq!(len, 0);
    assert_eq!(physical(b'W' as u32), PhysicalKey::Letter(b'w'));
    assert_eq!(physical(b'9' as u32), PhysicalKey::Digit(9));
    assert_eq!(physical(0x108), PhysicalKey::Up);
    assert_eq!(physical(0x218), PhysicalKey::Function(24));
    assert_eq!(physical(0xffff), PhysicalKey::Other(0xffff));
    assert!(unsafe { text([0xff].as_ptr(), 1) }.is_err());
    unsafe {
        unified_app_runner_destroy(ptr::null_mut());
        unified_app_runner_free_data(ptr::null(), 0);
    }
}
#[test]
#[ignore = "requires native Metal; exercises the real exported Apple player ABI"]
fn apple_abi_presents_shared_scene_ui_and_preserves_project_and_callback_ownership() {
    let layer = objc2_quartz_core::CAMetalLayer::new();
    let layer_ptr = (&*layer as *const objc2_quartz_core::CAMetalLayer)
        .cast_mut()
        .cast();
    let ptr = unsafe { unified_app_runner_create(layer_ptr, 960, 720, 1.5) };
    assert!(!ptr.is_null());
    let project = crate::native_player::tests::starter_with_menu();
    let source = shared::project_io::encode_project(&project).unwrap();
    assert_eq!(
        unsafe { unified_app_runner_load_project(ptr, source.as_ptr(), source.len()) },
        0
    );
    assert_eq!(unsafe { unified_app_runner_render(ptr) }, 0);
    assert_eq!(unsafe { (*ptr).core.presentations() }, 1);
    let p = unsafe {
        (*ptr)
            .core
            .application
            .as_ref()
            .unwrap()
            .viewport()
            .transform()
            .point([300., 220.])
    };
    unsafe {
        unified_app_runner_mouse_down(ptr, p[0], p[1]);
        unified_app_runner_mouse_up(ptr, p[0], p[1]);
    }
    let state = unsafe { read_output(|out, len| unified_app_runner_host_state(ptr, out, len)) };
    let state: serde_json::Value = serde_json::from_slice(&state).unwrap();
    let target = state["text_input"]["target"].as_u64().unwrap();
    let edited = "東京";
    assert_eq!(
        unsafe { unified_app_runner_text(ptr, target, edited.as_ptr(), edited.len()) },
        1
    );
    assert!(unsafe {
        (*ptr)
            .core
            .application
            .as_ref()
            .unwrap()
            .screen
            .elements
            .ui_state["start.name"]
            .contains(edited)
    });
    // Invalid bytes/cursor offsets do not mutate the editor and yield an owned,
    // explicit error message rather than unwinding into Swift.
    assert_eq!(
        unsafe { unified_app_runner_text(ptr, target, [0xff].as_ptr(), 1) },
        -1
    );
    assert_eq!(
        unsafe { unified_app_runner_composition(ptr, target, edited.as_ptr(), edited.len(), 1, 2) },
        -1
    );
    let error = unsafe { read_output(|out, len| unified_app_runner_last_error(ptr, out, len)) };
    assert!(
        String::from_utf8(error)
            .unwrap()
            .contains("UTF-8 byte boundaries")
    );
    // The new native protocol uses displayed-source revisions and UTF-16 ranges.
    let native: serde_json::Value = serde_json::from_slice(&unsafe {
        read_output(|out, len| unified_app_runner_text_snapshot(ptr, out, len))
    })
    .unwrap();
    let end = native["text"].as_str().unwrap().encode_utf16().count() as isize;
    let value = "A👩Z";
    assert_eq!(
        unsafe {
            unified_app_runner_text_edit(
                ptr,
                target,
                native["revision"].as_u64().unwrap(),
                1,
                0,
                end,
                value.as_ptr(),
                value.len(),
                -1,
                -1,
            )
        },
        1
    );
    let native: serde_json::Value = serde_json::from_slice(&unsafe {
        read_output(|out, len| unified_app_runner_text_snapshot(ptr, out, len))
    })
    .unwrap();
    assert_eq!(native["text"], value);
    let revision = native["revision"].as_u64().unwrap();
    assert_eq!(
        unsafe {
            unified_app_runner_text_edit(ptr, target, revision, 1, 1, 2, b"bad".as_ptr(), 3, -1, -1)
        },
        -1,
        "do not split the emoji surrogate pair"
    );
    let geometry: serde_json::Value = serde_json::from_slice(&unsafe {
        read_output(|out, len| {
            unified_app_runner_text_query(ptr, target, revision, 0, 1, 3, 0., 0., out, len)
        })
    })
    .unwrap();
    assert_eq!(geometry["first_range"], serde_json::json!([1, 3]));
    let caret: serde_json::Value = serde_json::from_slice(&unsafe {
        read_output(|out, len| {
            unified_app_runner_text_query(ptr, target, revision, 0, 3, 3, 0., 0., out, len)
        })
    })
    .unwrap();
    let rect = caret["caret"].as_array().unwrap();
    let x = rect[0].as_f64().unwrap() as f32;
    let y = (rect[1].as_f64().unwrap() + rect[3].as_f64().unwrap() / 2.) as f32;
    let hit: serde_json::Value = serde_json::from_slice(&unsafe {
        read_output(|out, len| {
            unified_app_runner_text_query(ptr, target, revision, 1, -1, -1, x, y, out, len)
        })
    })
    .unwrap();
    assert_eq!(hit["position"], 3);
    let moved: serde_json::Value = serde_json::from_slice(&unsafe {
        read_output(|out, len| {
            unified_app_runner_text_query(ptr, target, revision, 2, 3, 1, 0., 0., out, len)
        })
    })
    .unwrap();
    assert_eq!(
        moved["position"], 1,
        "native visual movement crosses the emoji as one grapheme"
    );
    let stale: serde_json::Value = serde_json::from_slice(&unsafe {
        read_output(|out, len| {
            unified_app_runner_text_query(
                ptr,
                target,
                revision.wrapping_sub(1),
                0,
                0,
                0,
                0.,
                0.,
                out,
                len,
            )
        })
    })
    .unwrap();
    assert!(stale.is_null());
    let preedit = "日本";
    assert_eq!(
        unsafe {
            unified_app_runner_text_edit(
                ptr,
                target,
                revision,
                2,
                1,
                3,
                preedit.as_ptr(),
                preedit.len(),
                2,
                2,
            )
        },
        1
    );
    assert_eq!(
        unsafe {
            (*ptr)
                .core
                .application
                .as_ref()
                .unwrap()
                .screen
                .elements
                .ui_state["start.name"]
                .as_str()
        },
        value
    );
    let marked: serde_json::Value = serde_json::from_slice(&unsafe {
        read_output(|out, len| unified_app_runner_text_snapshot(ptr, out, len))
    })
    .unwrap();
    assert_eq!(marked["text"], "A日本Z");
    assert_eq!(marked["marked"], serde_json::json!([1, 3]));
    assert_eq!(
        unsafe {
            unified_app_runner_text_edit(
                ptr,
                target,
                revision,
                1,
                0,
                1,
                b"stale".as_ptr(),
                5,
                -1,
                -1,
            )
        },
        0
    );
    assert_eq!(
        unsafe {
            unified_app_runner_text_edit(
                ptr,
                target,
                marked["revision"].as_u64().unwrap(),
                3,
                -1,
                -1,
                ptr::null(),
                0,
                -1,
                -1,
            )
        },
        1
    );
    assert_eq!(
        unsafe {
            (*ptr)
                .core
                .application
                .as_ref()
                .unwrap()
                .screen
                .elements
                .ui_state["start.name"]
                .as_str()
        },
        "A日本Z"
    );
    unsafe {
        unified_app_runner_modifiers(ptr, 8);
        unified_app_runner_key(ptr, b'Z' as u32, 1, 0);
        unified_app_runner_key(ptr, b'Z' as u32, 0, 0);
        unified_app_runner_modifiers(ptr, 0);
    }
    assert_eq!(
        unsafe {
            (*ptr)
                .core
                .application
                .as_ref()
                .unwrap()
                .screen
                .elements
                .ui_state["start.name"]
                .as_str()
        },
        value,
        "native unmark participates in shared undo"
    );
    let saved = unsafe { read_output(|out, len| unified_app_runner_save_project(ptr, out, len)) };
    assert_eq!(saved, source);
    let session = unsafe {
        (*ptr)
            .core
            .application
            .as_ref()
            .unwrap()
            .player
            .runtime
            .view()
            .session_id
    };
    assert_eq!(
        unsafe { unified_app_runner_load_project(ptr, b"invalid".as_ptr(), 7) },
        -1
    );
    assert_eq!(
        unsafe {
            (*ptr)
                .core
                .application
                .as_ref()
                .unwrap()
                .player
                .runtime
                .view()
                .session_id
        },
        session
    );
    // Capture an actual native clipboard request, replace the document, then
    // deliver the old completion: it cannot type into the replacement field.
    unsafe {
        unified_app_runner_modifiers(ptr, 8);
        unified_app_runner_key(ptr, b'V' as u32, 1, 0);
        unified_app_runner_key(ptr, b'V' as u32, 0, 0);
        unified_app_runner_modifiers(ptr, 0);
    }
    let requests =
        unsafe { read_output(|out, len| unified_app_runner_take_requests(ptr, out, len)) };
    let requests: Vec<serde_json::Value> = serde_json::from_slice(&requests).unwrap();
    let request = requests
        .iter()
        .find(|r| r["kind"] == "clipboard_read")
        .unwrap()["request"]
        .as_u64()
        .unwrap();
    assert_eq!(
        unsafe { unified_app_runner_load_project(ptr, source.as_ptr(), source.len()) },
        0
    );
    assert_ne!(
        unsafe {
            (*ptr)
                .core
                .application
                .as_ref()
                .unwrap()
                .player
                .runtime
                .view()
                .session_id
        },
        session
    );
    assert_eq!(
        unsafe { unified_app_runner_clipboard(ptr, request, 0, 1, b"late".as_ptr(), 4) },
        0
    );
    assert_eq!(
        unsafe { unified_app_runner_text(ptr, target, b"late".as_ptr(), 4) },
        0
    );
    assert_eq!(
        unsafe {
            (*ptr)
                .core
                .application
                .as_ref()
                .unwrap()
                .screen
                .elements
                .ui_state["start.name"]
                .as_str()
        },
        "Hero"
    );
    // Reject a serialized callback from a different host thread before touching
    // any Metal or application state. No concurrent access occurs in this test.
    let address = ptr as usize;
    assert_eq!(
        std::thread::spawn(move || unsafe {
            unified_app_runner_render(address as *mut EldironPlayerRunner)
        })
        .join()
        .unwrap(),
        -1
    );
    assert_eq!(unsafe { unified_app_runner_render(ptr) }, 0);
    let p = unsafe {
        (*ptr)
            .core
            .application
            .as_ref()
            .unwrap()
            .viewport()
            .transform()
            .point([480., 280.])
    };
    unsafe {
        unified_app_runner_mouse_down(ptr, p[0], p[1]);
        unified_app_runner_mouse_up(ptr, p[0], p[1]);
    }
    let deadline = Instant::now() + std::time::Duration::from_secs(10);
    loop {
        unsafe {
            (*ptr).core.update(std::time::Duration::from_millis(250));
        }
        assert_eq!(unsafe { unified_app_runner_render(ptr) }, 0);
        let app = unsafe { (*ptr).core.application.as_ref().unwrap() };
        if app.player.runtime.server.has_local_player() && !app.screen.view_images().is_empty() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "registered player scene did not become presentable"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(unsafe { (*ptr).core.presentations() } > 2);
    assert_eq!(
        unsafe { (*ptr).core.presenter.context().transfers().readback_bytes },
        0
    );
    assert!(unsafe {
        (*ptr)
            .core
            .application
            .as_ref()
            .unwrap()
            .screen
            .elements
            .game_widgets
            .values()
            .all(|w| w.buffer.pixels().is_empty() && w.upscale_buffer.pixels().is_empty())
    });
    let saved = unsafe { read_output(|out, len| unified_app_runner_save_project(ptr, out, len)) };
    assert_eq!(
        saved, source,
        "playing cannot serialize transient runtime state"
    );
    unsafe {
        unified_app_runner_suspend(ptr);
        unified_app_runner_resume(ptr);
        unified_app_runner_destroy(ptr);
    }
    drop(layer);
}
