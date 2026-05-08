use anyhow::{anyhow, Result};
use std::{
    env,
    ffi::OsString,
    path::PathBuf,
    process::{Command, Stdio},
};

const FFMPEG_ENV_VAR: &str = "KOOM_FFMPEG_PATH";

fn path_env_entries() -> impl Iterator<Item = PathBuf> {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
}

fn executable_names() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["ffmpeg.exe", "ffmpeg"]
    } else {
        &["ffmpeg"]
    }
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = env::var(FFMPEG_ENV_VAR) {
        candidates.push(PathBuf::from(path));
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            for name in executable_names() {
                candidates.push(exe_dir.join(name));
            }
        }
    }

    for dir in path_env_entries() {
        for name in executable_names() {
            candidates.push(dir.join(name));
        }
    }

    candidates
}

fn command_available(program: &OsString) -> bool {
    Command::new(program)
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn ffmpeg_program() -> OsString {
    if let Some(path) = candidate_paths().into_iter().find(|path| path.is_file()) {
        return path.into_os_string();
    }

    OsString::from(if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    })
}

pub fn ensure_ffmpeg_available() -> Result<()> {
    let program = ffmpeg_program();

    if command_available(&program) {
        return Ok(());
    }

    Err(anyhow!(
        "FFmpeg is required to record video but was not found. Install FFmpeg and add it to PATH, or set {} to the full path of ffmpeg.exe.",
        FFMPEG_ENV_VAR
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffmpeg_program_has_a_fallback_name() {
        let program = ffmpeg_program();
        assert!(!program.is_empty());
    }
}
