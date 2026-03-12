use std::path::PathBuf;

#[allow(unused)]
/// 获取 Windows 平台的 VRChat 应用数据路径
fn get_windows_vrchat_app_data_location() -> Option<PathBuf> {
    dirs::data_local_dir().map(|local_app_data| {
        let local_str = local_app_data.to_string_lossy();
        let local_low_str = local_str.replace("Local", "LocalLow");
        PathBuf::from(local_low_str).join("VRChat").join("VRChat")
    })
}

/// 检查给定路径是否为有效的 Steam 路径
fn is_valid_steam_path(path: &PathBuf) -> bool {
    if !path.exists() {
        return false;
    }
    let steamapps = path.join("steamapps");
    steamapps.exists()
}

/// 读取 Steam libraryfolders.vdf 文件并查找指定 appId 的库路径
fn get_steam_library_with_app_id(steam_path: &PathBuf, app_id: &str) -> Option<PathBuf> {
    let library_folders_vdf = steam_path.join("config").join("libraryfolders.vdf");
    if !library_folders_vdf.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&library_folders_vdf).ok()?;

    for line in content.lines() {
        let line = line.trim();
        if line.contains("\"path\"") {
            // 提取路径
            if let Some(start) = line.find("\"path\"") {
                let remaining = &line[start + 6..];
                // 找到引号包围的路径
                if let Some(quote_start) = remaining.find('"') {
                    if let Some(quote_end) = remaining[quote_start + 1..].find('"') {
                        let lib_path = &remaining[quote_start + 1..quote_start + 1 + quote_end];
                        let full_path = PathBuf::from(lib_path).join("steamapps");
                        if full_path.join("common").exists() {
                            // 检查是否包含 VRChat
                            let compat_path = full_path.join("compatdata").join(app_id);
                            if compat_path.exists() {
                                return Some(PathBuf::from(lib_path));
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

#[allow(unused)]
/// 获取 Linux 平台的 VRChat 应用数据路径
fn get_linux_vrchat_app_data_location() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let vrchat_app_id = "438100";

    // 尝试多个可能的 Steam 路径
    let mut steam_path = home.join(".local/share/steam");
    let steam_userdata_path = home.join(".steam/steam/userdata");

    // 检查 userdata 路径
    if steam_userdata_path.exists() {
        // 优先使用 userdata 路径
        steam_path = steam_userdata_path.parent()?.to_path_buf();
    }

    // 检查 Flatpak Steam
    let flatpak_steam_path = home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam");
    if !is_valid_steam_path(&steam_path) && is_valid_steam_path(&flatpak_steam_path) {
        steam_path = flatpak_steam_path;
    }

    // 检查 legacy Steam 路径
    let legacy_steam_path = home.join(".steam/steam");
    if !is_valid_steam_path(&steam_path) && is_valid_steam_path(&legacy_steam_path) {
        steam_path = legacy_steam_path;
    }

    if !is_valid_steam_path(&steam_path) {
        return None;
    }

    // 尝试从 libraryfolders.vdf 获取 VRChat 库路径
    let vrc_library_path = get_steam_library_with_app_id(&steam_path, vrchat_app_id)
        .unwrap_or(steam_path);

    // 构建 Proton/Steam Play 路径
    let vrc_prefix_path = vrc_library_path
        .join("steamapps")
        .join("compatdata")
        .join(vrchat_app_id)
        .join("pfx");

    let app_data = vrc_prefix_path
        .join("drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat");

    Some(app_data)
}

pub fn get_vrchat_app_data_location() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        get_windows_vrchat_app_data_location()
    }

    #[cfg(target_os = "linux")]
    {
        get_linux_vrchat_app_data_location()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        None
    }
}
