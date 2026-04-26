#![cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]

use std::vec;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use self_update::{Status, update::ReleaseUpdate};
use self_update::{cargo_crate_version, errors::Error, update::Release, version::bump_is_greater};

pub enum SelfUpdateEvent {
    AlreadyUpToDate,
    UpdateAvailable(Release),
    UpdateCompleted(Release),
    UpdateConfirm(Release),
    UpdateError(String),
    UpdateStart(Release),
}

pub struct SelfUpdater {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    bin_name: String,
    current_version: String,
    latest_release: Option<Release>,
    locked: bool,
    release_list: Vec<Release>,
    repo_name: String,
    repo_owner: String,
}

impl SelfUpdater {
    pub fn github_creator() -> Self {
        Self::new("markusmoenig", "Eldiron", "eldiron-creator")
    }

    pub fn new(repo_owner: &str, repo_name: &str, bin_name: &str) -> Self {
        #[cfg(target_os = "macos")]
        let _ = bin_name;

        Self {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            bin_name: bin_name.to_string(),
            current_version: cargo_crate_version!().to_string(),
            latest_release: None,
            locked: false,
            release_list: vec![],
            repo_name: repo_name.to_string(),
            repo_owner: repo_owner.to_string(),
        }
    }

    pub fn current_version(&self) -> &str {
        &self.current_version
    }

    pub fn fetch_release_list(&mut self) -> Result<(), Error> {
        if self.is_locked() {
            return Err(Error::Update(
                "Another operation is already executing.".to_string(),
            ));
        }

        self.locked = true;

        let release_list = self_update::backends::github::ReleaseList::configure()
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .build()
            .and_then(|release_list| release_list.fetch());

        self.locked = false;

        if let Ok(release_list) = release_list {
            self.release_list = release_list;

            self.latest_release = self
                .release_list
                .iter()
                .reduce(|acc, release| {
                    if bump_is_greater(&release.version, &acc.version).unwrap_or_default() {
                        return release;
                    }

                    acc
                })
                .cloned();

            return Ok(());
        }

        release_list.and(Ok(()))
    }

    pub fn get_release_by_version(&self, version_tag: &str) -> Option<&Release> {
        self.release_list
            .iter()
            .find(|release| release.version == version_tag)
    }

    pub fn has_newer_release(&self) -> bool {
        self.latest_release()
            .map(|latest_release| {
                bump_is_greater(&latest_release.version, &self.current_version).unwrap_or_default()
            })
            .unwrap_or_default()
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn latest_release(&self) -> Option<&Release> {
        self.latest_release.as_ref()
    }

    pub fn release_list(&self) -> &[Release] {
        &self.release_list
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    pub fn update(&mut self, release: &Release) -> Result<Status, Error> {
        if self.is_locked() {
            return Err(Error::Update(
                "Another operation is already executing.".to_string(),
            ));
        }

        self.locked = true;

        let result = self
            .build_update(&format!("v{}", release.version))
            .and_then(|release_update| release_update.update());

        if result.is_ok() {
            self.current_version.clone_from(&release.version);
        }

        self.locked = false;

        result
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    pub fn update_latest(&mut self) -> Result<Status, Error> {
        if let Some(release) = self.latest_release() {
            return self.update(&release.clone());
        }

        Err(Error::Release("Latest release not found.".to_string()))
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    fn build_update(&self, version_tag: &str) -> Result<Box<dyn ReleaseUpdate>, Error> {
        self_update::backends::github::Update::configure()
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .bin_name(&self.bin_name)
            .target(Self::release_target())
            .identifier(&self.bin_name)
            .current_version(&self.current_version)
            .no_confirm(true)
            .target_version_tag(version_tag)
            .build()
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    fn release_target() -> &'static str {
        #[cfg(target_os = "windows")]
        {
            "x86_64-pc-windows-msvc"
        }

        #[cfg(target_os = "linux")]
        {
            "x86_64-unknown-linux-gnu"
        }
    }
}
