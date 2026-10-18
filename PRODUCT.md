# Produkt: ArmaSync

<!-- impeccable:product-schema 1 -->

## Plattform

Windows- und Linux-Desktop mit eigenständiger headless CLI. Native Desktop-Anwendung; mobile/webbezogene Plattformwerte des Designwerkzeugs passen hier nicht und begründen keine mobile Ausweitung.

## Zweck und Nutzer

ArmaSync synchronisiert Arma-3-Modbestände, verwaltet Repositories und unterstützt Profile sowie Spielstart. Aus dem Bestand abgeleitete Nutzergruppen: Community-Mitglieder, Repository-Administratoren und Betreiber automatisierter Jobs. Dies sind Arbeitshypothesen für Usability-Tests, keine durchgeführte Nutzerforschung.

## Bestätigte Anforderungen

- Arma3Sync wird im Zuge der Rust-Portierung zu **ArmaSync**; gesamter aktiver Auftritt **Lynxnode.it**.
- Rust; modularer Core, CLI und GUI. Die GUI bleibt primäre Bedienoberfläche.
- Windows und Linux; Linux zunächst **nur Synchronisation**, vom Nutzer am 2026-10-18 bestätigt. Kein Proton-/Steam-Spielstart als Erstziel.
- Modern, intuitiv und zugänglich; vor GUI-Implementierung operationalisieren.
- AGPLv3 als Ziellizenz; Herkunft bestehender Bestandteile erhalten.
- Codex und Claude arbeiten parallel; Änderungen und letzte Integration nachvollziehbar dokumentieren.
- `master`, `development`, `legacy`, `testing`; Freigabe über `testing` nach `master`.

## Evidenz

Java-Quellen, Java-Updater, Release-/Installerskripte und 20 Java-Testklassen liegen vor. Inventar und Paritätsmatrix dokumentieren gefundene Funktionen. Es liegen keine neuen freigegebenen Lynxnode.it-Logos, kein bestätigter Update-Endpunkt und keine Rust-Leistungsdaten vor.

## Offene Entscheidungen

GUI-Toolkit, minimale Betriebssysteme, unterstützte Linux-Distributionen, erste Rust-Versionsnummer und Update-/Signaturinfrastruktur werden in P0/P1/P6 entschieden. ADRs kennzeichnen Vorschläge. Ein mit Rust integrierbares natives GUI-Toolkit ist bevorzugt; Web-Frontend-Code nicht stillschweigend einführen.

## Qualitätsprinzipien

Gemeinsame Logik; vorhersehbare Dateioperationen; sichere Migration mit Rückweg; zugängliche Bedienung; überprüfbare kleine Schritte. Deutsch und Englisch sind als erste UI-Sprachen vorgeschlagen, ohne sprachliche Altbestände pauschal zu löschen.
