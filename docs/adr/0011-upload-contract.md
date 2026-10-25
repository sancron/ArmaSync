# ADR-0011 — Upload-Vertrag (Plan, Apply, Abbruch) und TLS-Stacks (MIG-042, Schritt 1)

Datum: 2026-10-25. Status: **angenommen** für den Umfang Schritt 1 (Dateien hochladen in einen vorhandenen Zielbaum). Nicht angenommen: Metadaten-Veröffentlichung, Revisionen, Ausschlüsse, Löschen, Parallelität, Wiederaufnahme und Verifikation nach dem Upload; das sind Folgeschritte von MIG-042 mit eigenen Verträgen. Bezug: [ROADMAP](../migration/ROADMAP.md) (P4, MIG-042), [ADR-0004](0004-core-contracts.md) (Kernverträge), [ADR-0007](0007-transfer-ports.md) (Ports), [ADR-0010](0010-ftp-sftp-webdav-contract.md) (WebDAV, Entscheidung 4 und Nachtrag Option b), [ADR-0005](0005-credentials-and-data-paths.md).

## Kontext

Der Core kennt bisher nur den Download (`RepositorySource`, `LocalStore`). Für einen Upload fehlen zwei Gegenstücke: eine lokale Quelle, die Dateien auflistet und zum Lesen öffnet, und ein Remote-Ziel, das Ordner anlegt und Dateien schreibt. Der WebDAV-Client (MIG-040B) kann `exists`, `make_collection`, `upload` (PUT) und `delete` ausführen, ist aber nicht in einen Ablauf eingebunden.

## Entscheidungen

1. **Ports im Core, ohne Protokollnamen.** `LocalSource` (Quelle) und `RemoteTarget` (Ziel) liegen in `armasync-core::port`. Die Adapter implementieren sie: `LocalDirectory` für die Quelle, `WebDavClient` für das Ziel. Die Anwendungsschicht kennt nur die Ports.
2. **Quelle (`LocalSource`).**
   - `files(cancel)` liefert alle regulären Dateien unterhalb der Quellwurzel mit Pfad und Größe, sortiert nach Pfad. Links und Junctions werden nie aufgelistet.
   - `open(path)` liefert eine `OpenedSource` mit Größe zum Öffnungszeitpunkt und einem Leser. Links und Nicht-Dateien werden abgelehnt; die Eltern-Ordner werden wie beim Schreiben geprüft und unter Windows festgehalten (Ordner-Pinning, gleiche Schutzwirkung wie im Zielordner).
3. **Ziel (`RemoteTarget`).** Die drei Operationen sind `exists(path)`, `make_folder(path)` (ein Ordner je Aufruf) und `upload(path, source, cancel)`. Der Upload liest den Leser streamend mit bekannter Länge und bricht beim ersten Lesevorgang nach Abbruch ab.
4. **Plan.**
   - Für jeden Ordner-Vorfahren einer Datei: existiert er im Ziel nicht, entsteht ein Schritt `MakeFolder`. Ordner werden nach Tiefe, dann nach Pfad sortiert.
   - Für jede Datei: `replace` gibt an, ob sie im Ziel bereits existiert. Das ist eine Information; der Upload überschreibt in jedem Fall.
   - Der Plan enthält nur Schritte; er verändert nichts. Summen (Ordner, Dateien, Bytes) gehören zum Plan.
5. **Apply.**
   - Schritte in Planreihenfolge. Vor jedem Schritt wird der Abbruch geprüft.
   - Vor einem Upload wird die Quelle geöffnet. Weicht die Größe vom Plan ab, schlägt der Schritt mit `Integrity` fehl („changed since the plan“), bevor etwas gesendet wird.
   - Bricht der Nutzer ab, während ein Upload läuft, endet der Vorgang mit `Cancelled`. Ein Fehler nach einem gesetzten Abbruch wird als `Cancelled` gemeldet.
   - Der Bericht zählt angelegte Ordner, neu angelegte Dateien, ersetzte Dateien und Bytes. Bei einem Fehler gilt: Bereits fertige Schritte bleiben, der fehlgeschlagene Schritt hinterlässt keine Teildatei, die als fertig gilt.
6. **Sicherheitsregeln.**
   - Pfade sind immer `RelativePath` (keine `..`, keine absoluten Pfade).
   - Zugangsdaten nur über HTTPS, nur an die Herkunft der Basis-Adresse, bei jeder Anfrage und jedem Redirect geprüft (ADR-0010). Schreibende Methoden folgen keinen Redirects.
   - Der Ablauf löscht in Schritt 1 nichts.
   - Eine unvollständige Übertragung darf auf dem Server nicht als fertige Datei erscheinen. Das ist für den getesteten Loopback-Server belegt (Abbruch vor dem letzten Byte, keine Datei). Für andere Server ist es nicht zugesichert und wird nicht behauptet.
7. **Kosten und Grenzen Schritt 1.** `LocalSource::files` nutzt den bestehenden Scan (`LocalStore::list`) und berechnet dabei Hashes. Das ist bewusst übernommen; eine hashfreie Auflistung ist ein Folgeschritt. Nach dem Upload gibt es keine Prüfsumme auf dem Server (kein Read-back-Port in Schritt 1).
8. **Nicht im Scope (Folgeschritte MIG-042).** Metadaten-Build (`.a3s/sync`, `serverinfo`, zuletzt veröffentlicht), Revisionen, Dateiausschlüsse, Ereignisse und Changelogs, Parallelitätslimits, Wiederaufnahme, Read-back-Verifikation, Verdrahtung in CLI und GUI, Löschen verwalteter Dateien.

## TLS-Stacks (Entscheidung: getrennt bleiben)

Zwei Kryptostacks bleiben vorerst getrennt; eine Vereinheitlichung ist kein Bestandteil dieses ADR.

| Pfad | Client | Features (default-features aus) | Zertifikatsprüfung | Test-Wurzel |
| --- | --- | --- | --- | --- |
| Download (MIG-030B) | `ureq` 3.4.2 | `rustls`, `platform-verifier` | `RootCerts::PlatformVerifier` (Windows-Speicher) | `RootCerts::new_with_certs` ersetzt die Wurzeln |
| WebDAV (MIG-040B) | `reqwest` 0.13.5 blocking | `blocking`, `rustls` (→ `aws-lc-rs`, `rustls-platform-verifier`) | Plattformverifier | `tls_certs_only` ersetzt die Wurzeln |

Folgen: zwei Kryptobibliotheken im Programm (`ring` und `aws-lc-rs`), damit doppelte Binärgröße und zwei Build-Pfade (`aws-lc-sys` mit C-Quellen). Beide Verifier sind TLS-Zertifikatsprüfung ohne Abschaltung. Eine Vereinheitlichung würde denselben Client für Download und Upload verlangen und ist eine eigene Entscheidung, falls ein konkreter Befund es erfordert. Im Schritt 1 gibt es keinen solchen Befund.

## Belege

- Core-Ports `LocalStore`, `RepositorySource`, `CancelToken` (vorhandene Abbruchsemantik).
- Vorhandene Schutzmechanismen des lokalen Adapters: `secure_target`, `is_link`, Ordner-Pinning (Windows).
- Test-Nachweise: `crates/armasync-application/tests/upload_cases.rs` (Plan und Apply mit Fakes), `crates/armasync-adapters/tests/upload_flow.rs` (echte Quelle, WebDAV-Loopback), `crates/armasync-adapters/tests/webdav.rs` (Zeit- und Abbruchverhalten des Clients).
