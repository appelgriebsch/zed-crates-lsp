#![warn(clippy::all, clippy::pedantic)]

use std::fs;
use zed_extension_api::LanguageServerId;
use zed_extension_api::{self as zed, Result};

const EXTENSION_NAME: &str = "crates-lsp";

struct CratesLSPExtension {
    cached_binary_path: Option<String>,
}

impl CratesLSPExtension {
    /// Returns the path to the `crates-lsp` binary. If the binary is not found, it will download
    /// the latest version from GitHub and extract it to the current working directory.
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        if let Some(path) = worktree.which(EXTENSION_NAME) {
            return Ok(path);
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            format!("MathiasPius/{}", EXTENSION_NAME).as_str(),
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();

        let asset_name = format!(
            "{}-{arch}-{os}",
            EXTENSION_NAME,
            arch = match arch {
                zed::Architecture::Aarch64 => "aarch64",
                zed::Architecture::X86 => "x86",
                zed::Architecture::X8664 => "x86_64",
            },
            os = match platform {
                zed::Os::Mac => "apple-darwin.tar.gz",
                zed::Os::Linux => "unknown-linux-gnu.tar.gz",
                zed::Os::Windows => "pc-windows-msvc.zip",
            },
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {asset_name:?}"))?;

        let version_dir = format!("{}-{}", EXTENSION_NAME, release.version);

        fs::create_dir_all(&version_dir)
            .map_err(|err| format!("failed to create directory '{version_dir}': {err}"))?;

        let binary_path = format!(
            "{version_dir}/{bin_name}",
            bin_name = match platform {
                zed::Os::Windows => format!("{}.exe", EXTENSION_NAME),
                zed::Os::Mac | zed::Os::Linux => EXTENSION_NAME.to_string(),
            }
        );

        let file_type = match platform {
            zed::Os::Windows => zed::DownloadedFileType::Zip,
            zed::Os::Mac | zed::Os::Linux => zed::DownloadedFileType::GzipTar,
        };

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(&asset.download_url, &version_dir, file_type)
                .map_err(|err| format!("failed to download file: {err}"))?;

            zed::make_file_executable(&binary_path)?;

            let entries = fs::read_dir(".")
                .map_err(|err| format!("failed to list working directory {err}"))?;
            for entry in entries {
                let entry = entry.map_err(|err| format!("failed to load directory entry {err}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());

        Ok(binary_path)
    }
}

impl zed::Extension for CratesLSPExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: self.language_server_binary_path(language_server_id, worktree)?,
            args: Vec::default(),
            env: Vec::default(),
        })
    }
}

zed::register_extension!(CratesLSPExtension);
