#![no_std]
//! Font data used by TheFramework's browser text fallback.

/// Second part of the Noto Sans CJK Regular font collection.
pub const NOTO_SANS_CJK_REGULAR_TAIL: &[u8] = include_bytes!("../NotoSansCJK-Regular.ttc.part2");
