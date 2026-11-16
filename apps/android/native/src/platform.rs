use super::Saved;
use std::path::PathBuf;
static SAFE_AREA: std::sync::Mutex<[f32; 4]> = std::sync::Mutex::new([0.; 4]);
static BACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static WEATHER_CITY: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static WEATHER_POSITION: std::sync::Mutex<Option<[f64; 2]>> = std::sync::Mutex::new(None);
static WEATHER_LOCATION_STATUS: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
pub fn take_weather_position() -> Option<[f64; 2]> {
    WEATHER_POSITION
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}
pub fn take_weather_location_status() -> Option<String> {
    WEATHER_LOCATION_STATUS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}
pub fn weather_location() {
    #[cfg(target_os = "android")]
    {
        let _ = java("weatherLocation");
    }
}
pub fn cancel_weather_location() {
    *WEATHER_POSITION.lock().unwrap_or_else(|e| e.into_inner()) = None;
    #[cfg(target_os = "android")]
    {
        let _ = java("cancelWeatherLocation");
    }
}
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_davewagner_puzzgen_MainActivity_nativeWeatherPosition(
    _: jni::JNIEnv,
    _: jni::objects::JClass,
    latitude: f64,
    longitude: f64,
) {
    if let Some(coordinates) = puzzgen_weather::rounded_coordinates(latitude, longitude) {
        *WEATHER_POSITION.lock().unwrap_or_else(|e| e.into_inner()) = Some(coordinates);
    }
}
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_davewagner_puzzgen_MainActivity_nativeWeatherLocationStatus(
    mut env: jni::JNIEnv,
    _: jni::objects::JClass,
    message: jni::objects::JString,
) {
    if let Ok(value) = env.get_string(&message) {
        *WEATHER_LOCATION_STATUS
            .lock()
            .unwrap_or_else(|e| e.into_inner()) =
            Some(String::from(value).chars().take(180).collect());
    }
}
pub fn take_weather_city() -> Option<String> {
    WEATHER_CITY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}
pub fn weather_dialog() {
    cancel_weather_location();
    #[cfg(target_os = "android")]
    {
        let _ = java("weatherDialog");
    }
}
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_davewagner_puzzgen_MainActivity_nativeWeatherCity(
    mut env: jni::JNIEnv,
    _: jni::objects::JClass,
    city: jni::objects::JString,
) {
    if let Ok(s) = env.get_string(&city) {
        let city: String = s.into();
        *WEATHER_POSITION.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *WEATHER_CITY.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(city.trim().chars().take(100).collect());
    }
}
pub fn take_back() -> bool {
    BACK.swap(false, std::sync::atomic::Ordering::AcqRel)
}
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_davewagner_puzzgen_MainActivity_nativeBack(
    _: jni::JNIEnv,
    _: jni::objects::JClass,
) {
    BACK.store(true, std::sync::atomic::Ordering::Release);
}
pub fn background() {
    #[cfg(target_os = "android")]
    {
        let _ = java("backgroundTask");
    }
}
pub fn safe_area() -> [f32; 4] {
    *SAFE_AREA.lock().unwrap_or_else(|e| e.into_inner())
}
pub fn trace_input() -> bool {
    #[cfg(target_os = "android")]
    {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ENABLED.get_or_init(|| java("debugInput") == "true")
    }
    #[cfg(not(target_os = "android"))]
    {
        false
    }
}
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_davewagner_puzzgen_MainActivity_nativeSafeArea(
    _: jni::JNIEnv,
    _: jni::objects::JClass,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) {
    *SAFE_AREA.lock().unwrap_or_else(|e| e.into_inner()) =
        [left, top, right, bottom].map(|v| v.max(0) as f32);
}
#[cfg(target_os = "android")]
fn java(method: &str) -> String {
    use jni::{
        JavaVM,
        objects::{JObject, JString},
    };
    let Some(context) = bevy::android::ANDROID_APP.get() else {
        return String::new();
    };
    let Ok(vm) = (unsafe { JavaVM::from_raw(context.vm_as_ptr().cast()) }) else {
        return String::new();
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return String::new();
    };
    // Android's process context is an Application, not our NativeActivity.
    // Borrow the activity global reference owned by android-activity.
    let activity = unsafe { JObject::from_raw(context.activity_as_ptr().cast()) };
    let result = env
        .call_method(&activity, method, "()Ljava/lang/String;", &[])
        .ok()
        .and_then(|v| v.l().ok())
        .and_then(|s| env.get_string(&JString::from(s)).ok().map(String::from))
        .unwrap_or_default();
    std::mem::forget(activity);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    result
}
fn directory() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::from(java("dataDirectory"))
    }
    #[cfg(not(target_os = "android"))]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("PuzzgenPocket")
    }
}
pub fn load() -> Option<Saved> {
    let bytes = std::fs::read(directory().join("progress-v2.json")).ok()?;
    if bytes.len() > 2_000_000 {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}
pub fn save(value: &Saved) {
    let dir = directory();
    if dir.as_os_str().is_empty() {
        return;
    }
    if std::fs::create_dir_all(&dir).is_ok()
        && let Ok(bytes) = serde_json::to_vec(value)
    {
        let temp = dir.join("progress-v2.tmp");
        if std::fs::write(&temp, bytes).is_ok() {
            let dest = dir.join("progress-v2.json");
            #[cfg(target_os = "windows")]
            let _ = std::fs::remove_file(&dest);
            let _ = std::fs::rename(temp, dest);
        }
    }
}
pub fn api_key() -> String {
    #[cfg(target_os = "android")]
    {
        java("apiKey")
    }
    #[cfg(not(target_os = "android"))]
    {
        std::env::var("PUZZGEN_NATIVE_API_KEY").unwrap_or_default()
    }
}
pub fn guest_token() -> String {
    #[cfg(target_os = "android")]
    {
        java("guestToken")
    }
    #[cfg(not(target_os = "android"))]
    {
        String::new()
    }
}
pub fn api_origin() -> String {
    #[cfg(target_os = "android")]
    {
        java("apiOrigin")
    }
    #[cfg(not(target_os = "android"))]
    {
        std::env::var("PUZZGEN_NATIVE_API_ORIGIN")
            .unwrap_or_else(|_| "https://puzzlegen.davewagner.org".into())
    }
}
pub fn license_dialog() {
    #[cfg(target_os = "android")]
    {
        let _ = java("licenseDialog");
    }
    #[cfg(not(target_os = "android"))]
    {
        bevy::log::info!("Original notices: apps/android/game/public/licenses");
    }
}
