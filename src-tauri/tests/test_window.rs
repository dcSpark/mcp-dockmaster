// Integration test for window visibility
use tauri::Manager;

#[test]
fn test_window_visibility() {
    let app = tauri::test::mock_builder()
        .build()
        .expect("Failed to build mock app");
    
    let window = app.get_window("main").expect("Failed to get main window");
    assert!(window.is_visible().expect("Failed to check window visibility"));
}
