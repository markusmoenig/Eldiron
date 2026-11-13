//! Owned immutable PNG transfer for foreign hosts. No Swift buffer/pointer is
//! retained after a callback; stale handles and undersized copies are rejected.
use super::*;
/// Query with null output; insufficient capacity returns the required length
/// without touching output. The PNG remains valid until completion/replacement.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_clipboard_image_copy_v1(
    handle: u64,
    request: u64,
    out: *mut u8,
    capacity: usize,
) -> usize {
    with_hosted_handle(handle, |host| {
        let Some((id, bytes)) = &host.clipboard_png else {
            return 0;
        };
        if *id != request {
            return 0;
        }
        if !out.is_null() && capacity >= bytes.len() {
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
            }
        }
        bytes.len()
    })
    .unwrap_or(0)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_clipboard_image_complete_v1(
    handle: u64,
    request: u64,
    write: bool,
    success: bool,
    data: *const u8,
    length: usize,
) -> bool {
    if length > theframework::platform::image_clipboard::MAX_BYTES || (length > 0 && data.is_null())
    {
        return false;
    }
    with_hosted_handle(handle, |host| {
        let result = if write {
            if host
                .clipboard_png
                .as_ref()
                .is_none_or(|(id, _)| *id != request)
            {
                return false;
            }
            host.clipboard_png = None;
            ImageClipboardResult::Written(success)
        } else {
            let image = if success && length > 0 {
                theframework::platform::image_clipboard::decode_png(unsafe {
                    std::slice::from_raw_parts(data, length)
                })
                .ok()
            } else {
                None
            };
            ImageClipboardResult::Image(image)
        };
        host.app
            .input(&InputEvent::ClipboardImage { request, result });
        true
    })
    .unwrap_or(false)
}
