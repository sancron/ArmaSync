# ADR-0005 — Credentials, Datenverzeichnisse und versionierte Dokumente

Datum: 2026-10-18. Status: **angenommen** für Dokumentformat, Pfade und den Credential-Port (MIG-020); **offen** für die persistente Schlüsselbund-Anbindung. Bezug: [ARCHITECTURE](../migration/ARCHITECTURE.md), [DATA_MODEL](../migration/DATA_MODEL.md), [ADR-0004](0004-core-contracts.md).

## Kontext

Repositories, Profile und Einstellungen müssen versioniert, sicher gelesen und atomar geschrieben werden. Zugangsdaten dürfen weder in Dateien, Logs, Events noch Prozessargumenten landen. Der Core soll frei von OS- und Serialisierungsbibliotheken bleiben (ADR-0001).

## Entscheidung

1. **Modell im Core, Format in den Adaptern.** Validierte Typen (`Repository`, `Profile`, `Library`, `Settings`, `RepositoryUrl`, IDs) liegen im std-only Core; serde-Dokumentstrukturen und `schema_version` in `armasync-adapters`. Das Modell kann sich so ohne Formatbruch weiterentwickeln, das Format ohne Core-Abhängigkeiten.
2. **Zwei Dokumente**: `library.json` (Repositories + Profile, atomar zusammen) und `settings.toml`, beide mit `kind` und `schema_version`, Header zuerst geprüft, unbekannte Felder und neuere Versionen abgelehnt, 4 MiB Limit.
3. **Pfade** nach ARCHITECTURE als reine Funktion mit injizierter Umgebung; portabler Modus nur auf ausdrückliche Wahl; Legacy-Verzeichnisse werden nicht berührt.
4. **Credentials**: Core-Port `CredentialStore` mit `CredentialRef`; `Secret` mit geschwärztem `Debug`/`Display`. Dokumente enthalten ausschließlich die Referenz. **Keine Klartext-Persistenz**, auch nicht als Rückfall.
5. **Adapter jetzt**: `SessionCredentialStore` (Prozessspeicher). Persistente Secrets gibt es erst mit einem getesteten Schlüsselbund-Adapter; bis dahin fragt ein Frontend die Zugangsdaten pro Sitzung ab (GUI-Dialog; für die CLI ist eine `--no-input`-taugliche Zufuhr einzuplanen, derzeit nicht vorhanden).

## Bewertung Schlüsselbund (nicht umgesetzt, Stand 2026-10-18)

* Kandidat `keyring` (MIT OR Apache-2.0, aktiv gepflegt). Die aktuelle Hauptversion 4.2.0 verlangt **Rust 1.88** und widerspricht der MSRV 1.85; die 3.6.3 passt zur MSRV.
* `keyring` 3.x hat **keine** Standard-Features; die Plattform ist per Feature zu wählen (`windows-native`, `sync-secret-service`/`async-secret-service`, `apple-native`). Windows (Credential Manager) wäre mit `windows-native`, nur für `cfg(windows)`, umsetzbar und hier testbar. Unter Linux braucht `sync-secret-service` entweder die Systembibliothek `libdbus` (Entwicklungspakete im Build) oder eine zbus-basierte Variante mit größerem Abhängigkeitsbaum. Auf Headless-Hosts ohne Secret Service gibt es keinen Speicher.
* Nicht umgesetzt, weil Linux (Secret Service, Gnome-Keyring/KWallet) hier nicht testbar ist und eine halb getestete Sicherheitsfunktion schlechter ist als eine ehrlich fehlende.

**Vorschlag für den Folgetask (MIG-020B oder in MIG-031 integriert):** (a) Windows-Adapter mit `keyring` 3.6.x `windows-native` auf `cfg(windows)`; (b) Linux erst nach Test auf einem Desktop mit Secret Service, Entscheidung zwischen libdbus-Systembedarf und zbus; (c) Entscheidung, ob MSRV auf 1.88 angehoben wird (dann `keyring` 4.x); (d) Verhalten ohne Schlüsselbund: klare Meldung `Config`, Sitzungsspeicher als Alternative, nie stilles Klartext-Speichern. Die Entscheidung über MSRV und Linux-Backend trifft der Projektverantwortliche bzw. der Integrator mit Begründung.

## Nicht gewählt

Verschlüsselte Datei mit eingebautem Schlüssel (der Altbestand nutzte eingebaute Schlüssel; kein Vertrauensgewinn, siehe COMPATIBILITY); Klartext-Datei mit Dateirechten; Secrets in Umgebungsvariablen als Persistenz; ein Dokument je Entität (keine atomare Konsistenz); Rust-Layout-Serialisierung.

## Folgen

* GUI und CLI binden `LibraryStore`/`SettingsStore`/`DataRoots` ein (MIG-031); der GUI-Prototyp nutzt bis dahin eigene simulierte Typen.
* Eine Schemaänderung erhöht `schema_version` und bringt Migration samt Test mit; ältere Dateien werden nie überschrieben, ohne ein Backup zu halten.
* MIG-021 (Import) schreibt ausschließlich über diese Stores in ein Staging und ersetzt erst nach Validierung.
