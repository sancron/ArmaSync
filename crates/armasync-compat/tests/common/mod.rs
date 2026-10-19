// SPDX-License-Identifier: AGPL-3.0-only
//! Test helper: writes small serialization streams for auto-configuration files, so edge cases (other transports,
//! missing fields, hostile values) can be tested without real server files. All data is synthetic.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::PathBuf;

use flate2::Compression;
use flate2::write::GzEncoder;

pub const AUTOCONFIG: (&str, u64) = (
    "fr.soe.a3s.domain.repository.AutoConfig",
    0x258e_a3fb_4104_c73b,
);
pub const ABSTRACT_PROTOCOL: (&str, u64) =
    ("fr.soe.a3s.domain.AbstractProtocole", 0xf3b3_06f9_aa50_b9dd);
pub const HTTP: (&str, u64) = ("fr.soe.a3s.domain.Http", 0xffff_ffff_ffff_ffec);
pub const SFTP: (&str, u64) = ("fr.soe.a3s.domain.Sftp", 1);
pub const FTP: (&str, u64) = ("fr.soe.a3s.domain.Ftp", 0xe563_da9b_4902_3c24);
pub const WEBDAV: (&str, u64) = ("fr.soe.a3s.domain.Webdav", 0xffff_ffff_ffff_ffe2);
pub const SOCKS: (&str, u64) = ("fr.soe.a3s.domain.Socks", 0xffff_ffff_ffff_ffd8);

/// Path of a MIG-001 fixture file below `fixtures/legacy-formats/generated`.
pub fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/legacy-formats/generated")
        .join(relative)
}

pub fn read_fixture(relative: &str) -> Vec<u8> {
    std::fs::read(fixture(relative)).unwrap_or_else(|e| panic!("fixture {relative}: {e}"))
}

pub fn gzip(raw: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(raw).unwrap();
    encoder.finish().unwrap()
}

/// A field of an auto-configuration as the stream stores it.
#[derive(Clone)]
pub enum Text {
    Null,
    Str(String),
    /// Raw bytes written instead of a string element (for references and other oddities).
    Raw(Vec<u8>),
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self::Str(value.to_owned())
    }
}

#[derive(Clone)]
pub struct Params {
    pub repository_name: Text,
    pub class: (&'static str, u64),
    pub protocol_type: &'static str,
    pub url: Text,
    pub port: Text,
    pub login: Text,
    pub password: Text,
    pub favorites: usize,
    /// Adds a field of an unknown class after the known ones.
    pub unknown_class: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            repository_name: "Synthetic Repo".into(),
            class: HTTP,
            protocol_type: "HTTPS",
            url: "repo.example.invalid".into(),
            port: "443".into(),
            login: Text::Null,
            password: Text::Null,
            favorites: 0,
            unknown_class: false,
        }
    }
}

#[derive(Default)]
pub struct W {
    pub bytes: Vec<u8>,
}

impl W {
    pub fn new() -> Self {
        let mut w = Self::default();
        w.bytes.extend_from_slice(&[0xAC, 0xED, 0x00, 0x05]);
        w
    }

    pub fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }

    fn u16(&mut self, v: u16) {
        self.bytes.extend_from_slice(&v.to_be_bytes());
    }

    fn i32(&mut self, v: i32) {
        self.bytes.extend_from_slice(&v.to_be_bytes());
    }

    fn u64(&mut self, v: u64) {
        self.bytes.extend_from_slice(&v.to_be_bytes());
    }

    /// Writes `s` as modified UTF-8 (UTF-16 units of up to three bytes, NUL as `C0 80`).
    pub fn utf(&mut self, s: &str) {
        let mut encoded = Vec::new();
        for unit in s.encode_utf16() {
            match unit {
                0 => encoded.extend_from_slice(&[0xC0, 0x80]),
                1..=0x7F => encoded.push(u8::try_from(unit).unwrap()),
                0x80..=0x7FF => encoded.extend_from_slice(&[
                    0xC0 | u8::try_from(unit >> 6).unwrap(),
                    0x80 | u8::try_from(unit & 0x3F).unwrap(),
                ]),
                _ => encoded.extend_from_slice(&[
                    0xE0 | u8::try_from(unit >> 12).unwrap(),
                    0x80 | u8::try_from((unit >> 6) & 0x3F).unwrap(),
                    0x80 | u8::try_from(unit & 0x3F).unwrap(),
                ]),
            }
        }
        self.u16(u16::try_from(encoded.len()).unwrap());
        self.bytes.extend_from_slice(&encoded);
    }

    pub fn string(&mut self, s: &str) {
        self.u8(0x74);
        self.utf(s);
    }

    pub fn null(&mut self) {
        self.u8(0x70);
    }

    pub fn text(&mut self, text: &Text) {
        match text {
            Text::Null => self.null(),
            Text::Str(s) => self.string(s),
            Text::Raw(raw) => self.bytes.extend_from_slice(raw),
        }
    }

    /// Class descriptor with fields `(type code, name, type string for object fields)`.
    pub fn class_desc(
        &mut self,
        class: (&str, u64),
        flags: u8,
        fields: &[(u8, &str, &str)],
        sup: Option<&dyn Fn(&mut W)>,
    ) {
        self.u8(0x72);
        self.utf(class.0);
        self.u64(class.1);
        self.u8(flags);
        self.u16(u16::try_from(fields.len()).unwrap());
        for (code, name, ty) in fields {
            self.u8(*code);
            self.utf(name);
            if matches!(*code, b'L' | b'[') {
                self.string(ty);
            }
        }
        self.u8(0x78);
        match sup {
            Some(write) => write(self),
            None => self.null(),
        }
    }

    pub fn enum_constant(&mut self, type_name: &str, constant: &str) {
        self.u8(0x7E);
        let name = type_name.to_owned();
        self.class_desc(
            (&name, 0),
            0x12,
            &[],
            Some(&|w: &mut W| {
                w.class_desc(("java.lang.Enum", 0), 0x12, &[], None);
            }),
        );
        self.string(constant);
    }
}

/// A complete, uncompressed serialization stream for `params`.
pub fn autoconfig_stream(params: &Params) -> Vec<u8> {
    let mut w = W::new();
    w.u8(0x73); // TC_OBJECT
    let mut top_fields = vec![
        (b'L', "favoriteServers", "Ljava/util/ArrayList;"),
        (b'L', "protocole", "Lfr/soe/a3s/domain/AbstractProtocole;"),
        (b'L', "repositoryName", "Ljava/lang/String;"),
    ];
    if params.unknown_class {
        top_fields.push((b'L', "extra", "Ljava/lang/Object;"));
    }
    w.class_desc(AUTOCONFIG, 0x02, &top_fields, None);
    // favoriteServers: a list object
    w.u8(0x73);
    w.class_desc(
        ("java.util.ArrayList", 0x7881_d21d_99c7_619d),
        0x03,
        &[(b'I', "size", "")],
        None,
    );
    w.i32(i32::try_from(params.favorites).unwrap());
    w.u8(0x77); // TC_BLOCKDATA
    w.u8(4);
    w.i32(i32::try_from(params.favorites).unwrap());
    for _ in 0..params.favorites {
        w.u8(0x73);
        w.class_desc(
            (
                "fr.soe.a3s.domain.configration.FavoriteServer",
                0x4005_81f7_552a_9bb4,
            ),
            0x02,
            &[
                (b'I', "port", ""),
                (b'Z', "selected", ""),
                (b'L', "ipAddress", "Ljava/lang/String;"),
                (b'L', "name", "Ljava/lang/String;"),
                (b'L', "password", "Ljava/lang/String;"),
            ],
            None,
        );
        w.i32(2302);
        w.u8(0);
        w.string("192.0.2.20");
        w.string("Game server");
        w.string("fixture-favorite-password");
    }
    w.u8(0x78);
    // protocole
    w.u8(0x73);
    w.class_desc(
        params.class,
        0x02,
        &[],
        Some(&|w: &mut W| {
            w.class_desc(
                ABSTRACT_PROTOCOL,
                0x02,
                &[
                    (b'Z', "validateSSLCertificate", ""),
                    (b'L', "login", "Ljava/lang/String;"),
                    (b'L', "password", "Ljava/lang/String;"),
                    (b'L', "port", "Ljava/lang/String;"),
                    (b'L', "protocolType", "Lfr/soe/a3s/constant/ProtocolType;"),
                    (b'L', "url", "Ljava/lang/String;"),
                ],
                None,
            );
        }),
    );
    w.u8(1); // validateSSLCertificate
    w.text(&params.login);
    w.text(&params.password);
    w.text(&params.port);
    w.enum_constant("fr.soe.a3s.constant.ProtocolType", params.protocol_type);
    w.text(&params.url);
    // repositoryName
    w.text(&params.repository_name);
    if params.unknown_class {
        w.u8(0x73);
        w.class_desc(("com.example.Unknown", 1), 0x02, &[], None);
    }
    w.bytes
}

pub fn autoconfig_file(params: &Params) -> Vec<u8> {
    gzip(&autoconfig_stream(params))
}

// ---- file tree (`.a3s/sync`) builder ---------------------------------------------------------------------------

pub const TREE_DIRECTORY: (&str, u64) = (
    "fr.soe.a3s.domain.repository.SyncTreeDirectory",
    0xd85f_eb3c_784f_59f8,
);
pub const TREE_FILE: (&str, u64) = (
    "fr.soe.a3s.domain.repository.SyncTreeLeaf",
    0x7ace_d84d_2427_12d7,
);

#[derive(Clone)]
pub struct TestFile {
    pub name: String,
    pub sha1: Text,
    pub size: i64,
    pub compressed: bool,
    pub compressed_size: i64,
}

impl TestFile {
    pub fn new(name: &str, size: i64) -> Self {
        Self {
            name: name.to_owned(),
            sha1: "0123456789abcdef0123456789abcdef01234567".into(),
            size,
            compressed: false,
            compressed_size: 0,
        }
    }
}

#[derive(Clone)]
pub enum Item {
    Dir(TestDir),
    File(TestFile),
    Raw(Vec<u8>),
}

#[derive(Clone)]
pub struct TestDir {
    pub name: String,
    pub hidden: bool,
    pub addon: bool,
    pub items: Vec<Item>,
}

impl TestDir {
    pub fn new(name: &str, items: Vec<Item>) -> Self {
        Self {
            name: name.to_owned(),
            hidden: false,
            addon: false,
            items,
        }
    }
}

impl W {
    fn i64(&mut self, v: i64) {
        self.bytes.extend_from_slice(&v.to_be_bytes());
    }

    fn tree_file(&mut self, f: &TestFile) {
        self.u8(0x73);
        self.class_desc(
            TREE_FILE,
            0x02,
            &[
                (b'Z', "compressed", ""),
                (b'Z', "deleted", ""),
                (b'Z', "updated", ""),
                (b'J', "compressedSize", ""),
                (b'J', "size", ""),
                (b'L', "name", "Ljava/lang/String;"),
                (
                    b'L',
                    "parent",
                    "Lfr/soe/a3s/domain/repository/SyncTreeDirectory;",
                ),
                (b'L', "sha1", "Ljava/lang/String;"),
            ],
            None,
        );
        self.u8(u8::from(f.compressed));
        self.u8(0);
        self.u8(0);
        self.i64(f.compressed_size);
        self.i64(f.size);
        self.string(&f.name);
        self.null();
        self.text(&f.sha1);
    }

    pub fn tree_dir(&mut self, d: &TestDir) {
        self.u8(0x73);
        self.class_desc(
            TREE_DIRECTORY,
            0x02,
            &[
                (b'Z', "deleted", ""),
                (b'Z', "hidden", ""),
                (b'Z', "markAsAddon", ""),
                (b'Z', "updated", ""),
                (b'L', "list", "Ljava/util/ArrayList;"),
                (b'L', "name", "Ljava/lang/String;"),
                (
                    b'L',
                    "parent",
                    "Lfr/soe/a3s/domain/repository/SyncTreeDirectory;",
                ),
            ],
            None,
        );
        self.u8(0);
        self.u8(u8::from(d.hidden));
        self.u8(u8::from(d.addon));
        self.u8(0);
        self.u8(0x73);
        self.class_desc(
            ("java.util.ArrayList", 0x7881_d21d_99c7_619d),
            0x03,
            &[(b'I', "size", "")],
            None,
        );
        let count = i32::try_from(d.items.len()).unwrap();
        self.bytes.extend_from_slice(&count.to_be_bytes());
        self.u8(0x77);
        self.u8(4);
        self.bytes.extend_from_slice(&count.to_be_bytes());
        for item in &d.items {
            match item {
                Item::Dir(child) => self.tree_dir(child),
                Item::File(file) => self.tree_file(file),
                Item::Raw(raw) => self.bytes.extend_from_slice(raw),
            }
        }
        self.u8(0x78);
        self.string(&d.name);
        self.null();
    }
}

/// An uncompressed `.a3s/sync` stream for `root`.
pub fn tree_stream(root: &TestDir) -> Vec<u8> {
    let mut w = W::new();
    w.tree_dir(root);
    w.bytes
}

pub fn tree_file(root: &TestDir) -> Vec<u8> {
    gzip(&tree_stream(root))
}
