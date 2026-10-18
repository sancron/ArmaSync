# Gemeinsame Arbeitsanweisungen: ArmaSync / Lynxnode.it

Verbindlicher Projekteinstieg für Codex, Claude und Menschen. `CLAUDE.md` verweist auf dieselben Regeln. Direkte aktuelle Anweisungen des Projektverantwortlichen haben Vorrang; Änderungen dauerhafter Anforderungen dokumentieren.

## Vor jedem Arbeitsbeginn lesen

1. `PRODUCT.md`, `docs/migration/STATUS.md`, `docs/changes/LAST_CHANGE.md`.
2. `docs/ai/WORKFLOW.md` und die eigene zugewiesene Datei in `docs/tasks/`.
3. `docs/migration/ROADMAP.md`, `docs/migration/ARCHITECTURE.md` und passende ADRs in `docs/adr/`.
4. Bei Formaten: `docs/migration/COMPATIBILITY.md`; bei UI: `docs/migration/UI_REQUIREMENTS.md`; bei Branding/Lizenz: `docs/migration/BRANDING.md` und `LICENSING.md`.

Prüfe Git-Status, Branch, HEAD und vorhandene Änderungen. Nenne Task-ID, Basis, Arbeitsbereich und geplante Prüfung. Behaupte keine Implementierung oder Testergebnisse ohne Nachweis.

## Projektziel

- Produkt **ArmaSync**, sichtbare Marke/Publisher **Lynxnode.it**, neue technische ID `it.lynxnode.armasync`, Rust-Crates `armasync-*`.
- Modularer Rust-Core, separate GUI und eigenständige headless CLI. Keine doppelte Geschäftslogik; keine GUI-Laufzeit als CLI-Abhängigkeit.
- Windows und Linux als Synchronisationsziele; Spielstart zunächst nur Windows. Linux-Spielstart/Proton ist außerhalb der ersten Migration.
- Neue unabhängige Rust-Quellen: `AGPL-3.0-only`. Bestehende GPL-/Drittlizenzen und Urheberhinweise erhalten; kein pauschales Umetikettieren.
- Keine alten Team-/Länderpräfixe in neuen Typen, Modulen, Namespaces, Assets oder UI. Eng begrenzte Format-/Rechtsausnahmen stehen im Brandingplan. Kein globales Ersetzen von `fr`, `sof`, `soe` oder `a3s`.
- Nutzerdaten erhalten; Dateioperationen atomar, abbrechbar und plattformübergreifend sicher gestalten.
- Zugängliche GUI als Release-Kriterium behandeln. Leitvorgabe: vertraute Abläufe und Informationsstruktur der Baseline, aber moderner, intuitiver und zugänglicher; keine Altmarken/-grafiken (siehe `docs/migration/UI_REQUIREMENTS.md`, Abschnitt „Leitvorgabe“).

## Zusammenarbeit und Git

- Task-Branch → `development` → `testing` → `master`. Nicht direkt auf `master`, `testing` oder `legacy` entwickeln. `legacy` bleibt am dokumentierten Baseline-Commit.
- Parallele Systeme verwenden eigene Worktrees und Task-Branches: `codex/<task-id>-<thema>` beziehungsweise `claude/<task-id>-<thema>`.
- Ein im Task-Board benannter Integrator je Batch. Eigene Task-Datei, eigener Änderungsbericht und eigener Handoff. Gemeinsamen Status und Lockdateien nur durch den Integrator ändern.
- Keine fremden Änderungen verwerfen, fremde Worktrees verändern, Branches zwangsweise umbiegen oder Force-Pushes. Schnittstellenänderungen vor Implementierung als ADR/Vertrag abstimmen.
- Routinearbeit im zugewiesenen Umfang selbständig abschließen. Bei echten Anforderungskonflikten, fehlenden Zugangsdaten oder irreversibler Veröffentlichung eskalieren; keine pauschalen Rückfragen für jeden Schritt.

## Implementierung und Qualität

- Kleine vertikale Schritte gemäß Roadmap. Java erst nach belegter Funktionsparität entfernen.
- Verhalten aus Quellcode und Fixtures herleiten; alte Dokumentation kann veraltet sein. Keine erfundenen Kompatibilitätszusagen.
- Core ohne GUI-/Betriebssystem-Abhängigkeiten; I/O über Ports und Adapter. Netzwerkbibliotheken isolieren.
- Fehler typisieren; keine Secrets in Logs, Prozessargumenten, Fixtures oder JSON-Ausgaben. TLS-/SSH-Vertrauensprüfung nicht abschalten.
- CLI mit stabilen Exitcodes, `--json`, `--no-input` und verständlicher Hilfe; destruktive Aktionen mit expliziter Auswahl und Vorschau.
- Neue Abhängigkeiten auf Wartung, Lizenz, Systembedarf und Features prüfen; Versionen bei Implementierung verifizieren und pinnen. `Cargo.lock` für die Anwendung einchecken.
- Java: `gradle test :updater:test --console=plain`. Nach Erstellung des Rust-Workspace: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`. CLI zusätzlich isoliert ohne GUI bauen. Nicht verfügbare Plattform-/GUI-Prüfungen offen benennen.
- Tests nach Risiko: Datenintegrität, Parser, Pfadgrenzen, Netzwerkfehler, Zustandswechsel und GUI-/CLI-Parität. Dokumentation auf Links, Widersprüche und mit `git diff --check` prüfen.

## Jede Übergabe enthält

Task-ID; Ausgangs- und Ergebnis-Commit; Verhalten und Dateien; Prüfungen samt Resultaten; ungeprüfte Bereiche; Risiken; nächster konkreter Schritt. Bericht in `docs/changes/`, Handoff in `docs/handoffs/`. Der Integrator aktualisiert `STATUS.md`, `LAST_CHANGE.md`, Roadmap, Paritätsmatrix und Changelog bei Übernahme. Details: `docs/ai/WORKFLOW.md`.
