//! Private, versioned owner protocol. No bearer credential or generic URL is a
//! discovery result. The fixed CLI operation registry remains the only router.
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[cfg(windows)]
pub(crate) mod lifetime;
#[cfg(windows)]
pub(crate) mod security;
#[cfg(windows)]
pub(crate) mod windows;

pub(crate) const VERSION: u32 = 1;
const MAX_HEADER: usize = 64 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Hello {
    pub version: u32,
    pub profile_identity: String,
    pub challenge: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Welcome {
    pub version: u32,
    pub profile_identity: String,
    pub challenge: String,
    pub owner_incarnation: String,
    pub owner_pid: u32,
    pub owner_created: u64,
    pub owner_role: crate::local_client::OwnerRole,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Command {
    Business {
        kind: crate::client_protocol::CallKind,
    },
    ShowLibrary {
        document: Option<crate::local_service::InitialDocument>,
    },
    ShowCapture,
    LoginStartup,
    Shutdown,
    ToastActivation {
        opaque: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RequestHeader {
    pub request_id: String,
    pub owner_incarnation: String,
    pub command: Command,
    pub body_bytes: u64,
    pub secret_bytes: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReplyHeader {
    pub request_id: String,
    pub owner_incarnation: String,
    pub http_status: u16,
    pub content_type: String,
    pub body_bytes: u64,
}

pub(crate) fn canonical_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value)
        .is_ok_and(|id| id.get_version_num() == 4 && id.to_string() == value)
}

pub(crate) fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(crate) async fn write_header<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    value: &T,
) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    if bytes.is_empty() || bytes.len() > MAX_HEADER {
        return Err(invalid("Owner header exceeds its fixed budget"));
    }
    writer
        .write_u32_le(u32::try_from(bytes.len()).map_err(io::Error::other)?)
        .await?;
    writer.write_all(&bytes).await
}

pub(crate) async fn read_header<R: AsyncRead + Unpin, T: DeserializeOwned>(
    reader: &mut R,
) -> io::Result<T> {
    let length = reader.read_u32_le().await? as usize;
    if length == 0 || length > MAX_HEADER {
        return Err(invalid("Owner header length is invalid"));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).await?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

pub(crate) async fn read_body<R: AsyncRead + Unpin>(
    reader: &mut R,
    length: u64,
    maximum: usize,
) -> io::Result<Vec<u8>> {
    let length =
        usize::try_from(length).map_err(|_| invalid("Owner body length cannot be represented"))?;
    if length > maximum {
        return Err(invalid("Owner body exceeds its fixed budget"));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).await?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn byte_stream_framing_rejects_oversize_truncation_and_unknown_header_fields() {
        let (mut writer, mut reader) = tokio::io::duplex(128);
        let task = tokio::spawn(async move {
            writer.write_u32_le((MAX_HEADER + 1) as u32).await.unwrap();
        });
        assert!(read_header::<_, Hello>(&mut reader).await.is_err());
        task.await.unwrap();
        let (mut writer, mut reader) = tokio::io::duplex(128);
        let task = tokio::spawn(async move {
            writer.write_u32_le(30).await.unwrap();
            writer.write_all(b"short").await.unwrap();
        });
        assert!(read_header::<_, Hello>(&mut reader).await.is_err());
        task.await.unwrap();
        let (mut writer, mut reader) = tokio::io::duplex(128);
        let task = tokio::spawn(async move {
            write_header(&mut writer, &serde_json::json!({"version":1,"profileIdentity":"p","challenge":"c","token":"forbidden"})).await.unwrap();
        });
        assert!(read_header::<_, Hello>(&mut reader).await.is_err());
        task.await.unwrap();
    }
}
