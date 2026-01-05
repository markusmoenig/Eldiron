#![cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]

use std::vec;

use self_update::{
    Status, cargo_crate_version,
    errors::Error,
    update::{Release, ReleaseUpdate},
    version::bump_is_greater,
};

pub enum SelfUpdateEvent {
    AlreadyUpToDate,
    UpdateCompleted(Release),
    UpdateConfirm(Release),
    UpdateError(String),
    UpdateStart(Release),
}

pub struct SelfUpdater {
    bin_name: String,
    current_version: String,
    latest_release: Option<Release>,
    locked: bool,
    release_list: Vec<Release>,
    repo_name: String,
    repo_owner: String,
}

impl SelfUpdater {
    pub fn new(repo_owner: &str, repo_name: &str, bin_name: &str) -> Self {
        Self {
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
                    if bump_is_greater(&acc.version, &release.version).unwrap() {
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
                bump_is_greater(&self.current_version, &latest_release.version).unwrap_or_default()
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

    pub fn update_latest(&mut self) -> Result<Status, Error> {
        if let Some(release) = self.latest_release() {
            return self.update(&release.clone());
        }

        Err(Error::Release("Latest release not found.".to_string()))
    }

    fn build_update(&self, version_tag: &str) -> Result<Box<dyn ReleaseUpdate>, Error> {
        self_update::backends::github::Update::configure()
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .bin_name(&self.bin_name)
            .current_version(&self.current_version)
            .no_confirm(true)
            .target_version_tag(version_tag)
            .build()
    }
}
