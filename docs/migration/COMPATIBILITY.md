# Datenformate und Kompatibilitätsstrategie

Status: Spezifikation für P0/P2/P4, noch kein fertiger Importer. Ziel ist kontrollierte Datenübernahme und Repositoryweiterverwendung, nicht das ungeprüfte Umbenennen serialisierter Klassen.

## Formatgrenzen

| Daten | Beobachteter Bestand | Strategie |
| --- | --- | --- |
| Einstellungen / Profile / Repositorydefinitionen | GZIP + Java-Serialisierung, feste Class-Deskriptoren/UIDs, teils `SealedObject` | einmaliger lokaler Export → versioniertes neutrales JSON → Rust-Import |
| Server-Metadaten | `.a3s/autoconfig`, `serverinfo`, `sync`, `changelogs`, `events`, über DAO/Serializer | Datenparser ohne Java-Klassenladen; synthetische Golden Fixtures pro Typ |
| Delta | `.zsync`, Rolling Checksums, MD4/BrokenMD4, SHA-1 | kompatibler Parser und Golden Vectors; geprüfter Voll-Download-Fallback |
| Nutzdaten | `.pbo`, `.ebo`, `.bikey`, ZIP und normale Moddateien | unverändert synchronisieren; keine Brandingersetzung in Inhalten |
| Launcher-Presets | HTML-Import/-Export | vorhandene Fälle als sprachunabhängige Fixtures übernehmen |
| Updateformate | GitHub/JSON/XML und alte Adressen | keine automatische Fortsetzung alter Trust-/Serverkonfiguration; neue Updatepolitik |

P0 muss tatsächliche Verschlüsselungs-/Serialisierungsvarianten je Dateityp durch DAO-Aufrufe und Fixtures bestätigen. Nicht pauschal annehmen, jede Datei sei verschlüsselt oder jedes Feld identisch.

## Lokaler Migrationsweg

1. Quelle explizit wählen: altes AppData-/XDG-Verzeichnis, portable Installation oder Export. Keine Dateien automatisch verschieben/löschen; kein Import von Geheimnissen ohne Auswahl.
2. Auf einer Kopie arbeiten, Quelldateien mit Hashes inventarisieren. Exporthilfe bei Bedarf als separaten Legacy-Task auf Basis des eingefrorenen Commits entwickeln; niemals `legacy` selbst verändern. Der Hilfsquellcode bleibt in einem separaten `migration-tools/...`-Branch bzw. Quellarchiv und gehört nicht ins Rust-Release.
3. JVM-Exporthilfe nur offline mit vorhandenen Filtern und engen Ressourcenlimits ausführen; keine frei gewählten Netzwerkobjekte deserialisieren. Authentifizierungsdaten nur auf expliziten sicheren Migrationsweg; der neutrale Export enthält keine Passwörter, Token oder alten eingebauten Schlüssel.
4. Neutrales JSON mit `schema_version`, Quellformatversion, IDs/Zuordnungen, Repositories, Profilen, Gruppen, Prioritäten und relevanten Einstellungen erzeugen. Sensible URL-Bestandteile redigieren oder als CredentialRef auslagern. Neu anmelden statt Kennwörter im Export speichern.
5. Rust-Import validiert Schema, Größen, Pfade, IDs und Konflikte; zeigt Voransicht und Bericht. Staging plus atomarer Commit in neuen Datenroot. Importjournal ermöglicht Wiederholung ohne doppelte IDs.
6. Quell-/Zielanzahlen, Feldwerte, Gruppenreihenfolge und Pfadzuordnung vergleichen; Erfolg erst nach finaler Validierung melden. Abbruch vor Commit lässt bisherige Daten intakt. Backup und Journal ermöglichen Rückkehr.

Der normale Betrieb der Rust-Version benötigt **keine JVM**. Eine vorübergehende Offline-Exporthilfe ist ein separates Übergangswerkzeug mit klarer Herkunft und Voraussetzungen.

## Direkte Nutzung vorhandener Repositoryserver

Ein Offline-Konfigurationsexport allein reicht nicht, wenn ein Client nach jedem Serverupdate neue `.a3s`-Metadaten lesen muss. Dafür `armasync-compat` als **rein datenorientierten, begrenzten Parser** entwickeln. Typen neutral `LegacyDescriptor`, `LegacyRepository`, etc. nennen. Keine JVM einbetten, keine Java-Klassen laden oder als neue Rust-Klassennamen nachbilden.

Class-Deskriptoren mit alten Packagezeichenfolgen sind Daten des bestehenden Drahtformats. Sie dürfen nur in eng begrenzten Kompatibilitätstabellen/Fixtures vorkommen, einschließlich Erklärung, Herkunft und Tests. Keine allgemeine Regexfreigabe für alte Namespaces. Wenn die Forderung „keine alten Zeichenfolgen“ auch diese Formatbytes ausschließen soll, ist direkte Altserver-Kompatibilität technisch nicht dasselbe Ziel: dann braucht es eine ausdrücklich entschiedene Umstellung aller Server auf ein neues Format. Das ist keine stille Voreinstellung dieser Planung.

Initialziel: Lesen bestehender Metadaten. Vor Repository-Schreibfunktion in P4 muss ADR-0003 entweder interoperables Legacy-Schreiben oder ein neues versioniertes Format mit verbindlichem Servercutover festlegen. Keine Behauptung vollständiger Legacy-Client-Unterstützung ohne gegenseitigen Build-/Read-/Sync-Roundtrip.

## Parser- und Dateisicherheit

- Limits für komprimierte/entpackte Bytes, Rekursion, Objekte, Referenzen, String-/Arraylängen und Laufzeit vor Allokation prüfen; Zyklen, ungültige Referenzen, unbekannte Deskriptoren und UIDs ablehnen. P0-Limits mit großen echten Strukturen validieren.
- SHA-1/MD4 nur für Altprotokollintegrität einsetzen, nicht als Updateauthentizität. Neue Formate starke Hashes verwenden; Altformatchecks nicht unbemerkt ändern.
- Pfade als relative Repositorypfade behandeln. `..`, Root-/Drive-/UNC-/Devicepfade, alternative Separatoren, NUL und URL-encoding-bedingte Ausbrüche ablehnen.
- Windows-reservierte Namen, Alternate Data Streams, Junctions/Symlinks, Linux-Case-Sensitivity und Unicodekollisionen prüfen. Keine stille Zusammenlegung unterschiedlicher Dateien.
- Rootcontainment über den tatsächlichen Dateizugriff absichern; rein lexikalisches `normalize` reicht gegen Symlink-/TOCTOU-Angriffe nicht.
- `.part`-Dateien an URL/Revision/Validator/erwartete Größe binden. Server ohne Range, ignorierter Range, geänderter ETag, kurze Antwort und 416 sauber behandeln.
- Fertige Dateien zuerst im Ziel-Dateisystem stagen, prüfen und atomar tauschen; Cross-Device-/Disk-Full-/Windows-Lockfälle explizit behandeln.
- Exakter Abgleich löscht ausschließlich im bestätigten verwalteten Zielbereich; Vorschau, Besitzinventar und sichere Behandlung fremder Dateien. Keine automatische Löschung fremder Mods aufgrund Namensähnlichkeit.

## Erforderliche Fixtures

Leeres/kleines/großes Repository; mehrere Profile; Event/Changelog/Serverinfo; Sonderzeichen und lange Pfade; alte und neuere SFTP-Felder; verschlüsselte synthetische Konfiguration; kaputte GZIPs; unbekannte Klassen/UIDs; Zyklen/Größenbomben; zsync-Grenzblöcke und BrokenMD4-Fälle; ZIP-Traversal; Case-/Unicodekollisionen. Jeden Generator, Ausgangs-Commit und erwartete Ergebnisse einchecken. Lizenz und Herkunft der Fixtures separat nennen.
