//! Run the canonical common-player desktop assembly (including --smoke).
fn main() -> Result<(), Box<dyn std::error::Error>> {
    scenevm_unified_app::native_player::desktop::run(std::env::args_os())
}
