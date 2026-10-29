// Installing a newer release: download its setup, check it, start it out of this
// process. The setup closes this app itself (`--quit`), installs, and starts the new
// version (/RELAUNCH=1, installer/claude-usage.iss), so nothing here exits.

use super::*;
use sha2::Digest;
use std::io::{Read, Write};

// Silent, no "This will install..." prompt, never a reboot; its log beside the download.
const FIXED_ARGS: [&str; 5] = ["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-", "/RELAUNCH=1"];

pub(crate) fn setup_args(dir: &Path) -> Vec<String> {
    let mut args: Vec<String> = FIXED_ARGS.iter().map(|a| a.to_string()).collect();
    args.push(format!("/LOG={}", dir.join("setup.log").display()));
    args
}

fn update_dir() -> PathBuf {
    std::env::temp_dir().join("claude-usage-update")
}

impl App {
    // Downloads, checks and starts the setup; blocking (the tray calls it off its thread).
    pub(crate) fn install_update(&self) -> Result<(), String> {
        self.begin_install()?;
        self.finish_install()
    }

    // Claims the install: only from "available", and not while a Claude Desktop switch
    // or a sign-in runs (the setup closing this app would cut it short).
    pub(crate) fn begin_install(&self) -> Result<(), String> {
        if self.demo {
            return Err("Updates are off in the demo.".into());
        }
        if let Some(why) = self.update_blocked() {
            return Err(why.into());
        }
        let mut u = self.update.lock().unwrap();
        if u.phase != Phase::Available {
            return Err("No update to install; check for updates first.".into());
        }
        u.phase = Phase::Downloading;
        u.error = None;
        Ok(())
    }

    pub(crate) fn finish_install(&self) -> Result<(), String> {
        let res = self.fetch_setup().and_then(|(asset, path)| {
            let mut st = self.st.lock().unwrap();
            self.set_update(|u| u.phase = Phase::Installing);
            self.log(&mut st, format!("Installing update {}", asset.version));
            self.save(&st);
            drop(st);
            run_setup(&path)
        });
        if let Err(e) = &res {
            let mut st = self.st.lock().unwrap();
            self.set_update(|u| {
                u.phase = Phase::Error;
                u.error = Some(e.clone());
            });
            self.log(&mut st, format!("Update failed: {e}"));
            self.save(&st);
        }
        res
    }

    // The latest release's setup in %TEMP%\claude-usage-update, checked: written as
    // .part, renamed only once its size and SHA-256 match the release's.
    fn fetch_setup(&self) -> Result<(Asset, PathBuf), String> {
        let asset = pick_asset(&latest_release(&self.http)?)?;
        if !newer(&asset.version, VERSION) {
            return Err(format!("{} is not newer than this version ({VERSION}).", asset.version));
        }
        let dir = update_dir();
        fs::create_dir_all(&dir).map_err(|e| format!("Could not make {}: {e}", dir.display()))?;
        let (path, part) = (dir.join(&asset.name), dir.join(format!("{}.part", asset.name)));
        if let Err(e) = download(&asset, &part) {
            let _ = fs::remove_file(&part);
            return Err(e);
        }
        let _ = fs::remove_file(&path);
        fs::rename(&part, &path).map_err(|e| format!("Could not keep the download: {e}"))?;
        Ok((asset, path))
    }
}

// Streams the setup to `to`, hashing as it writes. Its own agent: the API's 20 s limit
// covers the whole body, too short for a setup on a slow line.
fn download(asset: &Asset, to: &Path) -> Result<(), String> {
    let http = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(20)).timeout_read(Duration::from_secs(60)).build();
    let resp = http
        .get(&asset.url)
        .set("User-Agent", &user_agent())
        .set("Accept", "application/octet-stream")
        .call()
        .map_err(|e| format!("Could not download {}: {e}", asset.name))?;
    // GitHub redirects to its asset host; anywhere else is not followed further.
    if !download_host_ok(resp.get_url()) {
        return Err(format!("The download of {} was sent somewhere other than GitHub.", asset.name));
    }
    let mut file = fs::File::create(to).map_err(|e| format!("Could not write {}: {e}", to.display()))?;
    let mut sha = sha2::Sha256::new();
    let mut body = resp.into_reader().take(asset.size + 1);
    let (mut buf, mut got) = (vec![0u8; 64 * 1024], 0u64);
    loop {
        let n = body.read(&mut buf).map_err(|e| format!("The download broke off: {e}"))?;
        if n == 0 {
            break;
        }
        sha.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| format!("Could not write {}: {e}", to.display()))?;
        got += n as u64;
    }
    file.sync_all().map_err(|e| format!("Could not write {}: {e}", to.display()))?;
    if got != asset.size {
        return Err(format!("The download has {got} bytes, GitHub says {}; it was deleted.", asset.size));
    }
    if hex(&sha.finalize()) != asset.sha256 {
        return Err("The download does not match GitHub's SHA-256 digest; it was deleted.".into());
    }
    Ok(())
}

// Started out of this app's job when allowed (a job may end its processes with it),
// else plainly; no console or stdio of ours (they may be gone, main.rs).
#[cfg(windows)]
fn run_setup(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Stdio;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    let args = setup_args(path.parent().unwrap_or(Path::new(".")));
    let spawn = |flags: u32| {
        Command::new(path).args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).creation_flags(flags).spawn()
    };
    spawn(CREATE_BREAKAWAY_FROM_JOB).or_else(|_| spawn(0)).map(|_| ()).map_err(|e| format!("Could not start the setup: {e}"))
}

#[cfg(not(windows))]
fn run_setup(_: &Path) -> Result<(), String> {
    Err("Installing updates works on Windows only.".into())
}
