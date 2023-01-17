
/// Returns the resource directory
pub fn get_resource_dir() -> std::path::PathBuf {

    // Uncomment this for macOS bundle, what a pain

    // if let Some(mut curr_exe) = std::env::current_exe().ok() {

    //     curr_exe.pop();
    //     curr_exe.pop();

    //     curr_exe = curr_exe.join("Resources");//.join("_up_");
    //     return curr_exe;
    // }

    std::path::PathBuf::new()
}