# Offene Rechtefragen (Tracker aus MIG-002)

Stand: 2026-10-18, geführt vom Integrator. Quelle und Belege: [RIGHTS_INVENTORY.md](../migration/RIGHTS_INVENTORY.md). **Alle Fragen sind offen.** Dieses Dokument und MIG-002 erteilen keine Lizenz-, Rechte- oder Übernahmefreigabe und ersetzen keine Rechtsberatung. Jede Frage ist ein eigenständig bearbeitbarer Punkt mit empfohlener Zuständigkeit; sie wird nur durch eine dokumentierte Entscheidung des Projektverantwortlichen (ggf. mit Rechtsberatung) geschlossen.

| ID | Frage (Kurzfassung) | Status | Empfohlene Zuständigkeit | Blockiert (spätere Portierungen / Aktionen) |
| --- | --- | --- | --- | --- |
| RQ-01 | Rechteinhaber und gültige Lizenzfassung je Teilbaum der eigenen Java-/Go-Quellen | offen | Projektverantwortlicher / Integrator | Übernahme oder Portierung jeglicher Bestandslogik in AGPL-3.0-only-Dateien; SPDX-Header auf Bestandsdateien |
| RQ-02 | Jazsync/MD4 (GPLv2-or-later, mehrere Autoren): Rust-Port unter AGPL-3.0-only zulässig? | offen | Projektverantwortlicher mit Rechtsberatung | Portierung/Übersetzung von Jazsync, MD4, BrokenMD4, `DataRange` (F08, MIG-0xx in P4); bis dahin nur unabhängige Neuimplementierung nach Formatbeschreibung und MIG-001-Fixtures |
| RQ-03 | JTattoo, junique (kein Lizenzbeleg im JAR), jshortcut (LICENSE prüfen): Herkunft und Weitergaberechte | offen | Integrator / Paketierer | Weitergabe der Java-Altversion mit diesen JARs; Rust-Ersatz für Look-and-Feel, Single-Instance, Shortcuts (F17) darf sie nicht übernehmen |
| RQ-04 | Herkunft/Rechte der alten App-/Installer-Icons und der SVG-Icons | offen | Branding-Verantwortlicher | Wiederverwendung dieser Grafiken in GUI, Installer, Paketen; bis dahin eigene Lynxnode.it-Assets |
| RQ-05 | Sardine-5.12-Lizenznachweis (keine Lizenzdatei im JAR) | offen | Integrator | Release-BOM der Java-Altversion; kein Einfluss auf Rust (WebDAV wird neu implementiert) |
| RQ-06 | Rust-Crates noch nicht gewählt: Lizenz-/Notice-Inventar | offen | Rust-Architekt / Integrator | Aufnahme jeder Abhängigkeit in `Cargo.toml` ohne Lizenzprüfung; erste Release-BOM |
| RQ-07 | Winget-Metadaten und NSIS-/Bootstrapper-Copyrights für neue Publisher-Identität | offen | Release-Verantwortlicher | Veröffentlichung neuer Pakete/Installer (P6) |
| RQ-08 | `LICENCE.md` ist GPLv3-Volltext: GPL-3.0-only oder -or-later für Bestandsquellen? | offen | Projektverantwortlicher / Rechtsberatung | SPDX-Zuordnung portierter Module; Umgang mit Bestandsheadern |

Die Tabelle wird bei Entscheidungen durch den Integrator fortgeschrieben (Datum, Entscheider, Beleg). Ungeklärte Fragen blockieren nur die genannten Aktionen, nicht neue unabhängig geschriebene Rust-Quellen.
