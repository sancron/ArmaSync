// SPDX-License-Identifier: AGPL-3.0-only
//! Plain-language notices about what an import does and does not cover.
//!
//! Both limited import paths (the local legacy export of MIG-021 and the server auto-configuration of MIG-022)
//! leave out things users may expect: repository definitions kept in encrypted files, settings, addon lists and
//! passwords. The notices say so in ordinary words and carry stable codes. The CLI prints them; a future GUI reads
//! the same list (English and German texts), so both frontends tell the same story. The notices are data: they
//! contain no names, addresses or other content from the imported files.

use super::document::ImportNote;
use super::server::SERVER_AUTOCONFIG_FORMAT;

/// `source.format` written by the offline exporter of MIG-021.
pub const LOCAL_FILES_FORMAT: &str = "legacy-local-files";

/// How a notice should be presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Information about what was done.
    Info,
    /// A limit the user should know about: something expected is not part of the result.
    Limitation,
}

impl Severity {
    /// Stable name for JSON.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Limitation => "limitation",
        }
    }
}

/// One notice with its texts. Frontends must not rely on the wording, only on the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Notice {
    /// Stable code.
    pub code: &'static str,
    /// Presentation hint (frontends must not rely on colour alone to convey it).
    pub severity: Severity,
    /// English text.
    pub en: &'static str,
    /// German text.
    pub de: &'static str,
}

const LOCAL_SCOPE: Notice = Notice {
    code: "local-import-scope",
    severity: Severity::Limitation,
    en: "This import only contains profile names and the names of the repositories they refer to, taken from unencrypted local files. Repository addresses, settings, addon lists, start options and passwords are not part of it.",
    de: "Dieser Import enthält nur Profilnamen und die Namen der Repositories, auf die sie verweisen, aus unverschlüsselten lokalen Dateien. Repository-Adressen, Einstellungen, Addon-Listen, Startoptionen und Kennwörter sind nicht enthalten.",
};

const LOCAL_ENCRYPTED: Notice = Notice {
    code: "encrypted-repositories-not-read",
    severity: Severity::Limitation,
    en: "Repository files ending in .a3s.repository are encrypted and were not read, so those repositories' addresses and local settings are missing. Whether and how they can be read is still undecided; your original files are untouched.",
    de: "Repository-Dateien mit der Endung .a3s.repository sind verschlüsselt und wurden nicht gelesen. Deshalb fehlen die Adressen und lokalen Einstellungen dieser Repositories. Ob und wie sie gelesen werden können, ist noch nicht entschieden; Ihre Originaldateien bleiben unverändert.",
};

const LOCAL_UNRESOLVED: Notice = Notice {
    code: "profile-repositories-missing",
    severity: Severity::Limitation,
    en: "Some profiles refer to repositories that are not in ArmaSync yet. Those references are left out of the profile. Add the repositories, then import the profile file again and choose to replace the existing profile.",
    de: "Einige Profile verweisen auf Repositories, die in ArmaSync noch nicht vorhanden sind. Diese Verweise werden im Profil ausgelassen. Fügen Sie die Repositories hinzu, importieren Sie die Profildatei danach erneut und wählen Sie, das vorhandene Profil zu ersetzen.",
};

const SERVER_SCOPE: Notice = Notice {
    code: "server-metadata-scope",
    severity: Severity::Info,
    en: "Only the connection data that the server publishes was imported: the repository name and its address. Passwords are never read from this file; enter them again if the repository needs them.",
    de: "Importiert wurden nur die Verbindungsdaten, die der Server veröffentlicht: Name und Adresse des Repositorys. Kennwörter werden aus dieser Datei nie gelesen; geben Sie sie bei Bedarf neu ein.",
};

const SERVER_NOT_LOCAL: Notice = Notice {
    code: "server-metadata-not-local-settings",
    severity: Severity::Limitation,
    en: "This does not replace your own repository files. Settings that were stored only there (download folders, exclusions, upload settings), your application settings, profiles and addon lists are not part of the server's data and were not imported.",
    de: "Dies ersetzt Ihre eigenen Repository-Dateien nicht. Einstellungen, die nur dort gespeichert waren (Download-Ordner, Ausschlüsse, Upload-Einstellungen), Ihre Programmeinstellungen, Profile und Addon-Listen gehören nicht zu den Daten des Servers und wurden nicht importiert.",
};

const SERVER_FAVORITES: Notice = Notice {
    code: "server-favorite-servers-skipped",
    severity: Severity::Info,
    en: "The file also lists favorite game servers. They were not imported.",
    de: "Die Datei enthält außerdem Favoriten-Spieleserver. Diese wurden nicht importiert.",
};

const SERVER_WEBDAV: Notice = Notice {
    code: "server-webdav-address-only",
    severity: Severity::Info,
    en: "The server publishes this repository as WebDAV. Only its web address was imported; ArmaSync does not use WebDAV yet.",
    de: "Der Server veröffentlicht dieses Repository als WebDAV. Importiert wurde nur die Webadresse; ArmaSync nutzt WebDAV noch nicht.",
};

/// The notices that apply to an import. `source_format` is the document's `source.format`; `notes` are the
/// document's notes; `has_unresolved_references` tells whether the plan left profile references out.
#[must_use]
pub fn scope_notices(
    source_format: &str,
    notes: &[ImportNote],
    has_unresolved_references: bool,
) -> Vec<Notice> {
    let has_note = |code: &str| notes.iter().any(|n| n.code == code);
    let mut list = Vec::new();
    match source_format {
        LOCAL_FILES_FORMAT => {
            list.push(LOCAL_SCOPE);
            if has_note("skipped-encrypted-repository") {
                list.push(LOCAL_ENCRYPTED);
            }
            if has_unresolved_references {
                list.push(LOCAL_UNRESOLVED);
            }
        }
        SERVER_AUTOCONFIG_FORMAT => {
            list.push(SERVER_SCOPE);
            list.push(SERVER_NOT_LOCAL);
            if has_note("skipped-favorite-servers") {
                list.push(SERVER_FAVORITES);
            }
            if has_note("webdav-recorded-as-address") {
                list.push(SERVER_WEBDAV);
            }
        }
        _ => {}
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(code: &str) -> ImportNote {
        ImportNote {
            code: code.to_owned(),
            subject: None,
            count: Some(1),
        }
    }

    #[test]
    fn local_import_always_states_its_scope() {
        let codes = |notices: Vec<Notice>| notices.iter().map(|n| n.code).collect::<Vec<_>>();
        assert_eq!(
            codes(scope_notices(LOCAL_FILES_FORMAT, &[], false)),
            ["local-import-scope"]
        );
        assert_eq!(
            codes(scope_notices(
                LOCAL_FILES_FORMAT,
                &[note("skipped-encrypted-repository")],
                true
            )),
            [
                "local-import-scope",
                "encrypted-repositories-not-read",
                "profile-repositories-missing"
            ]
        );
    }

    #[test]
    fn server_import_never_claims_to_replace_local_files() {
        let notices = scope_notices(
            SERVER_AUTOCONFIG_FORMAT,
            &[note("skipped-favorite-servers")],
            false,
        );
        let codes: Vec<_> = notices.iter().map(|n| n.code).collect();
        assert_eq!(
            codes,
            [
                "server-metadata-scope",
                "server-metadata-not-local-settings",
                "server-favorite-servers-skipped"
            ]
        );
    }

    #[test]
    fn unknown_sources_have_no_claims() {
        assert!(scope_notices("something-else", &[], true).is_empty());
    }

    #[test]
    fn texts_are_complete_and_free_of_legacy_branding() {
        for notice in [
            LOCAL_SCOPE,
            LOCAL_ENCRYPTED,
            LOCAL_UNRESOLVED,
            SERVER_SCOPE,
            SERVER_NOT_LOCAL,
            SERVER_FAVORITES,
            SERVER_WEBDAV,
        ] {
            assert!(
                notice.en.len() > 30 && notice.de.len() > 30,
                "{}",
                notice.code
            );
            assert!(
                notice
                    .code
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'-')
            );
            for text in [notice.en, notice.de] {
                assert!(!text.to_lowercase().contains("fr.soe"));
            }
        }
    }
}
