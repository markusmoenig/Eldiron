use crate::prelude::*;
use IOError::*;

use pbkdf2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Pbkdf2
};

// This implementation is for server based games with multiple users and login.

pub struct UserFS {

    path                        : PathBuf,
    users_path                  : PathBuf,
}

impl ServerIO for UserFS {

    fn new() -> Self where Self: Sized {
        Self {
            path                : PathBuf::new(),
            users_path          : PathBuf::new(),
        }
    }

    fn set_local_path(&mut self, path: PathBuf) {
        let users_path = path.join("users");

        if fs::metadata(users_path.clone()).is_ok() == false {
            _ = fs::create_dir(users_path.clone());
        }

        self.path = path;
        self.users_path = users_path;
    }

    fn login_user(&self, user_name: String, password: String) -> Result<(), IOError> {
        let user_path = self.users_path.join(user_name);
        if fs::metadata(user_path.clone()).is_ok() == true {
            if let Some(password_hash) = fs::read_to_string(user_path.join("password")).ok() {
                if let Some(parsed_hash) = PasswordHash::new(&password_hash).ok() {
                    if let Some(_result) = Pbkdf2.verify_password(password.as_ref(), &parsed_hash).ok() {

                        // If not already exist, create the users characters directory
                        let characters_path = user_path.join("characters");
                        if fs::metadata(characters_path.clone()).is_ok() == false {
                            _ = fs::create_dir(characters_path);
                        }

                        return Ok(());
                    } else {
                        return Err(WrongPassword);
                    }
                } else {
                    return Err(WrongPassword);
                }
            }
        }
        Err(UserNotFound)
    }

    fn create_user(&self, user_name: String, password: String) -> Result<(), IOError> {
        let user_path = self.users_path.join(user_name);
        if fs::metadata(user_path.clone()).is_ok() == false {
            if fs::create_dir(user_path.clone()).is_ok() {
                let password_path = user_path.join("password");
                let salt = SaltString::generate(&mut OsRng);
                if let Some(password_hash) = Pbkdf2.hash_password(password.as_ref(), &salt).ok() {
                    if  fs::write(password_path, password_hash.to_string()).is_ok() {
                        return Ok(());
                    }
                }
            }
        } else {
            return Err(UserAlreadyExists);
        }

        Err(UserNotFound)
    }

    fn get_user_character(&self, user_name: String, character_name: String) -> Result<Sheet, IOError> {
        let character_path = self.users_path.join(user_name).join("characters");
        if fs::metadata(character_path.clone()).is_ok() == true {
            let sheet_path = character_path.join(character_name.clone());
            if let Some(sheet_str) = fs::read_to_string(sheet_path).ok() {
                if let Some(sheet) = serde_json::from_str::<Sheet>(&sheet_str).ok() {
                    return Ok(sheet);
                }
            }
        }
        Err(UserNotFound)
    }

    fn save_user_character(&self, user_name: String, sheet: Sheet) -> Result<(), IOError> {
        let character_path = self.users_path.join(user_name).join("characters");
        if fs::metadata(character_path.clone()).is_ok() == true {
            let sheet_path = character_path.join(sheet.name.clone());
            if let Some(json) = serde_json::to_string_pretty(&sheet).ok() {
                if  fs::write(sheet_path, json.to_string()).is_ok() {
                    return Ok(());
                }
            }
        } else {
            return Err(UserNotFound);
        }
        Err(UserNotFound)
    }

    fn list_user_characters(&self, user_name: String) -> Result<Vec<CharacterData>, IOError> {
        let character_path = self.users_path.join(user_name).join("characters");

        let mut characters = vec![];
        let mut paths: Vec<_> = fs::read_dir(character_path.clone()).unwrap()
                                                .map(|r| r.unwrap())
                                                .collect();
        paths.sort_by_key(|dir| dir.path());

        for path in paths {
            let path = &path.path();

            if let Some(sheet_str) = fs::read_to_string(path).ok() {
                if let Some(sheet) = serde_json::from_str::<Sheet>(&sheet_str).ok() {
                    let char_data = CharacterData {
                        id                      : Uuid::new_v4(),
                        name                    : sheet.name,
                        tile                    : TileId { tilemap: Uuid::new_v4(), x_off: 0, y_off: 0, size: None },
                        index                   : 0,
                        position                : Position::new(Uuid::new_v4(), 0, 0),
                        old_position            : None,
                        max_transition_time     : 0,
                        curr_transition_time    : 0,
                        effects                 : vec![]
                    };
                    characters.push(char_data);
                }
            }
        }

        Ok(characters)
    }


}