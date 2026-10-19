// SPDX-License-Identifier: AGPL-3.0-only
//! ArmaSync compatibility parsers (MIG-022, `docs/migration/COMPATIBILITY.md`).
//!
//! Bounded, data-only readers for metadata that existing repository servers publish. The crate performs no I/O and
//! no network access: callers hand it bytes. It does not load or run anything named in the data, keeps no
//! credentials (secret fields are skipped while reading), and its errors never contain input data. Class names and
//! version numbers of the existing wire format appear only in the narrow table documented in the `autoconfig` module.
//!
//! Scope today: the repository auto-configuration (`.a3s/autoconfig`). It is public server metadata, not the
//! encrypted per-user repository definition and not a source of settings or addon lists.

mod autoconfig;
mod error;
mod gzip;
mod stream;

pub use autoconfig::{ServerAutoConfig, ServerProtocol, parse_autoconfig};
pub use error::{ParseError, ParseErrorKind};
pub use stream::{Limits, is_secret_field};
