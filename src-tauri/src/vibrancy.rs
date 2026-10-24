use tauri::WebviewWindow;

/// 开启 Windows Acrylic：窗口透出后面的实时桌面画面，
/// 模糊由系统合成器完成，前端只保留低透明度的主题染色。
pub fn enable(window: &WebviewWindow) -> Result<(), String> {
    window_vibrancy::apply_acrylic(window, None)
        .map_err(|e| format!("当前系统不支持毛玻璃透出：{e}"))
}

/// 关闭系统毛玻璃，恢复实心窗口。
pub fn disable(window: &WebviewWindow) -> Result<(), String> {
    window_vibrancy::clear_acrylic(window).map_err(|e| format!("关闭毛玻璃失败：{e}"))
}

/// 按配置应用；返回 Err 表示系统不支持，调用方应把配置回滚为关闭。
pub fn apply(window: &WebviewWindow, enabled: bool) -> Result<(), String> {
    if enabled {
        enable(window)
    } else {
        disable(window)
    }
}
