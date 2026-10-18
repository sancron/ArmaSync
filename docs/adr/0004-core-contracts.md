# ADR-0004 — Kernverträge: Pfade, Manifest, Plan, Fehler, Events, Jobs, Ports

Datum: 2026-10-18. Status: **angenommen** für die in MIG-011 implementierten Typen; Erweiterungen (Bereichs-Reads, Async, weitere Hashes) über neue ADRs/Tasks. Bezug: [ADR-0001](0001-rust-workspace.md), [ARCHITECTURE](../migration/ARCHITECTURE.md), [CLI_CONTRACT](../migration/CLI_CONTRACT.md).

## Kontext

GUI und CLI müssen dieselben Use-Cases mit demselben Verhalten nutzen. Der Java-Bestand hat Pfad-, Lösch- und Abbruchverhalten über statische DAOs verteilt. Die Verträge müssen vor den Adaptern feststehen und ohne I/O testbar sein.

## Entscheidung (implementiert in `armasync-core`, Test in `armasync-application`)

| Vertrag | Festlegung |
| --- | --- |
| `RelativePath` | Einzige zulässige Pfadform für Repository- und Zielpfade: `/`-getrennt, nicht absolut, keine `.`/`..`/leeren Segmente, kein Backslash, kein `:` (Laufwerk/ADS), keine Steuerzeichen, kein Segment mit Punkt/Leerzeichen am Ende, keine Windows-Gerätenamen (`CON`, `NUL`, `COM1`…), max. 1024 Byte. Jede externe Eingabe muss durch `RelativePath::parse`. |
| `Manifest` | Unveränderlich, nach Pfad sortiert; lehnt leere Revision, doppelte Pfade, Pfade, die sich nur durch Groß-/Kleinschreibung unterscheiden, und Datei/Verzeichnis-Konflikte ab. `ContentHash` ist Algorithmus + Hex (derzeit nur SHA-256; Legacy-Hashes folgen in der Compat-Schicht, Jazsync/MD4 nur nach RQ-02). |
| `SyncPlan` | Reine Funktion `plan_sync(manifest, lokal, optionen)`. Schritte `Download`/`Replace`/`Keep`/`Delete`, Summen, Warnungen. Löschen nur mit `delete_extra` **und** innerhalb `managed_roots`; sonst bleiben Extras erhalten (Warnung). Groß-/Kleinschreibungs-Abweichungen werden gewarnt, nie gelöscht. Löschschritte stehen zuletzt. |
| Revalidierung | `apply` berechnet den Plan aus aktueller Repository-Revision und aktuellem Lokalzustand neu; Abweichung → `Integrity`, nichts wird geändert. |
| Verifikation | Jede heruntergeladene Datei wird (Größe, Hash) geprüft, bevor `LocalStore::write` sie ablegt; ein Fehler lässt die bestehende Datei unverändert. |
| `ErrorKind` | `Usage`, `Config`, `Network`, `FileSystem`, `Integrity`, `Format`, `Unsupported`, `Cancelled` – deckungsgleich mit den CLI-Exitcodes 2–8 und 130. Meldungen enthalten keine Secrets. |
| `OperationEvent` | `schema_version` (1), `operation`, `sequence` (ab 0, lückenlos), `phase`, `state`, Zähler, optional `ErrorKind`. Keine Freitexte, URLs oder Zugangsdaten. Letztes Event je Operation ist terminal. |
| `JobState` | `Queued → Running → {Succeeded, Failed, Cancelled}`; `Queued → Cancelled`; terminale Zustände final; Abbruch ist nie Erfolg. Kein Pause/Resume ohne persistierten Resumezustand. |
| Ports | `RepositorySource` (Manifest, Datei lesen), `LocalStore` (list, **atomares** write, delete im Root, keine Links nach außen), `Hasher`, `EventSink`, `CancelToken` (kooperativ, zwischen Dateien geprüft). Synchron und blockierend; Async/Bereichs-Reads sind spätere, bewusste Erweiterungen. |

## Nicht gewählt

Asynchrone Ports sofort (würde Runtime-Wahl vorwegnehmen); Pfade als `PathBuf` im Core (OS-abhängig, Traversal-Risiko); Freitext in Events; automatisches Löschen als Default.

## Folgen

* Adapter (P3) müssen die dokumentierte Atomizität und die Root-Bindung des `LocalStore` einhalten; ein Adapter-Konformitätstest gegen dieselben Use-Case-Tests ist vorgesehen.
* `armasync-application` hängt produktiv nur von `armasync-core` ab; die In-Memory-Adapter dienen als Entwicklungsabhängigkeit der Tests.
* Änderungen an diesen Typen sind Vertragsänderungen und gehen über den Integrator.

## Belege

`crates/armasync-core/src/*.rs` (26 Unit-Tests), `crates/armasync-application/tests/sync_cases.rs` (11 Anwendungsfälle), Mutationsprüfung: Entfernen der Hashprüfung, der Revalidierung oder der Abbruchprüfung lässt je mindestens einen Test fehlschlagen. Betroffene Tasks: MIG-011; Folge: MIG-012, MIG-013, P2/P3.
