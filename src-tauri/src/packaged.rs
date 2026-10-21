//! The Microsoft Store build runs as an MSIX package. Windows then quietly
//! redirects what the app writes below %LOCALAPPDATA% into the package's own
//! folder. The app keeps seeing the usual paths, Explorer doesn't – so a folder
//! we open for the user has to be translated first.

use std::path::{Path, PathBuf};

/// The folder Explorer has to open to show the user `path`.
pub fn visible_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    if let (Some(family), Some(local)) = (package_family_name(), std::env::var_os("LOCALAPPDATA"))
        && let Some(moved) = redirected(&path, Path::new(&local), &family)
        && moved.exists()
    {
        return moved;
    }
    path
}

/// Where Windows keeps `path` for the package `family`, if it moves it at all.
#[cfg_attr(not(windows), allow(dead_code))]
fn redirected(path: &Path, local_app_data: &Path, family: &str) -> Option<PathBuf> {
    let rest = path.strip_prefix(local_app_data).ok()?;
    Some(local_app_data.join("Packages").join(family).join("LocalCache").join("Local").join(rest))
}

/// The package family name, or `None` outside a package (the normal installer).
#[cfg(windows)]
fn package_family_name() -> Option<String> {
    use windows_sys::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows_sys::Win32::Storage::Packaging::Appx::GetCurrentPackageFamilyName;

    let mut len = 0u32;
    // SAFETY: a null buffer with length 0 only asks for the needed length.
    if unsafe { GetCurrentPackageFamilyName(&mut len, std::ptr::null_mut()) } != ERROR_INSUFFICIENT_BUFFER {
        return None;
    }
    let mut name = vec![0u16; len as usize];
    // SAFETY: `name` holds the `len` UTF-16 units the first call asked for.
    if unsafe { GetCurrentPackageFamilyName(&mut len, name.as_mut_ptr()) } != 0 {
        return None;
    }
    name.truncate(len.saturating_sub(1) as usize); // drop the trailing NUL
    Some(String::from_utf16_lossy(&name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local() -> PathBuf {
        PathBuf::from("Users").join("anna").join("AppData").join("Local")
    }

    #[test]
    fn app_data_moves_into_the_package_folder() {
        let logs = local().join("io.github.zdkderpphb.wortlaut").join("logs");
        let expected = local()
            .join("Packages")
            .join("Wortlaut_abc123")
            .join("LocalCache")
            .join("Local")
            .join("io.github.zdkderpphb.wortlaut")
            .join("logs");
        assert_eq!(redirected(&logs, &local(), "Wortlaut_abc123"), Some(expected));
    }

    #[test]
    fn other_folders_stay_where_they_are() {
        let documents = PathBuf::from("Users").join("anna").join("Documents");
        assert_eq!(redirected(&documents, &local(), "Wortlaut_abc123"), None);
    }
}
