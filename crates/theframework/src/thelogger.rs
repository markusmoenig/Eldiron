use log::LevelFilter;
#[cfg(not(target_arch = "wasm32"))]
use log4rs::{
    append::rolling_file::{
        policy::compound::{
            roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
        },
        RollingFileAppender,
    },
    config::{Appender, Root},
    Config,
};

#[cfg(not(target_arch = "wasm32"))]
fn setup_logfile() {
    let roller = FixedWindowRoller::builder()
        .build("panic.{}.log", 3)
        .unwrap();
    let policy = CompoundPolicy::new(Box::new(SizeTrigger::new(2 * 1024)), Box::new(roller));

    let logfile = RollingFileAppender::builder()
        .build("panic.log", Box::new(policy))
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(LevelFilter::Error),
        )
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();
}

#[cfg(target_arch = "wasm32")]
fn setup_panic_hook() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Trace).expect("error initializing logger");
}

#[cfg(not(target_arch = "wasm32"))]
fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            &s
        } else {
            "Unknown panic"
        };

        log::error!(
            "Panic occurred at {:?}: {}\n",
            panic_info.location().unwrap(),
            payload
        );
    }));
}

pub fn setup_logger() {
    #[cfg(not(target_arch = "wasm32"))]
    setup_logfile();

    setup_panic_hook();
}
