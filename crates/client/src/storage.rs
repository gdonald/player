use leptos::prelude::window;

/// Reads a value this browser kept for the app. Private windows and blocked
/// storage read as missing.
pub fn get(key: &str) -> Option<String> {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

/// Keeps a value in this browser. Where storage is unavailable the value is
/// dropped.
pub fn set(key: &str, value: &str) {
    if let Some(storage) = window().local_storage().ok().flatten() {
        storage.set_item(key, value).ok();
    }
}
