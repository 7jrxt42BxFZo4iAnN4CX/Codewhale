use super::*;
use codewhale_tui::delivery_files::GuardedFile;
use std::collections::BTreeMap;

const RECEIPT: &str = ".codewhale-install.sha256";
const MAX_RECEIPT: usize = 512;

pub(super) struct CommandReceipt {
    directory: PathBuf,
    receipt: GuardedFile,
    hashes: BTreeMap<String, String>,
}

impl CommandReceipt {
    pub(super) fn capture(executable: &Path) -> Result<Option<Self>> {
        let directory = executable
            .parent()
            .context("executable has no parent")?
            .canonicalize()?;
        let Some(mut receipt) = GuardedFile::open_bounded(&directory, RECEIPT, MAX_RECEIPT)? else {
            return Ok(None);
        };
        receipt.verify_owned()?;
        let bytes = receipt.read_bounded(MAX_RECEIPT)?;
        let text = std::str::from_utf8(&bytes).context("command receipt is not UTF-8")?;
        let hashes = parse_checksum_manifest(text)?;
        if hashes.is_empty()
            || hashes.len() > 2
            || text.lines().count() != hashes.len()
            || hashes
                .keys()
                .any(|name| !matches!(name.as_str(), "codewhale" | "codew"))
        {
            bail!(
                "Invalid command ownership receipt {}; no files were changed",
                directory.join(RECEIPT).display()
            );
        }
        for (name, digest) in &hashes {
            if let Some(mut file) = GuardedFile::open(&directory, name)? {
                file.verify_owned()?;
                if file.sha256() != digest.as_str() {
                    bail!(
                        "Preserved modified command {}; receipt no longer matches",
                        directory.join(name).display()
                    );
                }
            }
        }
        Ok(Some(Self {
            directory,
            receipt,
            hashes: hashes.into_iter().collect(),
        }))
    }

    pub(super) fn renew(&mut self, verified: &[u8]) -> Result<()> {
        self.receipt.verify_owned()?;
        let new_hash = sha256_hex(verified);
        let mut text = String::new();
        for (name, old_hash) in &self.hashes {
            if let Some(mut file) = GuardedFile::open(&self.directory, name)? {
                if file.sha256() != old_hash.as_str() && file.sha256() != new_hash.as_str() {
                    bail!(
                        "Cannot renew ownership of modified command {}",
                        self.directory.join(name).display()
                    );
                }
                file.verify_owned()?;
                text.push_str(&format!("{}  {name}\n", file.sha256()));
            }
        }
        if text.is_empty() {
            bail!("No receipt-owned commands remain; ownership receipt was preserved");
        }
        if self.receipt.read_bounded(MAX_RECEIPT)? == text.as_bytes() {
            return Ok(());
        }
        self.receipt.verify_owned()?;
        let mut staged = GuardedFile::stage(&self.directory, text.as_bytes(), false)?;
        staged.verify_owned()?;
        let backup = GuardedFile::recovery_name();
        self.receipt.move_to_vacant(&backup).with_context(|| {
            format!(
                "receipt retirement failed; recovery at {}",
                self.directory.join(self.receipt.name()).display()
            )
        })?;
        if let Err(error) = staged.move_to_vacant(RECEIPT) {
            let restored = self.receipt.move_to_vacant(RECEIPT);
            bail!(
                "receipt publication failed: {error}; restoration: {restored:?}; previous receipt {}, staged receipt {}",
                self.directory.join(self.receipt.name()).display(),
                self.directory.join(staged.name()).display()
            );
        }
        println!(
            "Renewed command ownership receipt; previous receipt retained at {}",
            self.directory.join(backup).display()
        );
        self.receipt = staged;
        Ok(())
    }
}

pub(crate) fn run(yes: bool) -> Result<()> {
    let result = remove(yes);
    report_path_copies();
    result
}

fn remove(yes: bool) -> Result<()> {
    let identity = update_executable_identity()?;
    let method = InstallMethod::detect(&identity.path);
    println!("Selected executable: {}", identity.path.display());
    println!(
        "User state, credentials, sessions, workspaces, shell profiles and services are preserved."
    );
    if !method.supports_self_update() {
        println!(
            "This copy belongs to {}. Use: {}",
            method.label(),
            method.uninstall_command()
        );
        return Ok(());
    }
    if protected_update_path(&identity.path) {
        bail!(
            "System/package path was preserved. Identify its owning package manager and remove it through that manager."
        );
    }
    let primary_name = identity
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .context("executable basename is not UTF-8")?
        .to_string();
    if !matches!(primary_name.as_str(), "codewhale" | "codew") {
        bail!(
            "This command name has no Unix installer ownership contract; preserved {}",
            identity.path.display()
        );
    }
    let ownership = CommandReceipt::capture(&identity.path)?.with_context(|| format!(
        "No command ownership receipt beside {}. This legacy/manual copy is preserved. Inspect its exact path and provenance before removing it manually; reinstalling into a fresh directory creates a receipted install.", identity.path.display()))?;
    if ownership.hashes.get(&primary_name) != Some(&identity.file_hash) {
        bail!("The receipt does not own the selected executable; nothing was removed");
    }
    let directory = ownership.directory.clone();
    let mut files = Vec::new();
    let mut primary = None;
    for name in ownership.hashes.keys() {
        if let Some(file) = GuardedFile::open(&directory, name)? {
            if ownership.hashes.get(name).map(String::as_str) != Some(file.sha256()) {
                bail!(
                    "Command changed during uninstall preflight; preserved {}",
                    directory.join(name).display()
                );
            }
            if name == &primary_name {
                primary = Some(file);
            } else {
                files.push(file);
            }
        }
    }
    let target =
        release_asset_stem_for_prefix("codewhale", std::env::consts::OS, std::env::consts::ARCH);
    let companions =
        compiled_host::owned_for_uninstall(&directory, target.trim_start_matches("codewhale-"))?;
    if companions.is_empty() {
        println!("Any unreceipted compiled-host files are preserved.");
    }
    files.extend(companions);
    let mut receipt = ownership.receipt;
    let receipt_bytes = receipt.read_bounded(MAX_RECEIPT)?;
    files.push(receipt);
    files.push(primary.context("selected executable disappeared during preflight")?);
    for file in &mut files {
        file.verify_owned()?;
        println!(
            "{} {}",
            if yes { "Remove" } else { "Would remove" },
            directory.join(file.name()).display()
        );
    }
    validate_primary_update_identity(&identity)?;
    if !yes {
        println!(
            "No files were changed. Run this executable again with uninstall --yes to remove only this plan."
        );
        return Ok(());
    }
    let preflight = tempfile::Builder::new()
        .prefix(".codewhale-uninstall-preflight-")
        .tempdir_in(&directory)?
        .keep();
    std::fs::remove_dir(&preflight).with_context(|| {
        format!(
            "uninstall directory preflight failed at {}",
            preflight.display()
        )
    })?;
    let mut removed = Vec::new();
    for file in &mut files {
        let name = file.name().to_string();
        let mut unlinked = false;
        let outcome = validate_primary_update_identity(&identity)
            .and_then(|()| file.remove_checked(&mut unlinked).map_err(Into::into));
        if unlinked {
            removed.push(name.clone());
        }
        if let Err(error) = outcome {
            if removed.iter().any(|name| name == RECEIPT)
                && !removed.iter().any(|name| name == &primary_name)
            {
                let restore = (|| {
                    let mut staged = GuardedFile::stage(&directory, &receipt_bytes, false)?;
                    staged.verify_owned()?;
                    staged.move_to_vacant(RECEIPT).with_context(|| {
                        format!(
                            "receipt recovery bytes retained at {}",
                            directory.join(staged.name()).display()
                        )
                    })
                })();
                if let Err(restore) = restore {
                    eprintln!("Receipt recovery needs attention: {restore:#}");
                }
            }
            bail!(
                "Uninstall stopped at {name}: {error:#}. Already unlinked: {}. Any retained recovery locations are reported above; no unknown destination is overwritten.",
                removed.join(", ")
            );
        }
    }
    println!(
        "Removed this receipted Unix installation. Data and other installations are unchanged. Run hash -r in your shell."
    );
    Ok(())
}

fn report_path_copies() {
    let mut seen = std::collections::HashSet::new();
    for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        for name in [
            "codewhale",
            "codew",
            "codewhale-tui",
            "deepseek",
            "deepseek-tui",
        ] {
            let path = directory.join(name);
            if seen.insert(path.clone()) && std::fs::symlink_metadata(&path).is_ok() {
                println!("Remaining PATH entry (preserved): {}", path.display());
            }
        }
    }
}
