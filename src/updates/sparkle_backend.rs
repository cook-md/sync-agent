// Allow deprecated warnings from cocoa crate - we plan to migrate to objc2 in the future
// Also allow unexpected_cfgs from objc crate macros
#![allow(deprecated)]
#![allow(unexpected_cfgs)]

#[cfg(target_os = "macos")]
use anyhow::Result;
#[cfg(target_os = "macos")]
use log::{info, warn};

#[cfg(target_os = "macos")]
use cocoa::base::{id, nil};
#[cfg(target_os = "macos")]
use cocoa::foundation::NSAutoreleasePool;
#[cfg(target_os = "macos")]
use objc::runtime::Class;
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};

#[cfg(target_os = "macos")]
pub struct SparkleBackend {
    updater: Option<id>,
}

#[cfg(target_os = "macos")]
impl SparkleBackend {
    pub fn new() -> Result<Self> {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            // Try to load SPUStandardUpdaterController class
            let controller_class = Class::get("SPUStandardUpdaterController");

            if controller_class.is_none() {
                warn!("Sparkle.framework not found - SPUStandardUpdaterController class not available");
                warn!("Sparkle backend requires the app to be bundled with Sparkle.framework");
                warn!("For development, the axoupdater backend will be used instead");
                return Ok(Self { updater: None });
            }

            let controller_class = controller_class.unwrap();

            // Create updater controller
            let updater_controller: id = msg_send![controller_class, alloc];
            let updater_controller: id = msg_send![updater_controller,
                initWithStartingUpdater: true
                updaterDelegate: nil
                userDriverDelegate: nil
            ];

            if updater_controller == nil {
                anyhow::bail!("Failed to initialize Sparkle updater controller");
            }

            info!("Sparkle updater initialized successfully");

            Ok(Self {
                updater: Some(updater_controller),
            })
        }
    }

    pub async fn check(&self) -> Result<bool> {
        if self.updater.is_none() {
            warn!("Sparkle not available, cannot check for updates");
            return Ok(false);
        }

        info!("Checking for updates using Sparkle...");

        unsafe {
            let updater_controller = self.updater.unwrap();

            // Get the updater object from the controller
            let updater: id = msg_send![updater_controller, updater];

            if updater != nil {
                // Check for updates in background (non-intrusive)
                let _: () = msg_send![updater, checkForUpdatesInBackground];
                info!("Sparkle background update check initiated");
                Ok(true)
            } else {
                warn!("Sparkle updater object is nil");
                Ok(false)
            }
        }
    }

    pub async fn install(&self) -> Result<()> {
        if self.updater.is_none() {
            warn!("Sparkle not available, cannot install updates");
            return Ok(());
        }

        info!("Triggering Sparkle update UI...");

        unsafe {
            let updater_controller = self.updater.unwrap();

            // Get the updater object from the controller
            let updater: id = msg_send![updater_controller, updater];

            if updater != nil {
                // Show update UI (user-initiated check)
                let _: () = msg_send![updater, checkForUpdates: nil];
                info!("Sparkle UI update check initiated");
            } else {
                warn!("Sparkle updater object is nil");
            }
        }

        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl Drop for SparkleBackend {
    fn drop(&mut self) {
        if let Some(updater) = self.updater {
            unsafe {
                let _: () = msg_send![updater, release];
            }
        }
    }
}

#[cfg(target_os = "macos")]
unsafe impl Send for SparkleBackend {}

#[cfg(target_os = "macos")]
unsafe impl Sync for SparkleBackend {}

// Stub implementation for non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub struct SparkleBackend;

#[cfg(not(target_os = "macos"))]
impl SparkleBackend {
    #[allow(dead_code)]
    pub fn new() -> anyhow::Result<Self> {
        anyhow::bail!("Sparkle backend is only available on macOS")
    }

    #[allow(dead_code)]
    pub async fn check(&self) -> anyhow::Result<bool> {
        Ok(false)
    }

    #[allow(dead_code)]
    pub async fn install(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
