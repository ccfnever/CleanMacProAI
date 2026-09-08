use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn move_to_trash(path: &Path) -> Result<(), String> {
    let mut context = trash::TrashContext::default();

    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        context.set_delete_method(DeleteMethod::NsFileManager);
    }

    context.delete(path).map_err(|error| error.to_string())
}

pub fn move_application_to_trash(path: &Path) -> Result<(), String> {
    match move_to_trash(path) {
        Ok(()) => Ok(()),
        Err(original_error) => {
            #[cfg(target_os = "macos")]
            {
                move_application_to_trash_with_authorization(path).map_err(|error| {
                    format!("{original_error}; administrator authorization failed: {error}")
                })
            }

            #[cfg(not(target_os = "macos"))]
            {
                Err(original_error)
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn move_application_to_trash_with_authorization(path: &Path) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Unable to resolve application path: {error}"))?;
    if canonical.parent() != Some(Path::new("/Applications"))
        || canonical.extension().and_then(|value| value.to_str()) != Some("app")
    {
        return Err("Administrator authorization is only allowed for /Applications/*.app".to_string());
    }

    let trash_dir = dirs::home_dir()
        .ok_or_else(|| "Unable to locate the current user's home directory".to_string())?
        .join(".Trash");
    fs::create_dir_all(&trash_dir)
        .map_err(|error| format!("Unable to access the current user's Trash: {error}"))?;
    let destination = available_trash_destination(&trash_dir, &canonical)?;

    let script = r#"
on run argv
    set sourcePath to item 1 of argv
    set destinationPath to item 2 of argv
    do shell script "/bin/mv " & quoted form of sourcePath & " " & quoted form of destinationPath with administrator privileges
end run
"#;
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", script, "--"])
        .arg(&canonical)
        .arg(&destination)
        .output()
        .map_err(|error| format!("Unable to request administrator authorization: {error}"))?;

    if output.status.success() && !canonical.exists() && destination.exists() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.contains("(-128)") || stderr.to_lowercase().contains("user canceled") {
        return Err("管理员授权已取消".to_string());
    }
    if stderr.is_empty() {
        Err("The authorized move did not complete".to_string())
    } else {
        Err(stderr)
    }
}

#[cfg(target_os = "macos")]
fn available_trash_destination(trash_dir: &Path, source: &Path) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .ok_or_else(|| "Application path has no file name".to_string())?;
    let direct = trash_dir.join(file_name);
    if !direct.exists() {
        return Ok(direct);
    }

    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Application name is not valid UTF-8".to_string())?;
    let extension = source.extension().and_then(|value| value.to_str()).unwrap_or("");
    for index in 2..=10_000 {
        let candidate = trash_dir.join(format!("{stem} {index}.{extension}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("Unable to choose an unused destination in the Trash".to_string())
}
