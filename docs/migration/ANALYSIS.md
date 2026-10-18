# Projektanalyse vor der Rust-Migration

Analysiert am **2026-10-18**, Ausgangs-Commit **`b7753cea18b2ac2cc57e29f926901997a4de8570`** auf `master`, jetzt unverändert als `legacy`. Die Arbeitskopie war vor Beginn sauber. Repository: `https://github.com/sancron/ArmaSync.git`; vorhanden waren lokal und remote nur `master`. Dies ist die Analyse des vorliegenden Fork-Stands, nicht die Behauptung, der historische Erststand des ursprünglichen Upstream-Projekts sei gefunden worden.

## Umfang und Aussagegrenzen

Strukturinventur aller 515 getrackten Dateien; gezielte Untersuchung der Einstiegspunkte, Build-/Releasekonfiguration, Datenzugriffe, Domänenmodelle, Repository-/Transferdienste, Java-Serialisierung, Hash-/Deltaimplementierung, GUI-Struktur, CLI, Plattformintegration, Sicherheitsrichtlinien, Tests und Lizenzhinweise. Vollständiger Pfadindex: [INVENTORY.md](INVENTORY.md).

Die Analyse ist eine Migrations- und Architekturprüfung, kein vollständiges Audit jeder Ausführungskante. Binäre JARs wurden nicht dekompiliert; produktive Server, Installer, GUI und Linux wurden nicht ausgeführt. Es liegen keine repräsentativen Nutzerdatensätze oder Netzwerkbenchmarks vor. Aussagen über Laufzeitparität bleiben deshalb Abnahmeaufgaben.

## Gemessener Bestand

| Kennzahl | Ausgangsstand |
| --- | --- |
| Getrackte Dateien | 515 |
| Java-Hauptquellen | 317 Dateien, 53.055 physische Zeilen inklusive Kommentare/Leerzeilen |
| Java-Updater | 27 Hauptquelldateien |
| Java-Testklassen | 20: Hauptanwendung 14, Updater 6 |
| Ausgeführte JUnit-Tests | 64: 63 erfolgreich, 1 übersprungen, 0 Fehler/Failures |
| App-Version in `version.properties` | `2026.4.15` |
| Hauptanwendung / Updater | Java 25 / Java 21 |
| Build | Gradle Java + Application, zwei Projekte, kein Wrapper |
| CI | Eine WinGet-Publish-Pipeline; keine vorhandene allgemeine Build-/Testmatrix |

Aufteilung der 317 Hauptquelldateien nach oberstem Package: UI 108, Domain 39, Service 32, DAO 31, Exceptions 26, Controller/Observer 21, DTO 20, Konstanten 18, Jazsync 12, Utilities 3, Console 3, Main 2, Jazsyncmake 2.

## Systemstruktur und Datenfluss

```text
ArmA3Sync.main
  ├─ Swing GUI → Facade → Services / Worker / Observer
  └─ CommandConsole / CommandLine → CommandGeneral → Services
                          ↓
               Domain + DTO + DAO
                          ↓
          lokale Dateien / Netzwerk / Betriebssystem

unabhängiger Java-Updater → Updatekonfiguration → Download → Dateiaustausch
Gradle + PowerShell + NSIS + Go-Bootstrapper → Distribution
```

Die bestehenden Layer liefern brauchbare fachliche Grenzen, aber noch keinen unabhängig instanziierbaren Core: viele statische DAO-Zustände, globale Pfadkonstanten, Observable-/Observer-Verträge und auf lange Abläufe zugeschnittene Dienste. Ein bloßes Übersetzen der Klassen würde diese Kopplung konservieren. Die neuen Frontends müssen über gemeinsame Commands, Zustände und Ereignisse auf denselben Core zugreifen.

### Einstieg, CLI und Konfiguration

`src/main/java/fr/soe/a3s/main/ArmA3Sync.java` vereint Moduswahl, Debugargumente, Single-Instance-Verhalten und GUI-/CLI-Start. Er kennt unter anderem `-console`, `-build`, `-check`, `-sync`, `-extract`, `-update`, `-run` und `-unlock`. `ConsoleCommands` enthält zusätzlich interaktive Verwaltungsbefehle. `CommandLine.java` beendet auch mehrere Fehlerfälle mit `System.exit(0)`; das verhindert verlässliche Skriptauswertung. Neue CLI: klare Subcommands, eindeutige Exitcodes, getrennte Daten-/Diagnoseausgabe, keine GUI-Initialisierung.

`ApplicationPaths.java` trennt Installation, Konfiguration, Daten und Cache, unterstützt portable Daten und kopiert alte Verzeichnisse ohne Überschreiben. Diesen Datenverlustschutz übernehmen, nicht die alten Brandingpfade. `a3s.cfg`, `a3s.prefs`, Profile und Repositorydefinitionen sind keine frei lesbaren JSON-Dateien.

### Repository und Synchronisation

`RepositoryService`, `ConnectionService`, `dao/repository`, `service/administration` und `service/synchronization` decken Repository-CRUD, Build, Revisionen, Metadaten, Ereignisse, Modvergleich, Dateiübertragung, Resume, Entpacken und optionalen Abgleich überschüssiger Dateien ab. `RepositoryBuildProcessor` erzeugt unter anderem SHA-1-Signaturen und `.zsync`-Daten.

`DataAccessConstants` benennt `.a3s/{autoconfig,serverinfo,sync,changelogs,events}`, `.a3s.profile`, `.a3s.repository`, `.a3s.autoconfig`, `.part`, `.zsync`, `.pbo`, `.ebo`, `.pbo.zip`, `.bikey`. Diese sind Format-/Spielverträge; sie lassen sich nicht wie eine Fenstertitelzeichenfolge ersetzen.

**Größtes Migrationsrisiko:** `A3SFilesAccessor` verwendet GZIP-komprimierte Java-Objektserialisierung. Objektfilter erlauben explizit das alte Package; die Modelle haben feste `serialVersionUID`s. Je nach DAO kommt ein `SealedObject` hinzu. Das Umbenennen von Java-Packages bricht bestehende Daten und möglicherweise Server-Metadaten. Vor Umbenennung und vor Löschen des Java-Codes müssen Fixtures, Importstrategie und Formatgrenzen nachgewiesen sein.

`jazsync`/`jazsyncmake` enthalten Rolling Checksums, MD4, eine `BrokenMD4`-Variante, Metadatenlesen/-schreiben und Deltaoperationen. Ein gleichnamiges Rust-Crate allein beweist keine Bytekompatibilität. Bei zunächst fehlender Delta-Parität ist ein integritätsgeprüfter Voll-Download ein möglicher transparenter Fallback; kein stiller Verlust von Resume oder Reparaturverhalten.

### Netzwerk

Es gibt Adapter für HTTP(S), FTP/FTPS, SFTP und WebDAV; SOCKS ist ein Proxytyp. Downloads, Uploads, Serverchecks und Metadatenzugriff benötigen eine getrennte Fähigkeitsmatrix. README und Code unterscheiden sich bei Details: FTPS-Uploads werden für neue Auswahlen vorübergehend ausgeblendet, bleiben für bestehende Konfigurationen vorhanden. SFTP verwendet Apache MINA SSHD und derzeit Passwortauthentifizierung. Nicht behaupten, Schlüsselverwaltung sei schon implementiert.

Uploadparallelität wird als 1–10 mit Default 4 beschrieben und ist von Check-/Downloadparallelität getrennt. Wiederverwendete Sitzungen und begrenzte Worker sind bewusst vorhandene Optimierungen. Im Rust-Core müssen Limits, Backpressure, Timeout, Retry, Abbruch und Reihenfolge der Metadatenveröffentlichung explizit sein. Ein Upload darf keinen neuen Index veröffentlichen, bevor die referenzierten Dateien verfügbar sind.

### GUI und UX

Swing mit FlatLaf, eigenen Theme-Tokens, Geist-Schriften und SVG-Icons; vorhandene Dark-/Light- und Kontrastarbeit ist erkennbar. Es ist kein unverändertes Standard-Swing-UI. 108 UI-Klassen und große Panels zeigen jedoch viele konzentrierte Verantwortlichkeiten. Tastatur-/Screenreader-Nutzbarkeit ist nicht durch die vorhandenen Tokentests bewiesen.

Neue GUI-Informationsarchitektur: Repositories, Synchronisation, Profile/Start, Administration, Einstellungen und Hilfe/Lizenzen. Der wichtigste Ablauf ist Import → Ziel prüfen → Änderungen prüfen → synchronisieren → verifizieren. Lange Tasks brauchen sichtbaren Status, Abbruch, Fehlerkontext und Wiederaufnahme. Details: [UI_REQUIREMENTS.md](UI_REQUIREMENTS.md).

### Launcher und Plattformintegration

Profile, Addon-Gruppen/Reihenfolge, DLC/CDLC, Serverfavoriten, Preset-Import/-Export, externe Anwendungen, Windows-Registry, Autostart, Shortcuts, Prozessprüfung und TeamSpeak/TFAR/ACRE-Werkzeuge sind getrennte Funktionen. `LauncherDAO` enthält Windows-spezifische Prozesspfade und ältere String-/Shell-Aufrufe. Ein vorhandener Linux-Paketbuild beweist keine gleichwertige Spieleintegration.

Ziel: Windows-Spielstart mit strukturierten Argumenten und geprüften Pfaden. Unter Linux zunächst Synchronisation einschließlich Repositoryverwaltung; nicht verfügbare Windowsfunktionen klar ausblenden/erklären und über CLI mit definiertem Fehler melden. Sachliche Arma-3-Namen bleiben erhalten, obwohl das Tool ArmaSync heißt.

### Updater und Release

Separates Java-21-Projekt mit TOML-Konfiguration, GitHub-Releases, JSON-Manifest und XML-Fallback. SHA-256-Downloadprüfung und Schutz beim Entpacken/Verwalten von Dateien sind vorhanden. Eine Prüfsumme aus demselben kompromittierten Kanal ist keine unabhängige Herausgebersignatur. Der neue Updater braucht ein gesondertes Vertrauensmodell, Channels, signierte Metadaten/Pakete, Staging, Rollback und Tests für laufende Windows-Binaries.

Packaging umfasst Gradle-Distributionen, `jlink`/`jpackage`, NSIS, PowerShell, einen Go-Bootstrapper zur Java-Erkennung und WinGet-Manifeste. Diese Kombination wird durch native Rust-Pakete ersetzt; Java-Laufzeit und Go-Java-Bootstrapper gehören nicht ins finale Rust-Paket. NSIS kann als unabhängiges Packagingwerkzeug weiterverwendet werden, falls passend.

Aktuelle Releaseautomatisierung enthält Publisher, Paket-ID und Downloadadressen des bisherigen Forks. Ein automatisches Umschalten auf frei erfundene Lynxnode.it-Updatepfade wäre falsch. Bis Domainpfade, Schlüssel und Hosting bestätigt sind, Self-Update der Rust-Version deaktiviert lassen; normalen lokalen Betrieb nicht blockieren.

## Wartbarkeit und Dokumentationsdrift

Größte Dateien: `AddonsPanel.java` 1.938, `MainPanel.java` 1.691, `RepositoryService.java` 1.639, `DownloadPanel.java` 1.555, `LauncherOptionsPanel.java` 1.195, `EventsPanel.java` 1.138 Zeilen. Diese Bereiche nach Use-Cases aufteilen, nicht je eine riesige Rust-Datei erzeugen.

Der ursprüngliche README nennt Stable `2026.3.14`, die tatsächliche Version ist `2026.4.15`. `TESTING.md` beschreibt hauptsächlich zwei Persistenztests, vorhanden sind 20 Testklassen. `MIGRATION_NOTES.md` sagt, alle Libraries kämen lokal; Gradle nutzt überwiegend Maven Central. `PERFORMANCE_PLAN.md` bezeichnet parallele FTP-Uploads als deaktiviert, während neuere README-/Codepfade parallele Uploads beschreiben. Build-Wrapper-Empfehlungen und Versionsangaben widersprechen sich ebenfalls. Deshalb: ein Statusdokument, ein Integrationsjournal, Entscheidungshistorie und Tests als Evidenz, historische Pläne klar kennzeichnen.

## Sicherheits- und Lizenzbefunde

| Priorität | Beleg / Risiko | Konsequenz für Migration |
| --- | --- | --- |
| Hoch | Java-Serialisierung aus Dateien/Netzwerk; bereits vorhandener ObjectInputFilter begrenzt Typen/Größe | Rust-Parser ohne Codeausführung, Limits/Fuzzing und restriktive Deskriptorvalidierung. Vorhandene Filter nicht entfernen. |
| Hoch | `EncryptionProvider.java` mit fest eingebautem AES-Schlüssel und unspezifiziertem AES-Modus | Kein sicherer Secret-Speicher. Neue Credentials über OS-Keyring/Secret Service; keine Secrets in JSON migrieren. |
| Hoch | Pfade aus Repository/ZIP und exakter Abgleich können Dateien verändern/löschen | Traversal-, Symlink-/Junction-, Case-/Unicode-, TOCTOU- und Löschgrenzen testen; Vorschau und Besitznachweis. |
| Hoch | Update-Hashprüfung vorhanden, Signaturvertrauen nicht belegt | Herausgebersignaturen und Rollback vor Self-Update-Freigabe. |
| Mittel | SFTP-Host-Key-Vertrauen im betrachteten Verbindungsweg nicht explizit konfiguriert | Abhängigkeitsdefault noch prüfen; künftig known_hosts/Pinning und Schlüsselwechselzustand verbindlich. Keine unbelegte Behauptung eines bestätigten Exploits. |
| Mittel | TLS-Abschaltung mit lokaler/explicit-override-Policy möglich | Sichere Defaults erhalten; keine globale Produktionsoption zur Abschaltung. |
| Mittel | Lokale JAR-Duplikate, ältere Fassungen und transitive Abhängigkeiten | Tatsächlich verwendete Artefakte inventarisieren; keine CVE-Zusage ohne Scan. |
| Hoch | GPLv3-Bestand, eigene GPL-2.0-or-later-Header und fremde Assets | Rechte-/Herkunftsregister; AGPL-Ziel nicht durch bloßes Ersetzen der Lizenz erfüllen. |

## Schlussfolgerung für die Umsetzung

Eine modulare schrittweise Neuentwicklung ist sinnvoller als ein Klassen-für-Klassen-Port. Zuerst Format-/Lizenzgrundlage und unabhängigen Core absichern; früh eine echte GUI samt einem vollständigen Sync-Ablauf liefern; danach Transporte, Administration, Launcher und Updater nachziehen. Java erst entfernen, wenn die [Paritätsmatrix](FEATURE_PARITY.md) und [Release-Abnahme](VALIDATION.md) erfüllt sind. Nächste Tasks: [ROADMAP.md](ROADMAP.md).
