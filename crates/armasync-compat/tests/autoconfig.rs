// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the auto-configuration parser: the MIG-001 fixtures, every rejected fixture, crafted variants and
//! hostile values. All data is synthetic.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use armasync_compat::{Limits, ParseErrorKind, ServerProtocol, parse_autoconfig};
use common::{
    FTP, HTTP, Params, SFTP, SOCKS, Text, WEBDAV, autoconfig_file, autoconfig_stream, gzip,
    read_fixture,
};

fn parse(bytes: &[u8]) -> Result<armasync_compat::ServerAutoConfig, armasync_compat::ParseError> {
    parse_autoconfig(bytes, &Limits::AUTOCONFIG)
}

fn rejected(params: &Params) -> ParseErrorKind {
    match parse(&autoconfig_file(params)) {
        Ok(config) => panic!("accepted: {config:?}"),
        Err(error) => error.kind(),
    }
}

#[test]
fn reads_all_server_fixtures() {
    for scenario in ["empty", "small", "hostile-paths"] {
        let config = parse(&read_fixture(&format!(
            "valid/server/{scenario}/.a3s/autoconfig"
        )))
        .unwrap();
        assert_eq!(config.repository_name, "Example HTTP", "{scenario}");
        assert_eq!(config.protocol, ServerProtocol::Https);
        assert_eq!(config.location, "repo.example.invalid");
        assert_eq!(config.port, Some(443));
        assert!(config.uses_credentials);
        assert_eq!(config.favorite_server_count, 1);
    }
}

#[test]
fn secrets_never_reach_the_result_or_its_debug_output() {
    for scenario in ["empty", "small", "hostile-paths"] {
        let config = parse(&read_fixture(&format!(
            "valid/server/{scenario}/.a3s/autoconfig"
        )))
        .unwrap();
        let shown = format!("{config:?}");
        for secret in [
            "fixture-user",
            "fixture-password-not-a-secret",
            "fixture-server-password-not-a-secret",
        ] {
            assert!(!shown.contains(secret), "{secret} leaked");
        }
    }
    let params = Params {
        login: "synthetic-login-value".into(),
        password: "synthetic-secret-value".into(),
        favorites: 2,
        ..Params::default()
    };
    let config = parse(&autoconfig_file(&params)).unwrap();
    let shown = format!("{config:?}");
    assert!(!shown.contains("synthetic-login-value") && !shown.contains("synthetic-secret-value"));
    assert!(!shown.contains("fixture-favorite-password"));
    assert_eq!(config.favorite_server_count, 2);
    assert!(config.uses_credentials);
}

#[test]
fn credentials_flag_follows_presence_only() {
    let none = parse(&autoconfig_file(&Params::default())).unwrap();
    assert!(!none.uses_credentials);
    let empty = Params {
        login: "".into(),
        password: "".into(),
        ..Params::default()
    };
    assert!(!parse(&autoconfig_file(&empty)).unwrap().uses_credentials);
    let password_only = Params {
        password: "x".into(),
        ..Params::default()
    };
    assert!(
        parse(&autoconfig_file(&password_only))
            .unwrap()
            .uses_credentials
    );
}

#[test]
fn supported_transports_map_to_schemes() {
    let cases = [
        (FTP, "FTP", ServerProtocol::Ftp, "ftp", 21),
        (FTP, "FTPS", ServerProtocol::Ftps, "ftps", 21),
        (SFTP, "SFTP", ServerProtocol::Sftp, "sftp", 22),
        (HTTP, "HTTP", ServerProtocol::Http, "http", 80),
        (HTTP, "HTTPS", ServerProtocol::Https, "https", 443),
        (
            WEBDAV,
            "HTTP_WEBDAV",
            ServerProtocol::HttpWebdav,
            "http",
            80,
        ),
        (
            WEBDAV,
            "HTTPS_WEBDAV",
            ServerProtocol::HttpsWebdav,
            "https",
            443,
        ),
    ];
    for (class, constant, expected, scheme, port) in cases {
        let params = Params {
            class,
            protocol_type: constant,
            port: Text::Null,
            ..Params::default()
        };
        let config = parse(&autoconfig_file(&params)).unwrap();
        assert_eq!(config.protocol, expected, "{constant}");
        assert_eq!(config.protocol.scheme(), scheme);
        assert_eq!(config.protocol.default_port(), port);
        assert_eq!(config.protocol.is_webdav(), constant.contains("WEBDAV"));
        assert_eq!(config.port, None);
    }
}

#[test]
fn proxy_and_unknown_transports_are_unsupported_not_guessed() {
    for (class, constant) in [
        (SOCKS, "SOCKS5"),
        (SOCKS, "SOCKS4"),
        (HTTP, "A3S"),
        (SFTP, "HTTPS"),
    ] {
        let params = Params {
            class,
            protocol_type: constant,
            ..Params::default()
        };
        assert_eq!(
            rejected(&params),
            ParseErrorKind::UnsupportedProtocol,
            "{constant}"
        );
    }
}

#[test]
fn address_and_port_are_validated() {
    let with_url = |url: &str| Params {
        url: url.into(),
        ..Params::default()
    };
    let with_port = |port: &str| Params {
        port: port.into(),
        ..Params::default()
    };
    let config = parse(&autoconfig_file(&with_url(
        "repo.example.invalid/arma/repo",
    )))
    .unwrap();
    assert_eq!(config.location, "repo.example.invalid/arma/repo");
    assert!(parse(&autoconfig_file(&with_url("[2001:db8::1]/arma"))).is_ok());
    for url in [
        "user:pass@repo.example.invalid",
        "repo.example.invalid@evil.example.invalid",
        "https://repo.example.invalid",
        "repo.example.invalid:8443",
        "repo example",
        "repo.example.invalid\\share",
        "repo.example.invalid/?q=1",
        "repo.example.invalid/#x",
        "repo.example.invalid/%2e%2e",
        "/only/a/path",
        "",
    ] {
        assert_eq!(
            rejected(&with_url(url)),
            ParseErrorKind::InvalidValue,
            "{url:?}"
        );
    }
    assert_eq!(
        rejected(&Params {
            url: Text::Null,
            ..Params::default()
        }),
        ParseErrorKind::InvalidValue
    );
    assert_eq!(
        parse(&autoconfig_file(&with_port("8443"))).unwrap().port,
        Some(8443)
    );
    assert_eq!(parse(&autoconfig_file(&with_port(""))).unwrap().port, None);
    for port in ["0", "65536", "99999", "-1", "80a", "123456", " 80"] {
        assert_eq!(
            rejected(&with_port(port)),
            ParseErrorKind::InvalidValue,
            "{port:?}"
        );
    }
}

#[test]
fn repository_name_is_required_and_checked() {
    for name in [
        Text::Null,
        "".into(),
        "line\nbreak".into(),
        "x".repeat(201).as_str().into(),
    ] {
        let params = Params {
            repository_name: name,
            ..Params::default()
        };
        assert_eq!(rejected(&params), ParseErrorKind::InvalidValue);
    }
    let unicode = Params {
        repository_name: "Épreuve \u{1F680}".into(),
        ..Params::default()
    };
    assert_eq!(
        parse(&autoconfig_file(&unicode)).unwrap().repository_name,
        "Épreuve \u{1F680}"
    );
}

#[test]
fn unknown_classes_are_rejected_without_being_loaded() {
    let params = Params {
        unknown_class: true,
        ..Params::default()
    };
    assert_eq!(rejected(&params), ParseErrorKind::UnknownClass);
    // The same class name with a different version number is a different class.
    let mut other = HTTP;
    other.1 = 7;
    let params = Params {
        class: other,
        ..Params::default()
    };
    assert_eq!(rejected(&params), ParseErrorKind::UnknownClass);
}

#[test]
fn dangling_reference_is_malformed() {
    let params = Params {
        repository_name: Text::Raw(vec![0x71, 0x00, 0x7E, 0x7F, 0xFF]),
        ..Params::default()
    };
    assert_eq!(rejected(&params), ParseErrorKind::Malformed);
}

#[test]
fn rejects_every_invalid_fixture() {
    let dir = common::fixture("invalid");
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "zip") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(parse(&bytes).is_err(), "{} was accepted", path.display());
        checked += 1;
    }
    assert_eq!(
        checked, 15,
        "the invalid fixture set changed; review this test"
    );
}

#[test]
fn invalid_fixtures_fail_for_the_documented_reason() {
    let kind = |name: &str| {
        parse(&read_fixture(&format!("invalid/{name}")))
            .unwrap_err()
            .kind()
    };
    assert_eq!(kind("empty.bin"), ParseErrorKind::NotCompressed);
    assert_eq!(kind("not-gzip.bin"), ParseErrorKind::NotCompressed);
    assert_eq!(kind("gzip-bad-crc.bin"), ParseErrorKind::CompressionCorrupt);
    assert_eq!(
        kind("gzip-truncated.bin"),
        ParseErrorKind::CompressionCorrupt
    );
    assert_eq!(kind("gzip-expands-64mib.bin"), ParseErrorKind::TooLarge);
    assert_eq!(
        kind("gzip-not-serialization.bin"),
        ParseErrorKind::NotSerialization
    );
    assert_eq!(
        kind("serialization-bad-version.bin"),
        ParseErrorKind::NotSerialization
    );
    assert_eq!(
        kind("serialization-truncated.bin"),
        ParseErrorKind::Truncated
    );
    assert_eq!(
        kind("array-declared-1gib.bin"),
        ParseErrorKind::LimitExceeded
    );
    assert_eq!(kind("depth-150.bin"), ParseErrorKind::LimitExceeded);
    assert_eq!(
        kind("superclass-chain-150.bin"),
        ParseErrorKind::LimitExceeded
    );
    assert_eq!(kind("superclass-cycle.bin"), ParseErrorKind::Malformed);
    assert_eq!(kind("class-unknown.bin"), ParseErrorKind::UnknownClass);
    assert_eq!(kind("class-uid-mismatch.bin"), ParseErrorKind::UnknownClass);
}

#[test]
fn other_metadata_files_are_not_accepted_as_autoconfig() {
    for file in [
        "valid/server/small/.a3s/serverinfo",
        "valid/server/small/.a3s/sync",
        "valid/server/small/.a3s/changelogs",
        "valid/server/small/.a3s/events",
        "valid/local/a3s.cfg",
        "valid/local/profiles/Alpha.a3s.profile",
        "valid/local/autoconfig-export/export.a3s.autoconfig",
        "valid/local/repositories/Example-Http.a3s.repository",
    ] {
        assert!(parse(&read_fixture(file)).is_err(), "{file}");
    }
}

#[test]
fn limits_are_enforced() {
    let file = read_fixture("valid/server/small/.a3s/autoconfig");
    let tiny_input = Limits {
        max_compressed: 10,
        ..Limits::AUTOCONFIG
    };
    assert_eq!(
        parse_autoconfig(&file, &tiny_input).unwrap_err().kind(),
        ParseErrorKind::TooLarge
    );
    let tiny_output = Limits {
        max_decompressed: 50,
        ..Limits::AUTOCONFIG
    };
    assert_eq!(
        parse_autoconfig(&file, &tiny_output).unwrap_err().kind(),
        ParseErrorKind::TooLarge
    );
    let shallow = Limits {
        max_depth: 2,
        ..Limits::AUTOCONFIG
    };
    assert_eq!(
        parse_autoconfig(&file, &shallow).unwrap_err().kind(),
        ParseErrorKind::LimitExceeded
    );
    let few_handles = Limits {
        max_handles: 3,
        ..Limits::AUTOCONFIG
    };
    assert_eq!(
        parse_autoconfig(&file, &few_handles).unwrap_err().kind(),
        ParseErrorKind::LimitExceeded
    );
}

#[test]
fn trailing_data_is_rejected() {
    let mut stream = autoconfig_stream(&Params::default());
    stream.push(0x70);
    assert_eq!(
        parse(&gzip(&stream)).unwrap_err().kind(),
        ParseErrorKind::Malformed
    );
    let mut file = autoconfig_file(&Params::default());
    file.extend_from_slice(b"extra");
    assert_eq!(
        parse(&file).unwrap_err().kind(),
        ParseErrorKind::CompressionCorrupt
    );
}

#[test]
fn every_truncation_and_corruption_is_handled_without_panic() {
    let stream = autoconfig_stream(&Params {
        favorites: 1,
        login: "l".into(),
        password: "p".into(),
        ..Params::default()
    });
    assert!(parse(&gzip(&stream)).is_ok());
    for end in 0..stream.len() {
        assert!(
            parse(&gzip(&stream[..end])).is_err(),
            "prefix {end} accepted"
        );
    }
    for index in 0..stream.len() {
        let mut changed = stream.clone();
        changed[index] ^= 0xFF;
        // The outcome may be an error or a different valid document; it must never panic.
        let _ = parse(&gzip(&changed));
    }
}

#[test]
fn error_messages_contain_no_input_data() {
    let params = Params {
        repository_name: "marker-name-xyz".into(),
        url: "marker-user@marker-host.example.invalid".into(),
        password: "marker-password-xyz".into(),
        ..Params::default()
    };
    let error = parse(&autoconfig_file(&params)).unwrap_err();
    let shown = format!("{error} {error:?} {}", error.detail());
    assert!(!shown.contains("marker"), "{shown}");
}
