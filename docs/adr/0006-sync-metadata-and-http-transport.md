# ADR-0006 — Sync-Metadaten, SHA-1-Integrität und HTTP(S)-Transport für MIG-030

Datum: 2026-10-19. Status: **angenommen** für die Wahl und die Zerlegung; die Schnittstellen der Ports werden in den jeweiligen Tasks (MIG-030B/C) als getestete Typen festgeschrieben. Bezug: [SYNC_METADATA](../migration/SYNC_METADATA.md), [ADR-0003](0003-legacy-formats.md), [ADR-0004](0004-core-contracts.md), [TRANSPORT_MATRIX](../migration/TRANSPORT_MATRIX.md), [RIGHTS_QUESTIONS](../tasks/RIGHTS_QUESTIONS.md).

## Kontext

MIG-030 verlangt Scan → Plan → HTTP(S)-Download → Hashprüfung → atomare Installation. Der Kern (`plan_sync`, `apply`) existiert mit In-Memory-Adaptern; es fehlen der Leser der Server-Metadaten, ein echter Dateisystemadapter, ein HTTP-Client und die Anbindung. Die Server veröffentlichen **SHA-1** (nicht SHA-256) als Dateiprüfsumme; der Core kennt nur SHA-256. `RepositorySource::read` liefert eine ganze Datei als `Vec<u8>` – für Mod-Dateien im Gigabytebereich ungeeignet.

## Entscheidungen

1. **Leser:** `.a3s/sync` und `.a3s/serverinfo` werden in `armasync-compat` mit dem vorhandenen begrenzten Streamleser (MIG-022) gelesen und in neutrale DTO (`SyncDirectory`, `SyncFile`, `ServerInfo`) überführt. Compat bleibt ohne Core-Abhängigkeit; die Abbildung auf `Manifest` (mit `RelativePath`-Validierung) liegt in `armasync-adapters`. Keine Altquelle wird übernommen (SYNC_METADATA, Abschnitt „Rechte“).
2. **Hashart:** `HashAlgorithm::Sha1` wird **additiv** im Core ergänzt und ausdrücklich als *Integritätsprüfung des Altprotokolls* dokumentiert, nicht als Authentizität oder Kollisionsfestigkeit. Zusammen mit TLS-Prüfung (HTTPS) ist das der Schutz gegen Übertragungsfehler; bei `http`/`ftp` gilt die bestehende Warnung. Ein Manifest hat eine einheitliche Hashart pro Revision; `plan_sync` vergleicht nur gleiche Arten. SHA-1-Berechnung: Crate `sha1` 0.11.0 (MIT OR Apache-2.0, MSRV 1.85, gleiche RustCrypto-Familie wie das vorhandene `sha2` 0.11).
3. **Komprimierte Blätter:** nicht Teil der ersten Slices. Sie erscheinen als typisierte Planwarnung „nicht unterstützt“ und werden weder geladen noch stillschweigend als erledigt gezählt (MIG-030E: ZIP-Entpacken mit Bomben-Grenzen).
4. **HTTP-Client: `ureq` 3.4.x** (MIT OR Apache-2.0, MSRV 1.85), `default-features = false`, Features `rustls` und `platform-verifier`, ausschließlich in `armasync-adapters` (CLI und GUI hängen nur über Ports daran). Gründe: blockierend wie die bestehenden Ports (kein Tokio im Kern); MSRV exakt 1.85; reine-Rust-TLS (`rustls`) ohne OpenSSL-Systembedarf; Zertifikate über das Betriebssystem (`rustls-platform-verifier`: unter Windows der Systemspeicher, damit Unternehmens-CAs funktionieren); Proxy aus der Umgebung (`Proxy::try_from_env`); Timeouts je Phase; Weiterleitungen begrenzbar; `Range`-Header setzbar.
5. **TLS wird nie abgeschaltet:** keine Option, keine Umgebungsvariable und kein Testschalter schwächt Zertifikat- oder Hostnamenprüfung. Tests mit eigener Vertrauensbasis nutzen eine ausdrücklich injizierte Wurzel und sind getrennt benannt.
6. **Transportform:** Downloads werden in eine **Staging-Datei im Zielverzeichnis** geschrieben (gleiches Dateisystem), dort gegen SHA-1 und Größe geprüft und erst dann atomar ersetzt; Abbruch und Fehler lassen die Zieldatei unberührt und entfernen die Staging-Datei. Dafür wird `RepositorySource` um eine **streamende** Leseoperation mit Abbruchtoken und Fortschritt erweitert und `LocalStore` um eine Staging-/Commit-Operation (additive Methoden; die Signaturen legt MIG-030B/C als getestete Typen fest, bevor Code darauf aufbaut). Resume per `Range` ist **nicht** in MIG-030, sondern MIG-032 (gebunden an Validator, Größe, URL).

## Bewertung der Kandidaten (Stand 2026-10-19)

| Kandidat | Lizenz | MSRV | Befund | Entscheidung |
| --- | --- | --- | --- | --- |
| `ureq` 3.4.2 | MIT OR Apache-2.0 | 1.85 | blockierend; rustls; Plattformverifier; Proxy/Timeout-API | **gewählt** |
| `reqwest` 0.13.5 | MIT OR Apache-2.0 | 1.85 | async (Hyper/Tokio) – schwerer, zwingt Async in Ports; Blocking-Variante nutzt dennoch Tokio | verworfen (Gewicht, Async nicht gefordert) |
| `attohttpc` 0.31 | MPL-2.0 | unbekannt | schwaches Copyleft auf Dateiebene, MSRV nicht angegeben | verworfen |
| `minreq` 3.0 | ISC | 1.63 | sehr klein, aber TLS-/Proxy-/Timeout-Funktionsumfang für Server-Interop zu knapp | verworfen |
| `hyper` direkt / `curl` | – | – | zu niedrig bzw. C-Systembibliothek | nicht gewählt |

**Belege (Windows 11, Rust 1.85.0, Probe außerhalb des Repositories):** Build mit `cargo +1.85.0` erfolgreich (28 Pakete im Normalbaum); `https://example.com` mit `Range: bytes=0-9` → `206 Partial Content`, `Content-Range: bytes 0-9/577`; **abgelehnt** mit Fehler: abgelaufenes Zertifikat (`expired.badssl.com`), unbekannter Aussteller (`self-signed.badssl.com`), falscher Hostname (`wrong.host.badssl.com`). Nicht geprüft: Linux (zurückgestellt gemäß Prüfregel), expliziter Proxy-Betrieb, Verhalten hinter Unternehmens-TLS-Inspektion, Laufzeit bei großen Dateien.

## Lizenz-/Notice-Inventar (RQ-06)

RQ-06 lautet: Rust-Crates noch nicht gewählt; blockiert die Aufnahme jeder Abhängigkeit in `Cargo.toml` ohne Lizenzprüfung und die erste Release-BOM. Hier geprüft (Normalbaum der Probe): `ureq`, `ureq-proto`, `http`, `httparse`, `base64`, `bytes` (MIT), `log`, `percent-encoding`, `once_cell`, `getrandom`, `itoa`, `cfg-if`, `rustls-pki-types`, `rustls-platform-verifier`, `utf8-zero`, `zeroize`, `windows-link`, `windows-sys`: MIT OR Apache-2.0 (bzw. MIT); `rustls`: Apache-2.0 OR ISC OR MIT; `rustls-webpki`, `untrusted`: ISC; `subtle`: BSD-3-Clause; **`ring`: Apache-2.0 AND ISC** (kryptografische Primitive mit C/Assembler, baut mit den Windows-Buildtools; Notice-Pflicht für beide Lizenzen); **`webpki-roots`: CDLA-Permissive-2.0** (im Baum, Notice-Pflicht; Wurzelzertifikatsdaten, bei aktivem Plattformverifier nicht für die Prüfung genutzt). Alle sind mit AGPL-3.0-only vereinbar im Sinne einer permissiven Einbindung; das ist **keine Rechtsberatung**. Die erste Aufnahme (MIG-030B) ergänzt `THIRD_PARTY_NOTICES.md` aus `cargo tree`-Ausgabe und setzt die Zeile der RQ-06 im Tracker auf „teilweise bearbeitet“ (nicht geschlossen: Release-BOM offen). `sha1` 0.11.0 kommt hinzu.

## Korrektur und Ergänzung aus MIG-030B (2026-10-19)

* **Befund:** Der Standard von `ureq` 3 für die Vertrauensbasis ist `RootCerts::WebPki` (mitgelieferte Mozilla-Wurzeln), **nicht** der Plattformverifier, auch wenn das Feature `platform-verifier` aktiv ist. Der Adapter setzt deshalb `RootCerts::PlatformVerifier` ausdrücklich (Systemspeicher, damit Unternehmens-CAs funktionieren). Nachweis: manueller Test `--ignored` (gültiges Zertifikat von `example.com` akzeptiert; `expired`, `self-signed`, `wrong.host` und `untrusted-root` von `badssl.com` abgelehnt, Windows 11) und automatisierte Loopback-TLS-Tests mit selbstsigniertem Testzertifikat (abgelehnt ohne Injektion, akzeptiert nur mit ausdrücklich injizierter Wurzel, abgelehnt bei falschem Namen oder fremder Wurzel).
* `ureq` hat nur eine **Gesamtzeit für den Antworttext** (`timeout_recv_body`), keine Leerlaufzeit. Der Adapter berechnet sie je Datei als `max(Mindestzeit, Größe / Mindestrate)` (Standard 60 s, 32 KiB/s); eine festhängende Verbindung endet so mit einem Netzwerkfehler, eine langsamere Leitung als die Mindestrate ebenfalls (einstellbar). Der Abbruch wird zwischen den Stücken geprüft; eine blockierende Leseoperation kann ihn bis zum Ablauf dieser Zeit verzögern.
* TLS-Fehler tauchen bei `ureq` als `Io`-Fehler mit Text auf und werden anhand des Textes als „secure connection failed“ erkannt (Netzwerkfehler); das ist heuristisch.
* Weiterleitungen werden vom Adapter selbst verarbeitet (höchstens 5, nur gleicher Ursprung oder Upgrade desselben Hosts von `http` auf `https`).

## Zerlegung von MIG-030 (Eigentümer jeweils Claude)

| Task | Umfang | Voraussetzung | Disjunkter Schreibbereich |
| --- | --- | --- | --- |
| **MIG-030A** | `armasync-compat`: `.a3s/sync`- und `serverinfo`-Leser; Core: `HashAlgorithm::Sha1`; Adapter: Abbildung `SyncTree` → `Manifest` mit Warnungen (unsichere Pfade, komprimierte Blätter, Kollisionen); Golden-Tests mit allen MIG-001-Fixtures. Kein Netzwerk, keine Dateisystemzugriffe. | MIG-022 | `crates/armasync-compat/**`, `crates/armasync-core/src/hash.rs` (additiv), `crates/armasync-adapters/src/sync_meta/**` |
| **MIG-030B** | HTTP(S)-Adapter mit `ureq`: Abruf der Metadaten, streamende Dateiübertragung in eine Senke mit Abbruch/Fortschritt, Fehlerzuordnung (Netzwerk, Auth, TLS, Status, Größe); Port-Erweiterung; Tests gegen den Loopback-Testserver (MIG-003, Windows) | 030A (Metadaten), ADR-0006 | `crates/armasync-adapters/src/http/**`, Port-Ergänzung im Core, `Cargo.toml`/`Cargo.lock`, `THIRD_PARTY_NOTICES.md` |
| **MIG-030C** | Dateisystemadapter: Scan mit SHA-1 (Größen-/Zeitverkürzung nur mit Nachweis), Staging, Hash-/Größenprüfung, atomarer Austausch, Löschen im verwalteten Bereich, Containment gegen Symlinks/Junctions, Windows-Namensregeln | 030A | `crates/armasync-adapters/src/localfs/**` |
| **MIG-030D** | Anbindung: Use-Case-Verdrahtung, CLI `sync plan`/`sync run` (`--json`, `--dry-run`, `--no-input`, Abbruch), End-to-End gegen den Testserver, Windows-Abnahme | 030B, 030C | `crates/armasync-application/**`, `crates/armasync-cli/**` |
| MIG-030E (später) | ZIP-Blätter entpacken (Bombenschutz) | 030D | eigener Task |

Das P3-Gate („beide Frontends synchronisieren denselben Testbestand … unter Windows/Linux“) wird erst mit MIG-031/032 und der abschließenden Plattformvalidierung geprüft; **keine** der Teilaufgaben ist allein „Synchronisation fertig“.

## Offen / bewusst nicht entschieden

- RQ-01 bis RQ-08 bleiben offen; der Parser ist unabhängig geschrieben, die Frage der Verhaltensbeschreibung ist nicht entschieden (SYNC_METADATA, „Rechte“).
- Explizite Proxy-Konfiguration, Authentifizierung (HTTP Basic aus `CredentialStore`), `.zsync`/Delta, FTP/FTPS/SFTP/WebDAV: spätere Tasks (MIG-040/041).
- Größen-/Leistungsgrenzen für große Bäume sind ungemessen.

## Nicht gewählt

Übernahme des Altsyncalgorithmus; SHA-1 als Vertrauensanker; unbeschränkte Weiterleitungen; Abschalten der Zertifikatsprüfung für Tests; ein asynchroner Kern.
