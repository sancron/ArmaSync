# ADR-0001 — Modularer Rust-Workspace

Datum: 2026-10-18. Status: **angenommen für Modulgrenzen**, aus dem Nutzerauftrag abgeleitet; noch nicht implementiert.

## Kontext

GUI und CLI müssen auf Windows und Linux denselben verlässlichen Funktionskern verwenden. Der Java-Bestand hat zwar Layer, aber statische Zustände und monolithische Dienste. Ein direkter Klassenport würde diese Probleme übernehmen.

## Entscheidung

Cargo-Workspace mit unabhängig baubarem Core, Application-Services, injizierten Adaptern und separaten CLI-/GUI-Binaries. Zyklen und GUI-Abhängigkeiten in CLI/Core sind ausgeschlossen. Compat/Updater werden separat isoliert, sobald konkrete Funktion entsteht. [ARCHITECTURE.md](../migration/ARCHITECTURE.md) definiert Grenzen und Zielstruktur.

Nicht gewählt: CLI als Shell um die GUI; zwei unabhängige Syncimplementierungen; eingebettete JVM als Dauerlösung; unnötige Microservices. Native GUI-Technologie wird separat entschieden. Konkrete Versionspins sind keine heutige Festlegung.

## Folgen und Abnahme

Früher Vertrag für Commands/Events/Fehler nötig; ein Eigentümer für Workspace und Lockdatei. CLI muss in einer Umgebung ohne GUI-Systembibliotheken baubar sein. Dieselben Use-Case-Tests gelten für beide Frontends. Verifikation in P1/P3.
