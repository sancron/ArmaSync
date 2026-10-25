# MIG-032R — Resume mit `Range` und Validatorvertrag

- Status: **in Arbeit** (reserviert 2026-10-25). Eigentümer und Integrator: Claude (Projektleitung, Hauptbearbeiter). Basis-Branch `development`, Basis-SHA `1d7049d795e9cc8b949f859b0c42afe70f16472f`. Eigener Branch `claude/mig-032r-resume` (Hauptcheckout). Bericht: [MIG-032R](../changes/2026-10-25-MIG-032R.md), Handoff: [MIG-032R](../handoffs/MIG-032R.md).
- Ziel: Ein unterbrochener Download darf nur dann mit `Range` fortgesetzt werden, wenn der Server nachweislich dieselbe Revision der Datei ausliefert. Fehlt dieser Nachweis, wird neu geladen.
- Vertrag: [ADR-0009](../adr/0009-resume-range.md) (Zusatz zu ADR-0007). Der ADR wird vor der Implementierung geprüft und festgeschrieben.

## Warum ein eigener Vertrag nötig ist

* ADR-0007 sagt ausdrücklich „keine stillen Wiederholungen oder Resume (MIG-032)“. Resume ändert die Port-Semantik und braucht deshalb einen ADR-Zusatz vor der Implementierung.
* Fortsetzen ist nur sicher, wenn die bereits geladenen Bytes zur **aktuellen** Revision gehören. Ein Teil aus einer alten Revision darf nie mit einem neuen Rest zusammengesetzt werden (Datenintegrität).

## Umfang

1. Ports (Core, additiv): Eröffnen einer Übertragung mit Offset und `If-Range`-Validator; resumable Teildatei je Schlüssel (Quelle, Pfad, Größe); Aufbewahren, Verwerfen, Neustart und atomares Übernehmen.
2. HTTP-Adapter: `Range: bytes=<offset>-` und `If-Range` mit starkem ETag; Prüfung von Status 206, `Content-Range`, Gesamtgröße und ETag; Status 200, Status 416, falscher Bereich und verkürzte Antwort werden getrennt behandelt.
3. Dateisystem-Adapter: Teildatei und Zustandsdatei im Zielverzeichnis, unter dem reservierten Präfix, nie gelistet; Aufräumen nach Alter; Zielsperre wie bei allen Änderungen.
4. Anwendung (`sync::apply`): ein gemeinsamer Ablauf für Dateien und gepackte Archive; Gesamtdatei wird nach dem Fortsetzen über Größe und Manifest-Hash geprüft.
5. Tests: Loopback-Testserver (erweitert um `ETag`, `Range`, `If-Range`, Fehlerschalter), synthetische Fälle auf Anwendungsebene, Wiederaufnahme nach Neustart des Prozesszustands, CLI-Ende-zu-Ende.

## Abgrenzung

* Authentifizierung: ausgesetzt, nicht Teil dieses Tasks (ADR-0005 offen).
* Parallele Teilbereiche, Delta/`.zsync`, Linux-Läufe, echte Server, GitHub-CI (pausiert).
* RB-01 bis RB-06, RQ-01 bis RQ-08 und das P1-GUI-Gate bleiben offen.

## Schreibbereich (Claude, exklusiv)

* `docs/adr/0009-resume-range.md`, `docs/tasks/MIG-032R.md`, `docs/changes/2026-10-25-MIG-032R.md`, `docs/handoffs/MIG-032R.md`
* `crates/armasync-core/src/port.rs`, `crates/armasync-core/src/lib.rs` (additive Ports)
* `crates/armasync-adapters/src/http/**`, `crates/armasync-adapters/src/localfs/**`, `crates/armasync-adapters/src/memory.rs`, `crates/armasync-testkit/**` (Loopback-Server)
* `crates/armasync-application/src/sync.rs`, `crates/armasync-application/tests/**`, `crates/armasync-adapters/tests/**`, `crates/armasync-cli/tests/sync_e2e.rs`
* Integrator-exklusiv bei Übernahme: `STATUS.md`, `LAST_CHANGE.md`, `docs/tasks/README.md`, `CHANGELOG*`, `Cargo.lock` (keine neuen Abhängigkeiten geplant).

## Abnahme

* Korrekte Fortsetzung, fehlender oder geänderter Validator, ignorierte Range, falscher `Content-Range`, abgebrochene Antwort, ungültige Teildaten und Wiederaufnahme nach Prozessende sind getestet.
* Fehler oder veraltete Teildaten beschädigen nie eine vorhandene Zieldatei und werden nie ungeprüft installiert.
* CLI und GUI nutzen denselben Workflow; keine doppelte Geschäftslogik.
* Windows: fmt, Clippy `-D warnings`, Workspace-Tests, MSRV 1.85 ohne GUI, Ende-zu-Ende. WSL/Linux und GitHub-CI sind **zurückgestellt**.
