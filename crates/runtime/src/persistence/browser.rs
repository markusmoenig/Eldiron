use super::*;
use wasm_bindgen::prelude::*;
/// IndexedDB stores the complete checkpoint and CAS revision in one transaction.
/// A second tab cannot overwrite a newer revision. Storage/quota/permission
/// errors are returned, never replaced with an empty/new world.
pub struct BrowserWorldStore {
    database: String,
    slot: String,
}
impl BrowserWorldStore {
    pub fn new(database: impl Into<String>, slot: impl Into<String>) -> Self {
        Self {
            database: database.into(),
            slot: slot.into(),
        }
    }
}
#[wasm_bindgen(module = "/src/persistence/indexed_db.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn loadWorld(database: &str, slot: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn commitWorld(
        database: &str,
        slot: &str,
        expected: &str,
        revision: &str,
        identity: &str,
        bytes: &[u8],
    ) -> Result<JsValue, JsValue>;
}
fn error(value: JsValue) -> SaveError {
    if value.as_string().as_deref() == Some("save-conflict") {
        SaveError::Conflict
    } else {
        SaveError::Store(format!("IndexedDB: {value:?}"))
    }
}
impl WorldStore for BrowserWorldStore {
    async fn load(&mut self) -> Result<Option<WorldSave>, SaveError> {
        let value = loadWorld(&self.database, &self.slot).await.map_err(error)?;
        if value.is_null() || value.is_undefined() {
            return Ok(None);
        }
        let bytes = js_sys::Uint8Array::new(&value).to_vec();
        Ok(Some(WorldSave::decode(&bytes)?))
    }
    async fn commit(&mut self, expected: Option<u64>, saved: &WorldSave) -> Result<(), SaveError> {
        let prepared = PreparedSave::new(saved)?;
        self.commit_prepared(expected, &prepared).await
    }
    async fn commit_prepared(
        &mut self,
        expected: Option<u64>,
        prepared: &PreparedSave,
    ) -> Result<(), SaveError> {
        let saved = prepared.image();
        // Native/portable validations run before entering the browser transaction.
        if saved.revision
            != expected
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(SaveError::Conflict)?
        {
            return Err(SaveError::Conflict);
        }

        commitWorld(
            &self.database,
            &self.slot,
            &expected.map(|r| r.to_string()).unwrap_or_default(),
            &saved.revision.to_string(),
            &format!("{}:{}:{:?}", saved.world, saved.content, saved.purpose),
            prepared.bytes(),
        )
        .await
        .map_err(error)?;
        Ok(())
    }
}
