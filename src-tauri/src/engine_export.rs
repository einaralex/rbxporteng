//! Prepares Engine DJ data after a verified Pioneer export.
//! The caller holds its export reservation throughout; this stage deliberately
//! cannot be cancelled because the analyzer owns children and publication.
use std::collections::VecDeque;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn prepare(destination: &Path, executable: &str, replace_existing: bool) -> Result<(), String> {
    if executable.trim().is_empty() {
        preflight(destination, executable, replace_existing)?;
        #[cfg(target_os = "macos")]
        {
            rekordbox_to_engine::prepare_usb_with_options(destination, replace_existing)
                .map(|_| ())
                .map_err(|e| format!("Engine DJ preparation failed: {e:#}"))
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err("Engine DJ preparation currently requires macOS.".to_owned())
        }
    } else {
        run(destination, executable)
    }
}

/// Check known blockers before rbxport writes anything to the USB.
pub fn preflight(destination: &Path, executable: &str, replace_existing: bool) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("Engine DJ preparation currently requires macOS.".to_owned());
    }
    let target = destination.canonicalize().map_err(|e| format!("USB is not accessible: {e}"))?;
    if executable.trim().is_empty() {
        let analyzer = Path::new("/Applications/Engine DJ.app/Contents/MacOS/OfflineAnalyzer");
        if !analyzer.is_file() {
            return Err(format!("Engine DJ is not installed or its OfflineAnalyzer is missing at {}. Install Engine DJ before exporting, or turn off Engine DJ preparation in USB Export preferences.", analyzer.display()));
        }
        #[cfg(target_os = "macos")]
        rekordbox_to_engine::engine::validate_for_export(&target, replace_existing)
            .map_err(|e| format!("Engine Library cannot be refreshed: {e:#}"))?;
    } else {
        // External helpers may not support refreshes. Keep their fresh-USB
        // contract until an override explicitly advertises that capability.
        match target.join("Engine Library").symlink_metadata() {
            Ok(_) => return Err("Engine Library already exists. The external Engine helper requires a fresh USB export.".to_owned()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(format!("Cannot inspect the Engine Library: {e}")),
        }
    }
    if !executable.trim().is_empty() && executable.contains('/') && !Path::new(executable).is_file() {
        return Err(format!("External Engine helper was not found at {executable}. Check the executable path in USB Export preferences."));
    }
    Ok(())
}

/// `None` means the user declined replacement. The confirmation is requested
/// only for an existing directory that the bundled helper would otherwise
/// refuse; missing analyzers and unsafe filesystem entries remain errors.
pub fn preflight_with_replacement(
    destination: &Path,
    executable: &str,
    confirm: impl FnOnce(&str) -> bool,
) -> Result<Option<bool>, String> {
    match preflight(destination, executable, false) {
        Ok(()) => Ok(Some(false)),
        Err(error) => {
            if !executable.trim().is_empty() || !error.starts_with("Engine Library cannot be refreshed:") {
                return Err(error);
            }
            #[cfg(target_os = "macos")]
            {
                let target = destination.canonicalize().map_err(|e| format!("USB is not accessible: {e}"))?;
                let reason = rekordbox_to_engine::engine::replacement_reason(&target)
                    .map_err(|e| format!("Engine Library cannot be replaced: {e:#}"))?;
                let Some(reason) = reason else { return Err(error) };
                if !confirm(&reason) { return Ok(None); }
                preflight(destination, executable, true)?;
                Ok(Some(true))
            }
            #[cfg(not(target_os = "macos"))]
            { let _ = confirm; Err(error) }
        }
    }
}

fn run(destination: &Path, executable: &str) -> Result<(), String> {
    preflight(destination, executable, false)?;
    let target = destination.canonicalize().map_err(|e| format!("USB is not accessible: {e}"))?;
    let modern = ["PIONEER", ".PIONEER"].into_iter()
        .filter(|name| target.join(name).join("rekordbox/exportLibrary.db").is_file()).count();
    if modern != 1 {
        return Err("Engine DJ preparation requires exactly one modern Rekordbox exportLibrary.db.".to_owned());
    }
    // Explicit external overrides may still need the user's shell PATH.
    let mut paths: Vec<_> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(home) = std::env::var_os("HOME") { paths.push(std::path::PathBuf::from(home).join(".cargo/bin")); }
    paths.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"].map(std::path::PathBuf::from));
    let path = std::env::join_paths(paths).map_err(|e| format!("Cannot build helper search path: {e}"))?;
    let executable = executable;
    // Pass paths as arguments, never shell text. stdout is the helper's large
    // JSON report; stderr contains diagnostics and analysis progress.
    let mut child = Command::new(executable)
        .arg("--target").arg(&target)
        .env("PATH", path)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped())
        .spawn().map_err(|e| format!("Could not start helper ({executable}): {e}. Check the executable path in USB Export preferences."))?;
    let mut tail = VecDeque::with_capacity(8192);
    if let Some(mut stderr) = child.stderr.take() {
        let mut chunk = [0u8; 4096];
        loop {
            match stderr.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    tail.extend(&chunk[..n]);
                    while tail.len() > 8192 { tail.pop_front(); }
                },
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {},
                Err(e) => {
                    // Do not return while the worker can still write to USB.
                    let _ = child.wait();
                    return Err(format!("Could not read helper diagnostics: {e}"));
                },
            }
        }
    }
    let status = child.wait().map_err(|e| format!("Could not wait for helper: {e}"))?;
    if !status.success() {
        let bytes: Vec<u8> = tail.into_iter().collect();
        return Err(format!("helper exited with {status}: {}", String::from_utf8_lossy(&bytes).trim()));
    }
    if !target.join("Engine Library").is_dir() {
        return Err("helper finished without publishing an Engine Library.".to_owned());
    }
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new(script: &str) -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!("rbxport-engine-test-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir(&root).unwrap();
            let target = root.join("USB with spaces; literal");
            fs::create_dir_all(target.join("PIONEER/rekordbox")).unwrap();
            fs::write(target.join("PIONEER/rekordbox/exportLibrary.db"), b"fixture").unwrap();
            let executable = root.join("fake rb-engine");
            fs::write(&executable, format!("#!/bin/sh\nset -eu\n{script}\n")).unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
            Self(root)
        }
        fn target(&self) -> PathBuf { self.0.join("USB with spaces; literal") }
        fn run(&self) -> Result<(), String> { run(&self.target(), self.0.join("fake rb-engine").to_str().unwrap()) }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    #[test]
    fn passes_target_as_one_argument_and_waits_for_publication() {
        let f = Fixture::new("test \"$#\" = 2\ntest \"$1\" = --target\nmkdir \"$2/Engine Library\"");
        f.run().unwrap();
        assert!(f.target().join("Engine Library").is_dir());
    }
    #[test]
    fn reports_failure_and_preserves_pioneer_export() {
        let f = Fixture::new("echo 'analyzer unavailable' >&2\nexit 7");
        assert!(f.run().unwrap_err().contains("analyzer unavailable"));
        assert_eq!(fs::read(f.target().join("PIONEER/rekordbox/exportLibrary.db")).unwrap(), b"fixture");
    }
    #[test]
    fn refuses_existing_engine_library_without_starting_worker() {
        let f = Fixture::new("touch \"$2/started\"");
        fs::create_dir(f.target().join("Engine Library")).unwrap();
        assert!(f.run().unwrap_err().contains("already exists"));
        assert!(!f.target().join("started").exists());
    }
    #[test]
    fn replacing_an_unmanaged_library_requires_an_explicit_choice() {
        let f = Fixture::new("exit 0");
        fs::create_dir(f.target().join("Engine Library")).unwrap();
        fs::write(f.target().join("Engine Library/player-notes"), b"keep").unwrap();
        let declined = preflight_with_replacement(&f.target(), "", |reason| {
            assert!(reason.contains("not managed"));
            false
        }).unwrap();
        assert_eq!(declined, None);
        assert_eq!(fs::read(f.target().join("Engine Library/player-notes")).unwrap(), b"keep");
        let accepted = preflight_with_replacement(&f.target(), "", |_| true).unwrap();
        assert_eq!(accepted, Some(true));
    }
    #[test]
    fn first_export_does_not_request_replacement() {
        let f = Fixture::new("exit 0");
        assert_eq!(preflight_with_replacement(&f.target(), "", |_| panic!("unexpected confirmation")).unwrap(), Some(false));
    }
    #[test]
    fn rejects_missing_or_ambiguous_modern_export() {
        let f = Fixture::new("exit 0");
        fs::create_dir_all(f.target().join(".PIONEER/rekordbox")).unwrap();
        fs::write(f.target().join(".PIONEER/rekordbox/exportLibrary.db"), b"fixture").unwrap();
        assert!(f.run().is_err());
        fs::remove_dir_all(f.target().join("PIONEER")).unwrap();
        fs::remove_dir_all(f.target().join(".PIONEER")).unwrap();
        assert!(f.run().is_err());
    }
    #[test]
    fn success_exit_without_library_is_not_success() {
        let f = Fixture::new("exit 0");
        assert!(f.run().unwrap_err().contains("without publishing"));
    }
    #[test]
    fn drains_large_stderr_and_retains_the_error_tail() {
        let f = Fixture::new("i=0; while [ $i -lt 2000 ]; do echo 'analyzing a track with a long title' >&2; i=$((i + 1)); done\necho 'final failure' >&2\nexit 1");
        let error = f.run().unwrap_err();
        assert!(error.contains("final failure"));
        assert!(error.len() < 8300);
    }
}
