//! What differs between the desktop systems (Windows, Linux, macOS) and Android:
//! the clipboard, file dialogs, and the parts of the screen covered by system UI.

use eframe::egui;
use eframe::egui::epaint::MarginF32;
use std::path::PathBuf;

/// Phones and tablets: no native file dialogs, touch input, an on-screen keyboard.
pub const MOBILE: bool = cfg!(target_os = "android");

/// How shortcuts are spelled in tooltips: `⌘S` on macOS, `Ctrl+S` elsewhere.
pub fn shortcut(keys: &str) -> String {
    if cfg!(target_os = "macos") {
        keys.replace("Ctrl+Shift+", "⇧⌘")
            .replace("Ctrl+", "⌘")
            .replace("Alt+", "⌥")
    } else {
        keys.to_owned()
    }
}

/// Put `text` on the system clipboard.
pub fn copy(ctx: &egui::Context, text: String) {
    #[cfg(target_os = "android")]
    android::set_clipboard(&text);
    ctx.copy_text(text);
}

/// Read the system clipboard. `None` if it can't be read (or is empty).
pub fn paste() -> Option<String> {
    #[cfg(not(target_os = "android"))]
    {
        arboard::Clipboard::new()
            .and_then(|mut c| c.get_text())
            .ok()
    }
    #[cfg(target_os = "android")]
    {
        android::clipboard()
    }
}

/// Native "Open" dialog for `.sp` files. Android has none: the app lists [`documents_dir`] itself.
#[cfg(not(target_os = "android"))]
pub fn pick_open() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Specification (*.sp)", &["sp"])
        .add_filter("All files", &["*"])
        .pick_file()
}

/// Native "Save as" dialog.
#[cfg(not(target_os = "android"))]
pub fn pick_save(name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Specification (*.sp)", &["sp"])
        .set_file_name(format!("{name}.sp"))
        .save_file()
}

/// Where `.sp` files live on Android (the app's own folder in shared storage, which a computer
/// connected over USB sees as `Android/data/<package>/files`).
pub fn documents_dir() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    {
        android::documents_dir()
    }
    #[cfg(not(target_os = "android"))]
    {
        None
    }
}

/// Screen edges covered by the status bar, navigation bar or on-screen keyboard, in points.
pub fn insets(ctx: &egui::Context) -> MarginF32 {
    #[cfg(target_os = "android")]
    {
        android::insets(ctx)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = ctx;
        MarginF32::ZERO
    }
}

#[cfg(target_os = "android")]
pub mod android {
    use eframe::egui;
    use jni::objects::{JObject, JString};
    use jni::{Env, JValue, JavaVM, jni_sig, jni_str};
    use std::path::PathBuf;
    use std::sync::OnceLock;
    pub use winit::platform::android::activity::AndroidApp;
    use super::MarginF32;
    use winit::platform::android::activity::Rect;

    static APP: OnceLock<AndroidApp> = OnceLock::new();

    pub fn init(app: &AndroidApp) {
        let _ = APP.set(app.clone());
    }

    pub fn documents_dir() -> Option<PathBuf> {
        let app = APP.get()?;
        let dir = app
            .external_data_path()
            .or_else(|| app.internal_data_path())?
            .join("specifications");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    pub fn insets(ctx: &egui::Context) -> MarginF32 {
        let Some(win) = APP.get().and_then(|a| a.native_window()) else {
            return MarginF32::ZERO;
        };
        let (w, h) = (win.width(), win.height());
        if w <= 0 || h <= 0 {
            return MarginF32::ZERO;
        }
        // the part of the window not covered by system bars or the keyboard, in pixels: the
        // activity's content rect misses most of the keyboard, so also ask the window itself
        let mut covered = [0; 4]; // left, top, right, bottom
        let rects = [APP.get().map(|a| a.content_rect()), visible_frame()];
        for r in rects.into_iter().flatten() {
            if r.right > r.left && r.bottom > r.top {
                let edges = [r.left, r.top, w - r.right, h - r.bottom];
                for (c, e) in covered.iter_mut().zip(edges) {
                    *c = (*c).max(e);
                }
            }
        }
        let ppp = ctx.pixels_per_point();
        let pt = |px: i32| px.max(0) as f32 / ppp;
        MarginF32 {
            left: pt(covered[0]),
            top: pt(covered[1]),
            right: pt(covered[2]),
            bottom: pt(covered[3]),
        }
    }

    /// `getWindowVisibleDisplayFrame` of the activity's window, in pixels.
    fn visible_frame() -> Option<Rect> {
        with_activity(|env, activity| {
            let window = env
                .call_method(activity, jni_str!("getWindow"), jni_sig!("()Landroid/view/Window;"), &[])?
                .l()?;
            let decor = env
                .call_method(&window, jni_str!("getDecorView"), jni_sig!("()Landroid/view/View;"), &[])?
                .l()?;
            let rect = env.new_object(jni_str!("android/graphics/Rect"), jni_sig!("()V"), &[])?;
            env.call_method(
                &decor,
                jni_str!("getWindowVisibleDisplayFrame"),
                jni_sig!("(Landroid/graphics/Rect;)V"),
                &[JValue::Object(&rect)],
            )?;
            let mut field = |name| env.get_field(&rect, name, jni_sig!("I"))?.i();
            Ok(Rect {
                left: field(jni_str!("left"))?,
                top: field(jni_str!("top"))?,
                right: field(jni_str!("right"))?,
                bottom: field(jni_str!("bottom"))?,
            })
        })
    }

    /// Run `f` with a JNI environment and the activity.
    fn with_activity<T>(f: impl FnOnce(&mut Env, &JObject) -> jni::errors::Result<T>) -> Option<T> {
        let ctx = ndk_context::android_context();
        // SAFETY: android-activity fills in ndk-context with the process' VM and the activity
        let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) };
        let r = vm.attach_current_thread(|env| -> jni::errors::Result<Option<T>> {
            // SAFETY: a global reference to the activity, valid while the app runs
            let activity = unsafe { JObject::from_raw(env, ctx.context().cast()) };
            let r = f(env, &activity);
            if env.exception_check() {
                env.exception_clear(); // e.g. a SecurityException: report "no clipboard" instead
            }
            Ok(r.ok())
        });
        r.ok().flatten()
    }

    fn clipboard_manager<'l>(
        env: &mut Env<'l>,
        activity: &JObject,
    ) -> jni::errors::Result<JObject<'l>> {
        let name = env.new_string("clipboard")?;
        env.call_method(
            activity,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[JValue::Object(&name)],
        )?
        .l()
    }

    pub fn set_clipboard(text: &str) {
        with_activity(|env, activity| {
            let cm = clipboard_manager(env, activity)?;
            let label = env.new_string("speci-local")?;
            let text = env.new_string(text)?;
            let clip = env
                .call_static_method(
                    jni_str!("android/content/ClipData"),
                    jni_str!("newPlainText"),
                    jni_sig!(
                        "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"
                    ),
                    &[JValue::Object(&label), JValue::Object(&text)],
                )?
                .l()?;
            env.call_method(
                &cm,
                jni_str!("setPrimaryClip"),
                jni_sig!("(Landroid/content/ClipData;)V"),
                &[JValue::Object(&clip)],
            )?;
            Ok(())
        });
    }

    pub fn clipboard() -> Option<String> {
        with_activity(|env, activity| {
            let cm = clipboard_manager(env, activity)?;
            let clip = env
                .call_method(
                    &cm,
                    jni_str!("getPrimaryClip"),
                    jni_sig!("()Landroid/content/ClipData;"),
                    &[],
                )?
                .l()?;
            if clip.is_null() {
                return Ok(None);
            }
            let n = env
                .call_method(&clip, jni_str!("getItemCount"), jni_sig!("()I"), &[])?
                .i()?;
            if n < 1 {
                return Ok(None);
            }
            let item = env
                .call_method(
                    &clip,
                    jni_str!("getItemAt"),
                    jni_sig!("(I)Landroid/content/ClipData$Item;"),
                    &[JValue::Int(0)],
                )?
                .l()?;
            let seq = env
                .call_method(
                    &item,
                    jni_str!("coerceToText"),
                    jni_sig!("(Landroid/content/Context;)Ljava/lang/CharSequence;"),
                    &[JValue::Object(activity)],
                )?
                .l()?;
            if seq.is_null() {
                return Ok(None);
            }
            let s = env
                .call_method(
                    &seq,
                    jni_str!("toString"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            let s = env.cast_local::<JString>(s)?;
            Ok(Some(s.try_to_string(env)?))
        })
        .flatten()
        .filter(|s| !s.is_empty())
    }
}
