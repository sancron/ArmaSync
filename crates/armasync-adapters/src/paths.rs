// SPDX-License-Identifier: AGPL-3.0-only
//! Where ArmaSync keeps its files (docs/migration/ARCHITECTURE.md, "Build- und Plattformgrenzen").
//!
//! Resolution is a pure function of the platform, the mode and an environment lookup, so every platform layout is
//! testable on any host. Nothing here creates, moves or deletes directories except [`DataRoots::ensure_writable`],
//! which the caller invokes explicitly (for example when the user chooses portable mode).

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use armasync_core::{Error, ErrorKind};

use crate::Platform;
use crate::fsutil::io_error;

/// File name of the settings document inside the config root.
pub const SETTINGS_FILE: &str = "settings.toml";
/// File name of the library document (repositories and profiles) inside the config root.
pub const LIBRARY_FILE: &str = "library.json";

/// How the roots are chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathMode {
    /// The platform's user directories (never a legacy location).
    Standard,
    /// Everything below one directory chosen by the user (`config`, `data`, `cache` subdirectories).
    Portable(PathBuf),
}

/// The three roots: user configuration, persistent data and disposable cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRoots {
    config: PathBuf,
    data: PathBuf,
    cache: PathBuf,
    portable: bool,
}

impl DataRoots {
    /// Resolves the roots for `platform`. `env` returns the value of an environment variable, if set.
    ///
    /// * Windows: config `%APPDATA%\Lynxnode.it\ArmaSync`, data `%LOCALAPPDATA%\Lynxnode.it\ArmaSync`, cache
    ///   `...\ArmaSync\cache`; without `APPDATA`/`LOCALAPPDATA` the profile directory (`USERPROFILE`) is used.
    /// * Linux: `$XDG_CONFIG_HOME`, `$XDG_DATA_HOME`, `$XDG_CACHE_HOME` (only absolute values, as the XDG
    ///   specification requires) or `~/.config`, `~/.local/share`, `~/.cache`, each plus `lynxnode/armasync`.
    /// * Portable: `<root>/config`, `<root>/data`, `<root>/cache`.
    ///
    /// # Errors
    /// [`ErrorKind::Config`] if the needed environment is missing or not absolute;
    /// [`ErrorKind::Unsupported`] on platforms other than Windows and Linux (no implicit commitment).
    pub fn resolve(
        platform: Platform,
        mode: &PathMode,
        env: &dyn Fn(&str) -> Option<OsString>,
    ) -> Result<Self, Error> {
        if let PathMode::Portable(root) = mode {
            if root.as_os_str().is_empty() {
                return Err(Error::new(
                    ErrorKind::Config,
                    "the portable directory is empty",
                ));
            }
            return Ok(Self {
                config: root.join("config"),
                data: root.join("data"),
                cache: root.join("cache"),
                portable: true,
            });
        }
        match platform {
            Platform::Windows => {
                let profile = || absolute_var(env, "USERPROFILE");
                let roaming = absolute_var(env, "APPDATA")
                    .or_else(|| profile().map(|p| p.join("AppData").join("Roaming")));
                let local = absolute_var(env, "LOCALAPPDATA")
                    .or_else(|| profile().map(|p| p.join("AppData").join("Local")));
                let (Some(roaming), Some(local)) = (roaming, local) else {
                    return Err(Error::new(
                        ErrorKind::Config,
                        "APPDATA/LOCALAPPDATA (or USERPROFILE) are not set to an absolute path",
                    ));
                };
                let data = local.join("Lynxnode.it").join("ArmaSync");
                Ok(Self {
                    config: roaming.join("Lynxnode.it").join("ArmaSync"),
                    cache: data.join("cache"),
                    data,
                    portable: false,
                })
            }
            Platform::Linux => {
                let home = absolute_var(env, "HOME");
                let xdg = |variable: &str, fallback: &[&str]| -> Option<PathBuf> {
                    let base = absolute_var(env, variable).or_else(|| {
                        home.as_ref().map(|home| {
                            fallback
                                .iter()
                                .fold(home.clone(), |path, part| path.join(part))
                        })
                    })?;
                    Some(base.join("lynxnode").join("armasync"))
                };
                let (Some(config), Some(data), Some(cache)) = (
                    xdg("XDG_CONFIG_HOME", &[".config"]),
                    xdg("XDG_DATA_HOME", &[".local", "share"]),
                    xdg("XDG_CACHE_HOME", &[".cache"]),
                ) else {
                    return Err(Error::new(
                        ErrorKind::Config,
                        "HOME is not set to an absolute path and no XDG directory is configured",
                    ));
                };
                Ok(Self {
                    config,
                    data,
                    cache,
                    portable: false,
                })
            }
            Platform::Other => Err(Error::new(
                ErrorKind::Unsupported,
                "data directories are only defined for Windows and Linux",
            )),
        }
    }

    /// Resolves the roots for the running process and platform.
    ///
    /// # Errors
    /// See [`DataRoots::resolve`].
    pub fn for_current_process(mode: &PathMode) -> Result<Self, Error> {
        Self::resolve(Platform::current(), mode, &|name| std::env::var_os(name))
    }

    /// User configuration root (settings, library).
    #[must_use]
    pub fn config(&self) -> &Path {
        &self.config
    }

    /// Persistent data root.
    #[must_use]
    pub fn data(&self) -> &Path {
        &self.data
    }

    /// Disposable cache root.
    #[must_use]
    pub fn cache(&self) -> &Path {
        &self.cache
    }

    /// Whether the roots were chosen explicitly as portable.
    #[must_use]
    pub const fn is_portable(&self) -> bool {
        self.portable
    }

    /// Full path of the settings document.
    #[must_use]
    pub fn settings_file(&self) -> PathBuf {
        self.config.join(SETTINGS_FILE)
    }

    /// Full path of the library document.
    #[must_use]
    pub fn library_file(&self) -> PathBuf {
        self.config.join(LIBRARY_FILE)
    }

    /// Creates the three roots if needed and proves each is writable by creating and removing a probe file.
    ///
    /// Call this explicitly (portable mode, first start); the resolution itself never touches the disk.
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`] naming the directory that cannot be written.
    pub fn ensure_writable(&self) -> Result<(), Error> {
        for directory in [&self.config, &self.data, &self.cache] {
            fs::create_dir_all(directory).map_err(|e| io_error("create", directory, &e))?;
            let probe = directory.join(format!(".write-test-{}", std::process::id()));
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&probe)
                .map_err(|e| io_error("write to", directory, &e))?;
            file.write_all(b"ok")
                .map_err(|e| io_error("write to", directory, &e))?;
            drop(file);
            fs::remove_file(&probe).map_err(|e| io_error("clean up in", directory, &e))?;
        }
        Ok(())
    }
}

/// An environment variable that holds a non-empty absolute path.
fn absolute_var(env: &dyn Fn(&str) -> Option<OsString>, name: &str) -> Option<PathBuf> {
    let value = env(name)?;
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    path.is_absolute().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::{DataRoots, PathMode};
    use crate::Platform;
    use armasync_core::ErrorKind;
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), OsString::from(*v)))
            .collect();
        move |name| map.get(name).cloned()
    }

    /// An absolute path for the host (test inputs must be absolute on the machine running the test).
    fn abs(parts: &[&str]) -> PathBuf {
        let mut path = if cfg!(windows) {
            PathBuf::from("C:\\")
        } else {
            PathBuf::from("/")
        };
        for part in parts {
            path.push(part);
        }
        path
    }

    fn text(path: &std::path::Path) -> String {
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn windows_layout_uses_appdata_and_localappdata() {
        let roaming = text(&abs(&["Users", "u", "AppData", "Roaming"]));
        let local = text(&abs(&["Users", "u", "AppData", "Local"]));
        let roots = DataRoots::resolve(
            Platform::Windows,
            &PathMode::Standard,
            &env(&[("APPDATA", &roaming), ("LOCALAPPDATA", &local)]),
        )
        .unwrap();
        assert_eq!(
            roots.config(),
            abs(&["Users", "u", "AppData", "Roaming"])
                .join("Lynxnode.it")
                .join("ArmaSync")
        );
        let data = abs(&["Users", "u", "AppData", "Local"])
            .join("Lynxnode.it")
            .join("ArmaSync");
        assert_eq!(roots.data(), data);
        assert_eq!(roots.cache(), data.join("cache"));
        assert!(!roots.is_portable());
        assert_eq!(roots.library_file(), roots.config().join("library.json"));
        assert_eq!(roots.settings_file(), roots.config().join("settings.toml"));
    }

    #[test]
    fn windows_falls_back_to_the_profile_directory() {
        let profile = text(&abs(&["Users", "u"]));
        let roots = DataRoots::resolve(
            Platform::Windows,
            &PathMode::Standard,
            &env(&[("USERPROFILE", &profile), ("APPDATA", "relative\\path")]),
        )
        .unwrap();
        assert_eq!(
            roots.config(),
            abs(&["Users", "u"])
                .join("AppData")
                .join("Roaming")
                .join("Lynxnode.it")
                .join("ArmaSync")
        );
        let missing = DataRoots::resolve(Platform::Windows, &PathMode::Standard, &env(&[]));
        assert_eq!(missing.unwrap_err().kind(), ErrorKind::Config);
    }

    #[test]
    fn linux_layout_prefers_absolute_xdg_values_and_falls_back_to_home() {
        let home = text(&abs(&["home", "u"]));
        let xdg_config = text(&abs(&["cfg"]));
        let roots = DataRoots::resolve(
            Platform::Linux,
            &PathMode::Standard,
            &env(&[
                ("HOME", &home),
                ("XDG_CONFIG_HOME", &xdg_config),
                ("XDG_DATA_HOME", "relative/ignored"),
                ("XDG_CACHE_HOME", ""),
            ]),
        )
        .unwrap();
        let app = |base: PathBuf| base.join("lynxnode").join("armasync");
        assert_eq!(roots.config(), app(abs(&["cfg"])));
        assert_eq!(
            roots.data(),
            app(abs(&["home", "u"]).join(".local").join("share"))
        );
        assert_eq!(roots.cache(), app(abs(&["home", "u"]).join(".cache")));
    }

    #[test]
    fn linux_without_home_is_a_config_error() {
        let error = DataRoots::resolve(
            Platform::Linux,
            &PathMode::Standard,
            &env(&[("HOME", "rel")]),
        )
        .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Config);
    }

    #[test]
    fn other_platforms_are_not_implicitly_supported() {
        let error =
            DataRoots::resolve(Platform::Other, &PathMode::Standard, &env(&[])).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Unsupported);
    }

    #[test]
    fn portable_mode_uses_one_root_regardless_of_platform() {
        let root = abs(&["tools", "armasync"]);
        for platform in [Platform::Windows, Platform::Linux, Platform::Other] {
            let roots =
                DataRoots::resolve(platform, &PathMode::Portable(root.clone()), &env(&[])).unwrap();
            assert!(roots.is_portable());
            assert_eq!(roots.config(), root.join("config"));
            assert_eq!(roots.data(), root.join("data"));
            assert_eq!(roots.cache(), root.join("cache"));
        }
        let empty = DataRoots::resolve(
            Platform::Linux,
            &PathMode::Portable(PathBuf::new()),
            &env(&[]),
        );
        assert_eq!(empty.unwrap_err().kind(), ErrorKind::Config);
    }

    #[test]
    fn ensure_writable_creates_roots_and_leaves_no_probe_files() {
        let base = std::env::temp_dir().join(format!("armasync-paths-{}", std::process::id()));
        let roots = DataRoots::resolve(
            Platform::Linux,
            &PathMode::Portable(base.clone()),
            &env(&[]),
        )
        .unwrap();
        roots.ensure_writable().unwrap();
        for dir in [roots.config(), roots.data(), roots.cache()] {
            assert!(dir.is_dir());
            assert_eq!(std::fs::read_dir(dir).unwrap().count(), 0);
        }
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn ensure_writable_reports_the_unwritable_directory() {
        let base = std::env::temp_dir().join(format!("armasync-paths-file-{}", std::process::id()));
        std::fs::write(&base, b"a file, not a directory").unwrap();
        let roots = DataRoots::resolve(
            Platform::Linux,
            &PathMode::Portable(base.clone()),
            &env(&[]),
        )
        .unwrap();
        let error = roots.ensure_writable().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FileSystem);
        std::fs::remove_file(base).unwrap();
    }
}
