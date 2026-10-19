// SPDX-License-Identifier: AGPL-3.0-only
//! Typed reading of a repository's published auto-configuration (`.a3s/autoconfig`).
//!
//! The file tells a client how to reach a repository: its name, the transport and the address. It is public
//! server metadata. It is **not** the encrypted per-user repository definition: local download folders, exclusion
//! rules, upload settings and similar stay in the user's own files, and nothing here supplies settings or addon
//! lists.
//!
//! The class names and version numbers below are **wire data** of the existing format (see
//! `docs/migration/FORMAT_INVENTORY.md`, section 3). They are only compared with the descriptors in the stream; no
//! class of that name is ever loaded or created, and they are not names of types in this project. A stream that
//! mentions any class or version outside this table is rejected.

use crate::error::{ParseError, ParseErrorKind};
use crate::gzip::gunzip;
use crate::stream::{Document, Limits, Object, Value, read_stream};

/// Wire descriptors accepted in an auto-configuration stream: (name, version number). Enum types and the shared
/// base type use the version numbers the format defines for them.
const KNOWN_CLASSES: [(&str, u64); 12] = [
    (
        "fr.soe.a3s.domain.repository.AutoConfig",
        0x258e_a3fb_4104_c73b,
    ),
    ("fr.soe.a3s.domain.AbstractProtocole", 0xf3b3_06f9_aa50_b9dd),
    ("fr.soe.a3s.domain.Http", 0xffff_ffff_ffff_ffec),
    ("fr.soe.a3s.domain.Sftp", 1),
    ("fr.soe.a3s.domain.Ftp", 0xe563_da9b_4902_3c24),
    ("fr.soe.a3s.domain.Webdav", 0xffff_ffff_ffff_ffe2),
    ("fr.soe.a3s.domain.Socks", 0xffff_ffff_ffff_ffd8),
    ("fr.soe.a3s.constant.ProtocolType", 0),
    ("fr.soe.a3s.constant.EncryptionMode", 0),
    ("java.lang.Enum", 0),
    (
        "fr.soe.a3s.domain.configration.FavoriteServer",
        0x4005_81f7_552a_9bb4,
    ),
    ("java.util.ArrayList", 0x7881_d21d_99c7_619d),
];

const AUTOCONFIG_CLASS: &str = "fr.soe.a3s.domain.repository.AutoConfig";
const MAX_NAME_CHARS: usize = 200;
const MAX_LOCATION_BYTES: usize = 2048;

/// How a repository is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerProtocol {
    /// FTP.
    Ftp,
    /// FTP over TLS.
    Ftps,
    /// SFTP (SSH).
    Sftp,
    /// Plain HTTP.
    Http,
    /// HTTP over TLS.
    Https,
    /// WebDAV over plain HTTP.
    HttpWebdav,
    /// WebDAV over TLS.
    HttpsWebdav,
}

impl ServerProtocol {
    /// URL scheme.
    #[must_use]
    pub const fn scheme(self) -> &'static str {
        match self {
            Self::Ftp => "ftp",
            Self::Ftps => "ftps",
            Self::Sftp => "sftp",
            Self::Http | Self::HttpWebdav => "http",
            Self::Https | Self::HttpsWebdav => "https",
        }
    }

    /// The port that is implied when none is written.
    #[must_use]
    pub const fn default_port(self) -> u16 {
        match self {
            Self::Ftp | Self::Ftps => 21,
            Self::Sftp => 22,
            Self::Http | Self::HttpWebdav => 80,
            Self::Https | Self::HttpsWebdav => 443,
        }
    }

    /// Whether the repository is accessed as WebDAV (the neutral model records only the address).
    #[must_use]
    pub const fn is_webdav(self) -> bool {
        matches!(self, Self::HttpWebdav | Self::HttpsWebdav)
    }
}

/// The public connection data of a repository from its auto-configuration. Contains no credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerAutoConfig {
    /// The repository name as published.
    pub repository_name: String,
    /// The transport.
    pub protocol: ServerProtocol,
    /// Host (and optional path) without scheme, port or credentials.
    pub location: String,
    /// An explicit port if one was published.
    pub port: Option<u16>,
    /// The published connection included a user name or password. They are **not** read; the user must enter
    /// credentials again.
    pub uses_credentials: bool,
    /// Number of favourite game servers listed in the file (not read).
    pub favorite_server_count: usize,
}

fn invalid(detail: &'static str) -> ParseError {
    ParseError::new(ParseErrorKind::InvalidValue, detail)
}

fn structure(detail: &'static str) -> ParseError {
    ParseError::new(ParseErrorKind::UnexpectedStructure, detail)
}

/// Parses the bytes of a `.a3s/autoconfig` file.
///
/// # Errors
/// [`ParseError`] for anything that is not a well-formed, supported auto-configuration within `limits`.
/// Errors never contain data from the input.
pub fn parse_autoconfig(bytes: &[u8], limits: &Limits) -> Result<ServerAutoConfig, ParseError> {
    let raw = gunzip(bytes, limits)?;
    let document = read_stream(&raw, limits)?;
    document.ensure_only(&KNOWN_CLASSES)?;
    let top = document
        .object(&document.top)
        .filter(|o| document.class_of(o).name == AUTOCONFIG_CLASS)
        .ok_or_else(|| structure("the file is not a repository auto-configuration"))?;
    extract(&document, top)
}

fn text_field<'a>(object: &'a Object, name: &str) -> Result<Option<&'a str>, ParseError> {
    match object.field(name) {
        Some(Value::Text(text)) => Ok(Some(text)),
        Some(Value::Null) => Ok(None),
        _ => Err(structure("a field has an unexpected type")),
    }
}

fn extract(document: &Document, top: &Object) -> Result<ServerAutoConfig, ParseError> {
    let repository_name = text_field(top, "repositoryName")?
        .filter(|name| !name.is_empty())
        .ok_or_else(|| invalid("the repository has no name"))?;
    if repository_name.chars().count() > MAX_NAME_CHARS
        || repository_name.chars().any(char::is_control)
    {
        return Err(invalid("the repository name is not usable"));
    }
    let favorite_server_count = match top.field("favoriteServers") {
        Some(Value::Null) | None => 0,
        Some(value) => document
            .object(value)
            .filter(|list| document.class_of(list).name == "java.util.ArrayList")
            .map(|list| list.items.len())
            .ok_or_else(|| structure("the favourite server list has an unexpected type"))?,
    };
    let protocol_object = top
        .field("protocole")
        .and_then(|value| document.object(value))
        .ok_or_else(|| structure("the file contains no connection data"))?;
    let class = document.class_of(protocol_object).name.as_str();
    let type_name = match protocol_object.field("protocolType") {
        Some(value) => document
            .object(value)
            .and_then(|constant| constant.enum_name.as_deref())
            .ok_or_else(|| structure("the transport type is missing"))?,
        None => return Err(structure("the transport type is missing")),
    };
    let protocol = match (class, type_name) {
        ("fr.soe.a3s.domain.Ftp", "FTP") => ServerProtocol::Ftp,
        ("fr.soe.a3s.domain.Ftp", "FTPS") => ServerProtocol::Ftps,
        ("fr.soe.a3s.domain.Sftp", "SFTP") => ServerProtocol::Sftp,
        ("fr.soe.a3s.domain.Http", "HTTP") => ServerProtocol::Http,
        ("fr.soe.a3s.domain.Http", "HTTPS") => ServerProtocol::Https,
        ("fr.soe.a3s.domain.Webdav", "HTTP_WEBDAV") => ServerProtocol::HttpWebdav,
        ("fr.soe.a3s.domain.Webdav", "HTTPS_WEBDAV") => ServerProtocol::HttpsWebdav,
        ("fr.soe.a3s.domain.Socks", _) => {
            return Err(ParseError::new(
                ParseErrorKind::UnsupportedProtocol,
                "the connection is a proxy type, not a repository transport",
            ));
        }
        _ => {
            return Err(ParseError::new(
                ParseErrorKind::UnsupportedProtocol,
                "the repository transport is not supported",
            ));
        }
    };
    let location = text_field(protocol_object, "url")?
        .ok_or_else(|| invalid("the repository has no address"))?;
    validate_location(location)?;
    let port = parse_port(text_field(protocol_object, "port")?)?;
    // The login is read only as "present"; the secret fields were never kept as text.
    let uses_credentials = ["login", "password"]
        .iter()
        .any(|name| matches!(protocol_object.field(name), Some(Value::Redacted(true))));
    Ok(ServerAutoConfig {
        repository_name: repository_name.to_owned(),
        protocol,
        location: location.to_owned(),
        port,
        uses_credentials,
        favorite_server_count,
    })
}

fn validate_location(location: &str) -> Result<(), ParseError> {
    let bad = || invalid("the repository address is not usable");
    if location.is_empty()
        || location.len() > MAX_LOCATION_BYTES
        || location.chars().any(|c| {
            c.is_control() || c.is_whitespace() || matches!(c, '@' | '\\' | '?' | '#' | '%')
        })
        || location.contains("://")
    {
        return Err(bad());
    }
    let host = location.split('/').next().unwrap_or_default();
    if host.is_empty() || (host.contains(':') && !host.starts_with('[')) {
        return Err(bad());
    }
    Ok(())
}

fn parse_port(port: Option<&str>) -> Result<Option<u16>, ParseError> {
    match port {
        None | Some("") => Ok(None),
        Some(text) if text.len() <= 5 && text.bytes().all(|b| b.is_ascii_digit()) => text
            .parse::<u16>()
            .ok()
            .filter(|p| *p != 0)
            .map(Some)
            .ok_or_else(|| invalid("the port is outside the valid range")),
        Some(_) => Err(invalid("the port is not a number")),
    }
}
