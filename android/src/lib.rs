//! Android entry point: the NativeActivity calls `android_main` on its own thread.

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: speci_local::platform::android::AndroidApp) {
    // stderr goes nowhere on Android: send log messages and panics to logcat (`adb logcat -s speci-local`)
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("speci-local"),
    );
    std::panic::set_hook(Box::new(|info| log::error!("{info}")));
    if let Err(e) = speci_local::app::run_android(app) {
        log::error!("{e}");
    }
}
