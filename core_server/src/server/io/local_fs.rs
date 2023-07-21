use crate::prelude::*;
use IOError::*;

// This implementation is for local games with a single user. No login required.

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

    fn login_user(&self, _user_name: String, _password: String) -> Result<(), IOError> {
        return Ok(());
    }

    fn create_user(&self, _user_name: String, _password: String) -> Result<(), IOError> {
        let user_name = "local".to_string();
        let user_path = self.users_path.join(user_name);
        if fs::metadata(user_path.clone()).is_ok() == false {
            if fs::create_dir(user_path.clone()).is_ok() {
                return Ok(());
            }
        } else {
            return Err(UserAlreadyExists);
        }

        Err(UserNotFound)
    }

    fn get_user_character(&self, _user_name: String, character_name: String) -> Result<Sheet, IOError> {
        let user_name = "local".to_string();
        _ = self.create_user(user_name.clone(), "".to_string());
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

    fn save_user_character(&self, _user_name: String, sheet: Sheet) -> Result<(), IOError> {
        let user_name = "local".to_string();
        _ = self.create_user(user_name.clone(), "".to_string());
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

    fn list_user_characters(&self, _user_name: String) -> Result<Vec<CharacterData>, IOError> {
        let user_name = "local".to_string();
        _ = self.create_user(user_name.clone(), "".to_string());

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