use crate::*;
use serde::{Serialize, de::DeserializeOwned};
use std::io::Cursor;
const MAGIC: &[u8; 4] = b"ELDN";
const HEADER: usize = 12;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    TooLarge,
    InvalidEnvelope,
    WrongVersion,
    InvalidPayload,
}
impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for CodecError {}
fn encode<T: Serialize>(value: &T, kind: u8, limit: usize) -> Result<Vec<u8>, CodecError> {
    let payload = rmp_serde::to_vec_named(value).map_err(|_| CodecError::InvalidPayload)?;
    if payload.len() + HEADER > limit {
        return Err(CodecError::TooLarge);
    }
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.extend(MAGIC);
    out.extend(PROTOCOL_VERSION.to_be_bytes());
    out.extend([kind, 0]);
    out.extend((payload.len() as u32).to_be_bytes());
    out.extend(payload);
    Ok(out)
}
fn decode<T: DeserializeOwned>(bytes: &[u8], kind: u8, limit: usize) -> Result<T, CodecError> {
    if bytes.len() > limit {
        return Err(CodecError::TooLarge);
    }
    if bytes.len() < HEADER || &bytes[..4] != MAGIC || bytes[6] != kind || bytes[7] != 0 {
        return Err(CodecError::InvalidEnvelope);
    }
    if u16::from_be_bytes([bytes[4], bytes[5]]) != PROTOCOL_VERSION {
        return Err(CodecError::WrongVersion);
    }
    if u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize != bytes.len() - HEADER {
        return Err(CodecError::InvalidEnvelope);
    }
    let mut decoder = rmp_serde::Deserializer::new(Cursor::new(&bytes[HEADER..]));
    decoder.set_max_depth(64);
    let result = T::deserialize(&mut decoder).map_err(|_| CodecError::InvalidPayload)?;
    if decoder.position() as usize != bytes.len() - HEADER {
        return Err(CodecError::InvalidPayload);
    }
    Ok(result)
}
pub fn encode_client(value: &ClientMessage) -> Result<Vec<u8>, CodecError> {
    encode(value, 1, MAX_CLIENT_BYTES)
}
pub fn decode_client(bytes: &[u8]) -> Result<ClientMessage, CodecError> {
    let msg: ClientMessage = decode(bytes, 1, MAX_CLIENT_BYTES)?;
    let valid = match &msg {
        ClientMessage::Register { username, password }
        | ClientMessage::Login { username, password } => {
            username.len() <= 64 && password.0.len() <= 256
        }
        ClientMessage::ChangePassword {
            current_password,
            new_password,
        } => current_password.0.len() <= 256 && new_password.0.len() <= 256,
        ClientMessage::Guest { name } | ClientMessage::CreateCharacter { name, .. } => {
            name.len() <= 64
        }
        ClientMessage::Inputs { frames } => frames.len() <= 32,
        ClientMessage::Action(action) => action.bounded(),
        ClientMessage::Resume { token } => token.0.len() == 64,
        _ => true,
    };
    if !valid {
        return Err(CodecError::InvalidPayload);
    }
    Ok(msg)
}
pub fn encode_server(value: &ServerMessage) -> Result<Vec<u8>, CodecError> {
    encode(value, 2, MAX_SERVER_BYTES)
}
pub fn decode_server(bytes: &[u8]) -> Result<ServerMessage, CodecError> {
    let message: ServerMessage = decode(bytes, 2, MAX_SERVER_BYTES)?;
    let valid = match &message {
        ServerMessage::Frame { content, .. } => {
            content.len() <= eldiron_runtime::replication::MAX_REGIONS
        }
        ServerMessage::Preload { content } => content.len() <= 4,
        _ => true,
    };
    if !valid {
        return Err(CodecError::InvalidPayload);
    }
    Ok(message)
}
pub fn encode_content(value: &PhysicsContent) -> Result<Vec<u8>, CodecError> {
    encode(value, 3, MAX_CONTENT_BYTES)
}
pub fn decode_content(bytes: &[u8]) -> Result<PhysicsContent, CodecError> {
    decode(bytes, 3, MAX_CONTENT_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_envelopes_and_secret_redaction() {
        let msg = ClientMessage::Login {
            username: "tester".into(),
            password: Secret("never print me".into()),
        };
        let bytes = encode_client(&msg).unwrap();
        assert!(matches!(
            decode_client(&bytes).unwrap(),
            ClientMessage::Login { .. }
        ));
        assert!(!format!("{msg:?}").contains("never print me"));
        assert!(decode_server(&bytes).is_err());
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode_client(&trailing).is_err());
        let mut version = bytes.clone();
        version[5] += 1;
        assert_eq!(
            decode_client(&version).unwrap_err(),
            CodecError::WrongVersion
        );
        for i in 0..bytes.len() {
            assert!(decode_client(&bytes[..i]).is_err());
        }
        assert_eq!(
            decode_client(&vec![0; MAX_CLIENT_BYTES + 1]).unwrap_err(),
            CodecError::TooLarge
        );
    }
    #[test]
    fn oversized_and_forged_requests_are_rejected() {
        let msg = ClientMessage::Guest {
            name: "a".repeat(65),
        };
        assert!(decode_client(&encode_client(&msg).unwrap()).is_err());
        // No raw EntityAction, arbitrary host event, owner ID, speed or teleport API.
        let bytes = encode(
            &serde_json::json!({"Teleport": {"position":[1,2]}}),
            1,
            MAX_CLIENT_BYTES,
        )
        .unwrap();
        assert!(decode_client(&bytes).is_err());
    }
}

#[cfg(test)]
mod account_tests {
    use super::*;
    #[test]
    fn account_messages_bound_secrets_and_preserve_legacy_welcome() {
        let change = ClientMessage::ChangePassword {
            current_password: Secret("private original password".into()),
            new_password: Secret("private replacement password".into()),
        };
        let debug = format!("{change:?}");
        assert!(!debug.contains("private original"));
        assert!(!debug.contains("private replacement"));
        assert!(matches!(
            decode_client(&encode_client(&change).unwrap()).unwrap(),
            ClientMessage::ChangePassword { .. }
        ));
        let huge = ClientMessage::ChangePassword {
            current_password: Secret("x".repeat(257)),
            new_password: Secret("x".repeat(12)),
        };
        assert!(decode_client(&encode_client(&huge).unwrap()).is_err());
        let legacy=encode(&serde_json::json!({"Welcome":{"version":PROTOCOL_VERSION,"mode":"Persistent","content_revision":"a".repeat(64)}}),2,MAX_SERVER_BYTES).unwrap();
        assert!(matches!(
            decode_server(&legacy).unwrap(),
            ServerMessage::Welcome {
                account_management: false,
                ..
            }
        ));
        let update = ServerMessage::AccountUpdated(AccountUpdate::PasswordChanged);
        assert!(matches!(
            decode_server(&encode_server(&update).unwrap()).unwrap(),
            ServerMessage::AccountUpdated(AccountUpdate::PasswordChanged)
        ));
    }
}
