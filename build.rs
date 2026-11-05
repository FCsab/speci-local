// Embeds the application icon into the Windows .exe (shown in Explorer, the taskbar and Alt+Tab).
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().expect("embedding the Windows icon failed");
    }
}
