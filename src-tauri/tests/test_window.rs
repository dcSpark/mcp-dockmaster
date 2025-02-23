// Integration test for window visibility
use tauri::Manager;

#[test]
fn test_window_visibility() {
    let context = tauri::generate_context!();
    let builder = tauri::Builder::default();
    assert!(builder.setup(|app| {
        let window = app.get_webview_window("main").unwrap();
        assert!(window.is_visible().unwrap());
        Ok(())
    }).is_ok());
}
