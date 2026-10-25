# ADR-0010 — FTP/FTPS, SFTP und WebDAV: Reihenfolge, Vertrag und offene Entscheidungen (MIG-040A)

Datum: 2026-10-25. Status: **angenommen** (Bestätigung durch den Projektverantwortlichen am 2026-10-25) für Priorisierung, gemeinsamen Vertrag, Authentifizierungs- und TLS-Regeln sowie Entscheidung 4. **Offen** bleiben die Entscheidungen 1, 2, 3 und 5 bis 8; sie blockieren nur die jeweils in „Blockaden“ genannten Folgetasks. Bezug: [TRANSPORT_MATRIX](../migration/TRANSPORT_MATRIX.md), [FEATURE_PARITY](../migration/FEATURE_PARITY.md) (F05–F07, F11), [ADR-0005](0005-credentials-and-data-paths.md), [ADR-0006](0006-sync-metadata-and-http-transport.md), [ADR-0007](0007-transfer-ports.md), [ADR-0009](0009-resume-range.md), [RELEASE_BLOCKERS](../migration/RELEASE_BLOCKERS.md), [RIGHTS_QUESTIONS](../tasks/RIGHTS_QUESTIONS.md) (RQ-06).

## Kontext

Die Java-Baseline nutzt für die vier Protokolle Bibliotheken: FTP/FTPS Apache Commons Net, SFTP Apache MINA SSHD, WebDAV Sardine, HTTP die JVM-Clientschicht (`HttpDAO`). Laut TRANSPORT_MATRIX ist keines dieser Protokolle gegen einen echten Server belegt; im Loopback-Harness (MIG-003) gibt es nur HTTP und einen eingeschränkten WebDAV-Ausschnitt (`OPTIONS`, `PROPFIND`, `MKCOL`, `PUT`, `DELETE`, `GET`). Der Rust-Stand kann heute nur HTTP(S) über `ureq` (ADR-0006, blockierend, ohne Tokio im Core). Synchronisation läuft bisher nur über HTTP(S) und Dateisystem (MIG-030D); die Transportwahl erfolgt über Ports, ohne Änderung des Core-Vertrags.

Der Projektstandard setzt für neue Abhängigkeiten voraus: Lizenz-, Wartungs-, MSRV- und Plattformprüfung, ADR und Eintrag in die Lizenzdokumentation (RQ-06). Der Workspace ist auf **MSRV 1.85** festgelegt.

## Befunde zu Bibliotheken (Stand 2026-10-25, crates.io-API, nicht gebaut)

| Protokoll | Kandidat | Version | Lizenz | MSRV (crates.io) | Laufzeitmodell | Wartung (letzte Aktualisierung) | Befund |
| --- | --- | --- | --- | --- | --- | --- | --- |
| FTP/FTPS | `suppaftp` | 12.2.1 | MIT OR Apache-2.0 | **1.88.0** | nach Dokumentation synchron und async (Features zu prüfen) | 2026-10-15, ~5,05 Mio. Downloads | **MSRV über 1.85**; TLS-Backend und FTPS-Modus (explizit/implizit) müssen vor Aufnahme am Quellcode geprüft werden |
| SFTP | `russh` | 0.64.1 | Apache-2.0 | **1.89** | asynchron (Tokio-basiert, zu prüfen) | 2026-10-05 | **MSRV über 1.85**; Client und Server, reine Rust-Implementierung; Laufzeitmodell passt nicht zum blockierenden Core-Ansatz |
| SFTP | `russh-sftp` | 3.0.1 | Apache-2.0 | nicht angegeben | asynchron, setzt `russh` voraus | 2026-09-28 | nur zusammen mit `russh` sinnvoll; MSRV unklar |
| SFTP | `ssh2` | 0.9.6 | MIT OR Apache-2.0 | nicht angegeben | synchron; Bindung an **libssh2** (C, BSD-3-Clause) | 2026-06-30 | synchron passend; Windows-Build benötigt C-Toolchain und TLS-/Kryptobibliothek (vendored oder System); MSRV muss durch Build geprüft werden |
| WebDAV | `reqwest_dav` | 0.3.3 | MIT OR Apache-2.0 | nicht angegeben | asynchron (`reqwest` + Tokio) | 2026-03-02 | **Laufzeitmodell passt nicht** zu ADR-0006; geringe Wartungsdichte, daher nicht bevorzugt |
| WebDAV | eigener Client auf `ureq` | — | — | 1.85 (über bestehenden Adapter) | blockierend | — | keine neue Netzabhängigkeit; XML-Parsing für `PROPFIND` braucht ggf. einen eigenen Parser-Crate (Lizenz-/MSRV-Prüfung noch offen) |

Die Versionsangaben stammen aus der crates.io-API und sind nicht durch einen Build im Repository belegt. Vor jeder Aufnahme sind Version, Lizenzdatei, MSRV (`cargo +1.85.0 check`), Plattformanforderung (Windows-Build) und Feature-Umfang zu verifizieren und in `THIRD_PARTY_NOTICES.md` aufzunehmen.

## Entscheidung: Priorisierung

Die Reihenfolge begründet sich aus Parität, Sicherheit und Interoperabilität. Die Empfehlung ist bindend erst nach Bestätigung (siehe „Offene Entscheidungen“).

1. **WebDAV-Upload (F07) zuerst (Folgetask MIG-040B).** Geringster Zusatzaufwand: vorhandener HTTP-Adapter mit rustls und Plattformverifier, Authentifizierung über Basic, Testserver-Ansatz aus MIG-003 bereits vorhanden. Keine neue Laufzeit und keine neue Kryptobibliothek. Interoperabilität ist ungeprüft (echte WebDAV-Server fehlen).
2. **FTPS/FTP (F05) als Folgetask MIG-040C.** Ein Protokoll mit zwei Varianten. FTPS ist die sichere Form; reines FTP überträgt Zugangsdaten und Daten unverschlüsselt. Das Verhalten der Baseline (`AUTH TLS`, `PBSZ 0`, `PROT P`; siehe `FtpDAO`) ist als Ausgangspunkt zu verwenden, nicht als Vorgabe, wo es unsicher ist.
3. **SFTP (F06) als Folgetask MIG-040D.** Höchste Sicherheit, aber größter Aufwand: Host-Key-Vertrauensspeicher (zustandsbehaftet, neue Datei), Laufzeit- und MSRV-Frage, und Schlüsselauthentifizierung hängt an ADR-0005 (persistente Credentials offen). Beginn erst nach Entscheidung zu Bibliothek und Laufzeit.

F11 (Parallelität, Session-Reuse, Limits) ist kein eigener Protokollumfang, sondern gilt für alle Folgetasks: begrenzte Worker, konsistenter Abbruch, Größenlimits.

## Gemeinsamer Vertrag (für alle Folgetasks)

- **Port:** Die Transportadapter implementieren die bestehenden Ports (`fetch`/Streaming und Staging nach ADR-0007). Core und Application bleiben protokollunabhängig; keine Protokollnamen im Core.
- **Adapter-Ort:** `armasync-adapters`, je Protokoll ein Modul; Abhängigkeiten nur dort (wie `ureq`).
- **Capabilities:** Jeder Adapter meldet, was er unterstützt (Auflistung, Download, Upload, Löschen, Verzeichnisanlage, Resume ja/nein, Auth-Arten). Der Plan verweigert Operationen, die ein Transport nicht kann, mit einem typisierten Fehler; kein stiller Ersatz.
- **Resume:** FTP `REST` und SFTP-Offsets sind **nicht** Teil von ADR-0009 (der dort festgelegte Validatorvertrag gilt nur für HTTP-`Range`). Jede Protokoll-Fortsetzung braucht einen eigenen Validatorvertrag; bis dahin: Neustart von Null, Fehlerfall dokumentiert.
- **Verbindungsaufbau:** Timeouts für Verbindung, Authentifizierung, Antwort und Leerlauf getrennt konfigurierbar; Standardwerte in der Implementierung festlegen und testen.
- **Fehler:** Typisierte Fehlerklassen (siehe unten), abgebildet auf die bestehende Exitcode-Kategorie. Keine Roh-Protokollzeilen mit Zugangsdaten in Meldungen oder Logs.
- **Pfade:** Jede Serverpfadangabe läuft durch die bestehende sichere Pfadprüfung (`RelativePath`); keine Pfadausbrüche über `..`, absolute Pfade oder Server-Symlinks.
- **Abbruch:** Kooperativer Abbruch über das bestehende Abbruchsignal; danach keine offenen Datei-Handles oder Verbindungen (Test).

## Authentifizierung

| Protokoll | Zulässige Arten in Phase 1 | Regeln |
| --- | --- | --- |
| WebDAV | HTTP Basic (über HTTPS) | Basic nur über TLS (Entscheidung 4). Anonyme Zugriffe über `http` sind möglich, solange keine Zugangsdaten gesendet werden. Kein Digest/NTLM in Phase 1. |
| FTP/FTPS | USER/PASS | Bei reinem FTP Warnung; Zugangsdaten nie in Prozessargumenten oder Logs. |
| SFTP | Passwort | Schlüsselauthentifizierung erst nach ADR-0005 (persistente Schlüssel/Credentials). |

Zugangsdaten kommen in Phase 1 nur aus dem CredentialStore-Port (MIG-020) bzw. aus der Sitzungseingabe. **Keine persistente Credential-Speicherung** vor Entscheidung ADR-0005.

## TLS- und Host-Key-Prüfung

- **TLS (WebDAV, FTPS):** Zertifikatskette und Hostname werden immer geprüft. Keine Option, Umgebungsvariable oder Testschalter schwächt das (ADR-0006, Regel 5). Tests verwenden eine ausdrücklich injizierte Testwurzel, getrennt benannt.
- **FTPS:** Explizit (`AUTH TLS`, Port 21) und implizit (Port 990) sind zu unterscheiden; Datenkanal ebenfalls verschlüsselt (`PROT P`). Ein Modus ohne Datenkanal-Verschlüsselung wird nicht angeboten, außer die Nutzerentscheidung lautet anders.
- **SFTP:** Host-Key-Prüfung ist Pflicht. Unbekannter Schlüssel: Verbindung abbrechen mit klarer Meldung und Fingerabdruck; Vertrauen wird nur durch expliziten Nutzerakt gespeichert (Speicherort und Format sind eine Folgeentscheidung). Ein **geänderter** Schlüssel bricht immer hart ab, ohne Dialog-Umgehung. Kein stilles Akzeptieren beim ersten Mal (kein TOFU ohne Bestätigung).
- **Proxy:** Keine impliziten Proxies. Explizite Proxy-Konfiguration ist eine separate Folgeaufgabe (Matrix: Proxy offen).

## Fehlerfälle (Akzeptanzliste je Protokoll)

Mindestens getestet werden: Verbindung verweigert, Namensauflösung, Timeout (Verbindung, Antwort, Leerlauf), Abbruch durch Nutzer, Authentifizierung fehlgeschlagen, Zugriff verweigert, Ziel fehlt, Ziel existiert bereits, Serverabbruch mitten im Transfer, falsche Größe, falscher Hash, unzulässiger Pfad, Zertifikat ungültig/abgelaufen/falscher Hostname (TLS-Protokolle), Host-Key unbekannt/geändert (SFTP), Server ohne Passivmodus (FTP), WebDAV `423 Locked`, `507 Insufficient Storage`, `409` bei fehlendem Elternverzeichnis, Umleitung auf fremden Host.

## Lokale Testserver (Vorschlag, noch nicht gebaut)

Grundsatz: nur Loopback, synthetische Zugangsdaten, selbst erzeugte Zertifikate, keine öffentlichen Testserver, keine echten Zugangsdaten.

- **WebDAV:** Die bestehende Python-Standardbibliothek-Harness (`migration-tools/transport-testserver`) erweitern: TLS-Variante mit Testwurzel, `423`/`507`/`409`-Antworten, Umleitungen. Interoperabilität mit echten WebDAV-Servern (z. B. Apache `mod_dav`, IIS-WebDAV) ist ein eigener Schritt und ist offen.
- **FTP/FTPS:** Ein Python-FTP-Server ist nicht in der Standardbibliothek. Kandidat `pyftpdlib` (MIT, Version und Wartung zu prüfen) als Test-Abhängigkeit; ihre Lizenz und Version gehören in die Lizenzdokumentation. Alternative: Rust-Testserver im Testkit (kein neuer Fremdcode, aber Aufwand).
- **SFTP:** Kandidaten: OpenSSH-Server (Windows-Feature „OpenSSH-Server“, BSD-Lizenz; Installation benötigt Administratorrechte und ist deshalb ein Entscheidungspunkt), oder ein Rust-Testserver mit `russh` (Abhängigkeit siehe oben). Lizenz und MSRV des Testwerkzeugs sind ebenfalls zu dokumentieren.

## Entscheidung 4: Umsetzungsregeln (WebDAV, MIG-040B)

- Zugangsdaten werden nur für Basis-Adressen mit `https://` angenommen. Eine `http://`-Adresse mit Zugangsdaten wird beim Anlegen des Clients abgelehnt, bevor eine Verbindung entsteht.
- Zugangsdaten werden nur an die Herkunft (Schema, Host, Port) der Basis-Adresse gesendet. Das wird bei jeder Anfrage geprüft, auch bei jeder Weiterleitung.
- Weiterleitungen auf eine andere Herkunft (anderer Host, anderer Port, anderes Schema, insbesondere `https` → `http`) werden abgelehnt, ohne das Ziel zu verbinden. Das gilt unabhängig davon, ob Zugangsdaten vorhanden sind.
- Schreibende Methoden (`PUT`, `MKCOL`, `DELETE`) folgen keinen Weiterleitungen und brechen mit Fehler ab. Lesende Methoden ohne Körper (`OPTIONS`, `PROPFIND`) folgen Weiterleitungen nur innerhalb derselben Herkunft.
- Zugangsdaten erscheinen nie in Fehlermeldungen oder Debug-Ausgaben. Der Basic-Header wird nur in TLS-Anfragen gesetzt.
- Keine neue Abhängigkeit: die Base64-Kodierung für den Basic-Header und die Existenzprüfung über Statuscodes (ohne Auswertung des XML-Körpers) sind im Adapter umgesetzt.
- Uploads kommen nur aus Dateien mit bekannter Länge, damit `Content-Length` gesetzt wird.

## Offene Entscheidungen (Projektverantwortlicher)

1. **MSRV:** `suppaftp` (1.88) und `russh` (1.89) liegen über dem Workspace-MSRV 1.85. Optionen: (a) MSRV anheben auf mindestens 1.89 (Auswirkung: [RUST_TOOLCHAIN](../migration/RUST_TOOLCHAIN.md), CI-MSRV-Job, Begründung in ADR-0006); (b) ältere, mit 1.85 bauende Versionen pinnen (zu prüfen); (c) eigene Protokollschicht auf `ureq`/`rustls` (FTP/FTPS-Befehlsebene wäre eigener Code, Sicherheitsrisiko). Empfehlung: (b) zuerst prüfen; (a) nur mit neuer Entscheidung.
2. **SFTP-Bibliothek und Laufzeit:** `ssh2` (synchron, C-Abhängigkeit) oder `russh` (asynchron, Tokio im Adapter, rein Rust). Die Wahl hängt an Entscheidung 1 und am Windows-Build von libssh2/OpenSSL.
3. **Plaintext-FTP:** Ist reines FTP ohne TLS als Option zugelassen (mit Warnung und Opt-in), oder nur FTPS? Empfehlung: nur FTPS und FTP nur mit ausdrücklichem Opt-in pro Repository.
4. **WebDAV über `http`: ENTSCHIEDEN (2026-10-25): nein.** HTTP Basic Auth ist nur über HTTPS erlaubt; es gibt keine Opt-in-Ausnahme für unverschlüsseltes HTTP. Anonyme Zugriffe über HTTP bleiben möglich, solange keine Zugangsdaten gesendet werden. Umsetzungsregeln siehe Abschnitt „Entscheidung 4: Umsetzungsregeln“.
5. **Persistenz des SFTP-Host-Key-Vertrauensspeichers:** Format und Ort (neue Datei unter den Datenverzeichnissen, ADR-0005).
6. **Schlüsselauthentifizierung SFTP:** erst nach ADR-0005 (Schlüsselbund bzw. Dateiberechtigungen). Bis dahin nur Passwort.
7. **Testwerkzeuge:** Zulassung von `pyftpdlib` bzw. OpenSSH als Testabhängigkeit und Installationsrechte auf dem Windows-Entwicklungsrechner.
8. **Legacy-Parität:** Ist die Baseline-Eigenheit „FTPS nur mit `PBSZ 0`/`PROT P`“ als Verhalten zu übernehmen oder ist eine sicherere Standardeinstellung gewünscht? Empfehlung: sichere Standardeinstellung, Abweichung dokumentieren.

## Blockaden

- **MIG-040B (WebDAV)**: nicht blockiert (Entscheidung 4 ist getroffen). Umsetzung ohne neue Abhängigkeit und ohne XML-Auswertung; siehe Umsetzungsregeln.
- **MIG-040C (FTP/FTPS)** ist blockiert durch Entscheidungen 1, 3, 7.
- **MIG-040D (SFTP)** ist blockiert durch Entscheidungen 1, 2, 5, 6, 7.
- Keine Protokollimplementierung vor Bestätigung dieser ADR.

## Folgen

- Keine Änderung am Code, an Abhängigkeiten, an `Cargo.toml` oder `Cargo.lock` in MIG-040A.
- Die Feature-Parität F05–F07 bleibt „offen“, bis Folgetasks mit Testservern belegt sind.
- Lizenzprüfung (RQ-06) für jede Abhängigkeit vor der Aufnahme.
- Rechtefragen RQ-01 bis RQ-08 bleiben unberührt.

## Nachtrag 2026-10-25 (MIG-040B): Blockade für MKCOL und PROPFIND

- **Befund:** `ureq` 3.4.2 mit `ureq-proto` 0.6.5 lässt über HTTP/1.1 nur `GET`, `HEAD`, `POST`, `PUT`, `DELETE`, `CONNECT`, `OPTIONS` und `TRACE` zu. Andere Methoden werden vor dem Senden mit `MethodVersionMismatch` abgelehnt (`ureq-proto`, `src/ext.rs`; im Test belegt: `MKCOL` und `PROPFIND` schlagen fehl). WebDAV braucht `MKCOL` (Ordner anlegen) und `PROPFIND` (Existenzprüfung) über HTTP/1.1.
- **Umgesetzt in MIG-040B:** `OPTIONS` (Fähigkeiten, WebDAV-Klasse 1), `PUT` (Upload einer Datei mit bekannter Länge in einen vorhandenen Ordner) und `DELETE`. Keine Änderung an `Cargo.toml` oder `Cargo.lock`.
- **Nicht umgesetzt, Entscheidung nötig** (nicht vorweggenommen):
  a. eigener minimaler HTTP/1.1-Schreiber über `rustls` (`rustls` wäre dann eine direkte Abhängigkeit des Adapters; bereits transitiv im Lockfile; ADR und Sicherheitsprüfung des eigenen Protokollcodes nötig);
  b. eine HTTP-Bibliothek, die beliebige Methoden zulässt (neue Abhängigkeit; Lizenz-, MSRV- und Wartungsprüfung nach RQ-06);
  c. Ordner außerhalb des Clients anlegen und Uploads nur in vorhandene Ordner zulassen (Stand von MIG-040B).
- **Auswirkung:** Uploads in neu anzulegende Unterordner sind nicht möglich, bis a oder b entschieden ist. MIG-042 (Build/Upload) ist davon betroffen; MIG-040C und MIG-040D sind davon nicht berührt.

## Belege

- Quellen: TRANSPORT_MATRIX (Zeilen HTTP, FTP, FTPS, SFTP, WebDAV, Authentifizierung), FEATURE_PARITY F05–F07, ADR-0005/0006/0007/0009, Workspace-`Cargo.toml` (`rust-version = "1.85"`).
- Bibliotheksdaten: crates.io-API (`/api/v1/crates/<name>`) am 2026-10-25; nicht gebaut, nicht geprüft gegen Quelltext.
