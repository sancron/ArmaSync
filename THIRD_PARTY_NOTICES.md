# Herkunft und Drittbestandteile

**Vorläufiges Herkunftsregister**, kein abgeschlossener Lizenzscan. Neue Marke/Publisher: **Lynxnode.it**; Produkt: **ArmaSync**.

| Bestandteil | Beobachteter Nachweis | Behandlung |
| --- | --- | --- |
| Java-Anwendung | `legacy` bei `b7753cea18b2ac2cc57e29f926901997a4de8570`, `LICENCE.md`, `main/ArmA3Sync.java` | GPLv3 erhalten; ursprünglicher Autor Major_Shepard / Sons of Exiled und weitere Mitwirkende als Herkunft dokumentiert. |
| Jazsync / Jarsync / GNU-Crypto-Ableitungen | Header in `jazsync/` und `jazsyncmake/` | GPL-2.0-or-later; u. a. Tomas Hlavnicka, Casey Marshall und Free Software Foundation. Portierung gesondert prüfen. |
| Fork-Wartung | README/Credits auf `legacy`, Git-Verlauf | Unter anderem squadalpha und vPzBrig21; daraus keine neue Publisheridentität ableiten. |
| Apache Commons, HttpComponents, MINA SSHD | `build.gradle` | Lizenztexte über aufgelöste Artefakte samt Transitiven prüfen; keine automatische Rust-Übernahme. |
| FlatLaf / Extras / JSVG / Sardine | `build.gradle`, Artefakte | Lizenz und Nutzung prüfen. |
| JTattoo, JUnique, JShortcut | lokale JARs unter `resources/lib/` | Provenienz/Lizenz prüfen; nicht als freigegeben voraussetzen. |
| Geist / Inter | `src/main/resources/fr/soe/a3s/ui/fonts/*OFL.txt` | OFL-Texte bei Wiederverwendung beilegen; Fonts nicht umbenennen. |
| Alte Logos / SVGs / Installericons | `installer/assets/`, Icon-Ressourcen | Herkunft prüfen; unabhängig erstellte Lynxnode.it-Assets vorsehen. |
| Arma 3, DLC/CDLC und externe Tools | Spieleintegration | Sachlich erforderliche Bezeichnungen erhalten; keine Markenrechte behaupten. |

## Rust-Abhängigkeiten des HTTP-Adapters (MIG-030B, Stand 2026-10-19, Prüfung nach RQ-06; keine Rechtsberatung)

Aufgenommen in `armasync-adapters`: `ureq` 3.4.2 mit `rustls` und `platform-verifier`, `sha1` 0.11.0. Normalbaum (aus `cargo tree`): `ureq`, `ureq-proto`, `http`, `httparse`, `base64`, `bytes`, `log`, `percent-encoding`, `once_cell`, `getrandom`, `itoa`, `cfg-if`, `rustls-pki-types`, `rustls-platform-verifier`, `utf8-zero`, `zeroize`, `windows-link`, `windows-sys`, `sha1`: **MIT OR Apache-2.0** (bzw. MIT); `rustls`: **Apache-2.0 OR ISC OR MIT**; `rustls-webpki`, `untrusted`: **ISC**; `subtle`: **BSD-3-Clause**; **`ring` 0.17: Apache-2.0 AND ISC** (beide Lizenztexte beilegen; enthält C/Assembler); **`webpki-roots` 1.0: CDLA-Permissive-2.0** (Mozilla-Wurzelzertifikatsdaten, im Baum, aber vom Client nicht zur Prüfung genutzt, solange der Plattformverifier gewählt ist; Notice trotzdem beilegen). Nur Tests (nicht ausgeliefert): `rcgen` (MIT OR Apache-2.0) mit `time`, `yasna`, `pem`. Die vollständige Release-BOM (alle Pakete, Texte, Systembestandteile) steht noch aus (RQ-06 bleibt offen).

**Nachtrag MIG-032A (2026-10-19):** `ctrlc` 3.5.2 (MIT OR Apache-2.0; Ctrl+C-Behandlung der CLI) mit `windows-sys` (bereits aufgeführt) bzw. `nix` (MIT, nur Unix); keine weiteren Normalabhängigkeiten.

**Nachtrag MIG-030E (2026-10-25, ADR-0008):** `zip` 7.3.0 (MIT; MSRV 1.83; Feature `deflate-flate2`, ohne AES/bzip2/zstd/lzma/ppmd/xz/zopfli) in `armasync-adapters` zum Entpacken gepackter Blätter; zusätzlich `typed-path` 0.12.4 (MIT OR Apache-2.0), `simd-adler32` 0.3.10 (MIT); `flate2` 1.1.10 (MIT OR Apache-2.0) mit `miniz_oxide` 0.9.1 (MIT OR Zlib OR Apache-2.0), `crc32fast` 1.5.2 (MIT OR Apache-2.0), `adler2` 2.0.1 (0BSD OR MIT OR Apache-2.0). Nur Entwicklung (Tests): `zip` mit Schreibfunktion im selben Crate. Zip-Version 9.x verworfen (MSRV 1.88). Keine Rechtsberatung; die vollständige Release-BOM bleibt offen (RQ-06).

**Nachtrag MIG-030H (2026-10-19):** `fs4` 1.1.0 (MIT OR Apache-2.0; Dateisperren) in `armasync-adapters`; keine weiteren Normalabhängigkeiten.

**Nachtrag MIG-040B (2026-10-25, ADR-0010 Entscheidung b; Prüfung nach RQ-06; keine Rechtsberatung):** `reqwest` 0.13.5 (MIT OR Apache-2.0; MSRV 1.85.0; Features `blocking` und `rustls`, Default-Features aus) in `armasync-adapters` für den WebDAV-Client (`MKCOL`, `PROPFIND`, `OPTIONS`, `PUT`, `DELETE`). Der HTTP-Download-Pfad bleibt bei `ureq`.

Im kompilierten Normalbaum des Adapters neu hinzugekommen (`cargo tree -p armasync-adapters -e normal`, 21 Pakete) und jeweils nach Cargo-Metadaten:
`reqwest` 0.13.5 (MIT OR Apache-2.0); `hyper` 1.12.0 (MIT); `hyper-util` 0.1.21 (MIT); `hyper-rustls` 0.27.10 (Apache-2.0 OR ISC OR MIT); `tokio-rustls` 0.26.6 (MIT OR Apache-2.0); `tower` 0.5.3 (MIT); `tower-http` 0.6.11 (MIT); `tower-layer` 0.3.3 (MIT); `tower-service` 0.3.3 (MIT); `http-body` 1.1.0 (MIT); `http-body-util` 0.1.5 (MIT); `futures-channel` 0.3.34 und `futures-sink` 0.3.34 (MIT OR Apache-2.0); `ipnet` 2.12.2 (MIT OR Apache-2.0); `mio` 1.2.4 (MIT); `socket2` 0.6.5 (MIT OR Apache-2.0); `sync_wrapper` 1.0.2 (Apache-2.0); `try-lock` 0.2.5 (MIT); `want` 0.3.2 (MIT); **`aws-lc-rs` 1.18.2 (ISC AND (Apache-2.0 OR ISC))** und **`aws-lc-sys` 0.46.0** (Mischlizenz laut Cargo-Metadaten: ISC, Apache-2.0, MIT, BSD-3-Clause; enthält C-Quellen; Lizenztexte aus dem Crate-Paket für die Release-BOM übernehmen).

Nur im Lockfile (featureunabhängig eingetragen, **nicht kompiliert und nicht ausgeliefert**): `quinn` 0.11.12, `quinn-proto` 0.11.19, `quinn-udp` 0.5.16 (MIT OR Apache-2.0; HTTP/3, nicht aktiv), `lru-slab` 0.1.3 (MIT OR Apache-2.0 OR Zlib). Nur Build-Zeit (für `aws-lc-sys`, auf dem Build-Rechner): `cmake` 0.1.58, `dunce` 1.0.5, `fs_extra` 1.3.0 (MIT). Zweite `rand`-Linie nur im Lockfile bzw. über Build-/Test-Pfade: `rand` 0.10.3, `rand_core` 0.10.1, `rand_pcg` 0.10.2, `chacha20` 0.10.2 (MIT OR Apache-2.0); `tinyvec` 1.13.3 (Zlib OR Apache-2.0 OR MIT).

Offen: Zwei TLS-Stacks im Programm (`ring` für `ureq`, `aws-lc-rs` für `reqwest`); Vereinheitlichung ist eine Folgeentscheidung. Die vollständige Release-BOM (Lizenztexte, Systembestandteile) bleibt offen (RQ-06).

Neue Übernahmen erhalten Original-Commit/URL, Lizenz, Rechtebeleg, Zielpfad und Prüfdatum. Vor Rust-Release vollständigen Bericht aus Cargo.lock und ausgelieferten Assets/Systembestandteilen erzeugen. Die Java-Credits stimmen nicht vollständig mit Gradle überein und sind keine verlässliche Abhängigkeitsinventur.
