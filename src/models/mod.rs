pub mod asset_data;
pub mod category_data;
pub mod database;
pub mod engine_data;
pub mod project_data;

use crate::config::APP_ID;
use egs_api::EpicGames;
use gtk4::gio;
use gtk4::glib::{MainContext, Receiver, Sender, UserDirectory, PRIORITY_DEFAULT};
use gtk4::prelude::*;
use log::{debug, error, info};

#[cfg(target_os = "linux")]
use secret_service::{EncryptionType, SecretService};

use std::cell::RefCell;
use std::thread;

pub struct Model {
    pub epic_games: RefCell<EpicGames>,
    #[cfg(target_os = "linux")]
    pub secret_service: Option<SecretService<'static>>,
    pub sender: Sender<crate::ui::messages::Msg>,
    pub receiver: RefCell<Option<Receiver<crate::ui::messages::Msg>>>,
    pub settings: gio::Settings,
    #[cfg(target_os = "linux")]
    pub dclient: RefCell<Option<ghregistry::Client>>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        let mut obj = Self {
            epic_games: RefCell::new(EpicGames::new()),
            #[cfg(target_os = "linux")]
            secret_service: match SecretService::new(EncryptionType::Dh) {
                Ok(ss) => Some(ss),
                Err(e) => {
                    error!(
                        "Unable to initialize Secret service no secrets will be stored: {}",
                        e
                    );
                    None
                }
            },
            sender,
            receiver: RefCell::new(Some(receiver)),
            settings: gio::Settings::new(APP_ID),
            #[cfg(target_os = "linux")]
            dclient: RefCell::new(None),
        };
        obj.load_secrets();
        obj.load_defaults();
        obj
    }

    fn load_defaults(&mut self) {
        if self.settings.string("cache-directory").is_empty() {
            let mut dir = gtk4::glib::user_cache_dir();
            dir.push("epic_asset_manager");
            self.settings
                .set_string("cache-directory", dir.to_str().unwrap())
                .unwrap();
        }

        if self
            .settings
            .string("temporary-download-directory")
            .is_empty()
        {
            let mut dir = gtk4::glib::tmp_dir();
            dir.push("epic_asset_manager");
            self.settings
                .set_string("temporary-download-directory", dir.to_str().unwrap())
                .unwrap();
        }

        if self.settings.strv("unreal-projects-directories").is_empty() {
            let mut dir = gtk4::glib::user_special_dir(UserDirectory::Documents);
            dir.push("Unreal Projects");
            self.settings
                .set_strv("unreal-projects-directories", &[dir.to_str().unwrap()])
                .unwrap();
        }

        if self.settings.strv("unreal-vault-directories").is_empty() {
            let mut dir = gtk4::glib::user_special_dir(UserDirectory::Documents);
            dir.push("EpicVault");
            self.settings
                .set_strv("unreal-vault-directories", &[dir.to_str().unwrap()])
                .unwrap();
        }

        if self.settings.strv("unreal-engine-directories").is_empty() {
            let mut dir = gtk4::glib::user_special_dir(UserDirectory::Documents);
            dir.push("Unreal Engine");
            self.settings
                .set_strv("unreal-engine-directories", &[dir.to_str().unwrap()])
                .unwrap();
        }
    }

    pub fn validate_registry_login(&self, user: String, token: String) {
        debug!("Trying to validate token for {}", user);
        #[cfg(target_os = "linux")]
        {
            let client = ghregistry::Client::configure()
                .registry("ghcr.io")
                .insecure_registry(false)
                .username(Some(user))
                .password(Some(token))
                .build()
                .unwrap();
            let sender = self.sender.clone();
            thread::spawn(move || {
                let login_scope = "repository:epicgames/unreal-engine:pull";
                match client.authenticate(&[login_scope]) {
                    Ok(docker_client) => match docker_client.is_auth() {
                        Ok(auth) => {
                            if auth {
                                sender
                                    .send(crate::ui::messages::Msg::DockerClient(docker_client))
                                    .unwrap();
                                info!("Docker Authenticated");
                            }
                        }
                        Err(e) => {
                            error!("Failed authentication verification {:?}", e);
                        }
                    },
                    Err(e) => {
                        error!("Failed authentication {:?}", e);
                        sender
                            .send(crate::ui::messages::Msg::GithubAuthFailed)
                            .unwrap();
                    }
                };
            });
        }
    }

    fn load_secrets(&mut self) {
        #[cfg(target_os = "linux")]
        {
            match &self.secret_service {
                None => {
                    error!("Unable to load secrets from Secret service");
                }
                Some(ss) => {
                    if let Ok(collection) = ss.get_any_collection() {
                        if let Ok(items) = collection.search_items(
                            [("application", crate::config::APP_ID)]
                                .iter()
                                .copied()
                                .collect(),
                        ) {
                            let mut ud = egs_api::api::UserData::new();
                            for item in items {
                                let label = if let Ok(l) = item.get_label() {
                                    l
                                } else {
                                    debug!("No label skipping");
                                    continue;
                                };
                                debug!("Loading: {}", label);
                                if let Ok(attributes) = item.get_attributes() {
                                    match label.as_str() {
                                        "eam_epic_games_token" => {
                                            let t = match attributes.get("type") {
                                                None => {
                                                    debug!("Access token does not have type");
                                                    continue;
                                                }
                                                Some(v) => v.clone(),
                                            };
                                            let exp = match chrono::DateTime::parse_from_rfc3339(
                                                self.settings.string("token-expiration").as_str(),
                                            ) {
                                                Ok(d) => d.with_timezone(&chrono::Utc),
                                                Err(e) => {
                                                    debug!(
                                                        "Failed to parse token expiration date {}",
                                                        e
                                                    );
                                                    continue;
                                                }
                                            };
                                            let now = chrono::offset::Utc::now();
                                            let td = exp - now;
                                            if td.num_seconds() < 600 {
                                                info!("Token {} is expired, removing", label);
                                                item.delete().unwrap_or_default();
                                                continue;
                                            }
                                            ud.expires_at = Some(exp);
                                            ud.token_type = Some(t);
                                            if let Ok(d) = item.get_secret() {
                                                if let Ok(s) = std::str::from_utf8(d.as_slice()) {
                                                    debug!("Loaded access token");
                                                    ud.set_access_token(Some(s.to_string()));
                                                }
                                            };
                                        }
                                        "eam_epic_games_refresh_token" => {
                                            let exp = match chrono::DateTime::parse_from_rfc3339(
                                                self.settings
                                                    .string("refresh-token-expiration")
                                                    .as_str(),
                                            ) {
                                                Ok(d) => d.with_timezone(&chrono::Utc),
                                                Err(e) => {
                                                    debug!(
                                                "Failed to parse refresh token expiration date {}",
                                                e
                                            );
                                                    continue;
                                                }
                                            };
                                            let now = chrono::offset::Utc::now();
                                            let td = exp - now;
                                            if td.num_seconds() < 600 {
                                                info!("Token {} is expired, removing", label);
                                                item.delete().unwrap_or_default();
                                                continue;
                                            }
                                            ud.refresh_expires_at = Some(exp);
                                            if let Ok(d) = item.get_secret() {
                                                if let Ok(s) = std::str::from_utf8(d.as_slice()) {
                                                    debug!("Loaded refresh token");
                                                    ud.set_refresh_token(Some(s.to_string()));
                                                }
                                            };
                                        }
                                        "eam_github_token" => {
                                            if let Ok(d) = item.get_secret() {
                                                if let Ok(s) = std::str::from_utf8(d.as_slice()) {
                                                    self.validate_registry_login(
                                                        self.settings
                                                            .string("github-user")
                                                            .to_string(),
                                                        s.to_string(),
                                                    );
                                                }
                                            };
                                        }
                                        &_ => {}
                                    }
                                }
                            }
                            self.epic_games.borrow_mut().set_user_details(ud);
                        };
                    };
                }
            }
        }
    }
}
