use crate::sync::SyncStatus;
use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};

pub struct TrayMenu {
    pub menu: Menu,
    pub status_item: MenuItem,
    pub folder_item: MenuItem,
    pub sync_toggle: MenuItem,
    pub user_item: MenuItem,
    pub login_logout: MenuItem,
    pub auto_start: CheckMenuItem,
    pub set_folder: MenuItem,
    pub open_folder: MenuItem,
    pub open_web: MenuItem,
    pub check_updates: MenuItem,
    pub about: MenuItem,
    pub quit: MenuItem,
}

impl Default for TrayMenu {
    fn default() -> Self {
        Self::new(true)
    }
}

impl TrayMenu {
    pub fn new(auto_start_enabled: bool) -> Self {
        let menu = Menu::new();

        // Status and folder info
        let status_item = MenuItem::new("Status: Starting 🟠", false, None);
        let folder_item = MenuItem::new("Not configured", false, None);

        // Sync control
        let sync_toggle = MenuItem::new("Pause Sync", true, None);

        // User account
        let user_item = MenuItem::new("Not logged in", false, None);
        let login_logout = MenuItem::new("Login", true, None);

        // Settings
        let auto_start =
            CheckMenuItem::new("Start on system startup", true, auto_start_enabled, None);

        // Actions
        let set_folder = MenuItem::new("Set recipes folder...", true, None);
        let open_folder = MenuItem::new("Open recipes folder", true, None);
        let open_web = MenuItem::new("Open cook.md", true, None);
        let check_updates = MenuItem::new("Check for updates...", true, None);
        let about = MenuItem::new("About Cook Sync", true, None);
        let quit = MenuItem::new("Quit", true, None);

        // Build menu
        menu.append(&status_item).unwrap();
        menu.append(&sync_toggle).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&user_item).unwrap();
        menu.append(&login_logout).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&folder_item).unwrap();
        menu.append(&set_folder).unwrap();
        menu.append(&open_folder).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&open_web).unwrap();
        menu.append(&auto_start).unwrap();
        menu.append(&check_updates).unwrap();
        menu.append(&about).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&quit).unwrap();

        TrayMenu {
            menu,
            status_item,
            folder_item,
            sync_toggle,
            user_item,
            login_logout,
            auto_start,
            set_folder,
            open_folder,
            open_web,
            check_updates,
            about,
            quit,
        }
    }

    pub fn update_status(&self, status: SyncStatus, error_msg: Option<&str>) {
        let (indicator, text) = match status {
            SyncStatus::Starting => ("🟠", "Starting".to_string()),
            SyncStatus::Idle => ("🟢", "Idle".to_string()),
            SyncStatus::Syncing => ("🟢", "Syncing".to_string()),
            SyncStatus::Paused => ("🟠", "Paused".to_string()),
            SyncStatus::Offline => ("🟠", "Offline".to_string()),
            SyncStatus::Error => ("🔴", error_msg.unwrap_or("Error").to_string()),
        };
        self.status_item
            .set_text(format!("Status: {text} {indicator}"));
    }

    pub fn update_user(&self, email: Option<&str>) {
        match email {
            Some(email) => {
                self.user_item.set_text(email);
                self.login_logout.set_text("Logout");
            }
            None => {
                self.user_item.set_text("Not logged in");
                self.login_logout.set_text("Login");
            }
        }
    }

    pub fn update_sync_toggle(&self, is_paused: bool) {
        if is_paused {
            self.sync_toggle.set_text("Resume Sync");
        } else {
            self.sync_toggle.set_text("Pause Sync");
        }
    }

    #[allow(dead_code)]
    pub fn set_auto_start(&self, enabled: bool) {
        self.auto_start.set_checked(enabled);
    }

    pub fn get_menu_id(&self, item: &MenuItem) -> MenuId {
        item.id().clone()
    }

    pub fn get_check_menu_id(&self, item: &CheckMenuItem) -> MenuId {
        item.id().clone()
    }

    pub fn update_folder(&self, path: Option<&std::path::Path>) {
        match path {
            Some(p) => {
                // Convert to string and use tilde for home directory
                let path_str = p.to_string_lossy();
                let display_path = if let Some(home_dir) = dirs::home_dir() {
                    let home_str = home_dir.to_string_lossy();
                    if path_str.starts_with(&home_str as &str) {
                        path_str.replacen(&home_str as &str, "~", 1)
                    } else {
                        path_str.to_string()
                    }
                } else {
                    path_str.to_string()
                };
                self.folder_item.set_text(&display_path);
            }
            None => self.folder_item.set_text("Not configured"),
        }
    }
}
