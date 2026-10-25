# Sync-Metadaten (`.a3s/sync`, `.a3s/serverinfo`): Vertrag für Scan und Plan (MIG-030)

Stand: 2026-10-19. Herkunft der Aussagen: (1) die synthetischen MIG-001-Fixtures `fixtures/legacy-formats/generated/valid/server/*/.a3s/{sync,serverinfo}` mit den aus der echten Baseline wieder eingelesenen Erwartungsdateien, (2) Verhaltensbeobachtung am Baseline-Quelltext (nur gelesen, nichts übernommen; siehe „Rechte“), (3) [FORMAT_INVENTORY](FORMAT_INVENTORY.md). Was **nicht** durch Fixtures belegt ist, steht unter „Nicht belegt“. Rahmen: [ADR-0006](../adr/0006-sync-metadata-and-http-transport.md), [ADR-0003](../adr/0003-legacy-formats.md).

## Was `.a3s/sync` enthält

Eine GZIP-komprimierte Objektserialisierung (Container wie bei `autoconfig`, siehe FORMAT_INVENTORY Abschnitt 1) mit **einem Baum**: Wurzelverzeichnis namens `racine` (Konstante des Formats, kein Pfadbestandteil), darunter Verzeichnisse und Dateien.

| Knoten | Serialisierte Felder (Wire-Namen) | Bedeutung |
| --- | --- | --- |
| Verzeichnis (`SyncTreeDirectory`, Version `d85feb3c784f59f8`) | `name`, `parent` (Rückverweis, **zyklisch**), `list`, `markAsAddon`, `updated`, `deleted`, `hidden` | `markAsAddon`: Verzeichnis ist ein Addon/Mod-Ordner der obersten Ebene; `hidden`: serverseitig gesetzt (Fixture `@second`); `updated`/`deleted` sind Client-Zustand und im Serverbestand immer `false` |
| Datei (`SyncTreeLeaf`, Version `7aced84d242712d7`) | `name`, `parent`, `sha1`, `size`, `compressedSize`, `compressed`, `updated`, `deleted` | `sha1`: SHA-1 des **unkomprimierten** Inhalts (40 Hexziffern, Kleinbuchstaben in den Fixtures); `size`: Länge des unkomprimierten Inhalts; `compressed`: auf dem Server liegt stattdessen `<name>.zip`; `compressedSize`: Länge dieser ZIP-Datei |

Nicht serialisiert (im Altclient flüchtig, daher nie in der Datei): Zielpfad, lokaler SHA-1, Fortschritt, „optional“.

Die Reihenfolge im Baum ist: zuerst Verzeichnisse, dann Dateien, je nach Name ohne Beachtung der Groß-/Kleinschreibung sortiert. Der Neuparser darf sich **nicht** auf die Reihenfolge verlassen und sortiert selbst.

Der relative Pfad einer Datei ist der Namensweg von der Wurzel (ohne `racine`) mit `/`; der Download-URL-Pfad ist `<Repositoryadresse>/<relativer Pfad>` und bei `compressed` zusätzlich mit Endung `.zip`.

`.a3s/serverinfo` (Version `6ad21056c345a7f9`): `revision` (ganze Zahl), `buildDate`, `numberOfFiles`, `totalFilesSize`, `numberOfConnections`, `compressedPboFilesOnly`, `noPartialFileTransfer`, `repositoryContentUpdated`, `hiddenFolderPaths` (Menge). Es dient als **Revisionsquelle** (`revision`, `buildDate`) und für Anzeigewerte. Beobachtung in den Fixtures: `numberOfFiles` (2) stimmt nicht mit der Zahl der Blätter des Baums (3, darunter die `.zip`-Variante) überein; die Zahl wird deshalb nur angezeigt, nie als Prüfwert benutzt.

## Was Scan und Plan benötigen

| Eingabe | Quelle | Verwendung |
| --- | --- | --- |
| Pfad, Größe, SHA-1 je Datei | `sync` | `FileEntry` des Core-Manifests (Hashart `Sha1`, nur Integrität des Altprotokolls, **keine Authentizität**) |
| Revision | `serverinfo.revision` (+ `buildDate`) | `Manifest::revision`, Neuvalidierung des Plans vor `apply` (ADR-0004) |
| `compressed`, `compressedSize` | `sync` | **MIG-030E (ADR-0008):** mit `compressedSize` von 1 Byte bis 2 GiB wird das Blatt als entpackte Datei installiert; das Archiv `<name>.zip` (exakt `compressedSize` Bytes) wird in eine Spool-Datei geladen, genau ein Eintrag mit Stored oder Deflate entpackt und Größe und SHA-1 geprüft. Ohne nutzbare Größe (`0`, zu groß): nicht unterstützt (Warnung, nie geladen, nie gelöscht) |
| `hidden` (und `serverinfo.hiddenFolderPaths`) | `sync`, `serverinfo` | Ordner, in denen fremde lokale Dateien nie als „extra“ gelöscht werden |
| `markAsAddon` | `sync` | Gruppierung der Pläne nach Addon (Anzeige) |
| lokale Dateien: Pfad, Größe, SHA-1 | Scan des Zielverzeichnisses (MIG-030C) | `LocalFile`; Datei „veraltet“, wenn sie fehlt oder ihr SHA-1 vom Server abweicht (Verhaltensbeschreibung der Baseline) |

Löschen: Nur im bestätigten verwalteten Zielbereich und nur bei ausdrücklicher Wahl (`delete_extra`, Standard aus; COMPATIBILITY „Parser- und Dateisicherheit“). Die Altfunktion „exakter Abgleich“ ist ein Nutzerschalter und wird nicht als Voreinstellung übernommen. Temporäre Teildateien (`.part`) und `*.pbo.zip` im Ziel werden vom Scan nicht als Fremddateien gewertet.

## Grenzen und Fehlerfälle (Pflicht für den Parser)

- Gleiche Grenzen wie `Limits` der Compat-Crate (Größe, Tiefe ≤ 100, Handles); für sehr große Repositories ist die Grenze für Elemente eigens zu bestimmen und an **synthetisch erzeugten großen Bäumen** zu messen (offen, siehe unten). Baumtiefe und Dateizahl werden vor dem Aufbau des neutralen Baums geprüft; die Konvertierung folgt **nie** dem `parent`-Verweis (Zyklen) und arbeitet iterativ oder tiefenbegrenzt.
- Pfadnamen sind Daten, nicht Pfade: jeder Name durchläuft `RelativePath` (kein `..`, kein absoluter/UNC/Laufwerkspfad, kein `\`, NUL, Windows-reservierte Namen, Alternate Data Streams, Prozentkodierung). Das Fixture `hostile-paths` enthält diese Fälle; ein **einziger** unsicherer Name macht die Datei nicht stillschweigend ungültig, sondern der Eintrag wird mit typisierter Warnung **nicht übernommen** und nie heruntergeladen; Fehlerhafte Struktur (falsche Klasse, Zyklus in Klassenkette, Rückverweis ins Leere) lehnt das **ganze** Dokument ab.
- Kollisionen (gleicher Pfad, Groß-/Kleinschreibung, Datei und Verzeichnis) lehnt `Manifest::new` ab; es wird nichts still zusammengeführt (das Fixture `hostile-paths` enthält `Case.pbo`/`case.PBO` und wird deshalb als Ganzes abgelehnt). **Befund (MIG-030A):** Der Core kennt keine Unicode-Normalisierung; NFC- und NFD-Schreibweise desselben Namens gelten als verschiedene Namen. Das ist auf NTFS und ext4 (Zielsysteme) korrekt; auf macOS (nicht zugesagt) würden sie kollidieren, dort wäre vor einer Unterstützung eine Normalisierungsprüfung nachzurüsten.
- SHA-1 muss 40 Hexziffern sein; Größe ≥ 0; negative Größen werden abgelehnt. Beobachtung: im Fixture `small` ist die Datei `example_core.pbo.zip` selbst als Blatt mit `compressed = true` und `compressedSize = 0` eingetragen; `compressedSize` 0 bei `compressed` ist daher **kein** Fehler und wird als „unbekannt“ behandelt (Bedeutung dieser Konstellation: nicht belegt).
- Unbekannte Klasse oder Versionsnummer: abgelehnt (`UnknownClass`), wie bei `autoconfig`. Ältere Dateiversionen mit anderen Feldmengen: **nicht belegt**.
- Keine Geheimnisse: die Datei enthält keine; die Redaktionsregel des Lesers bleibt aktiv.

## Nicht belegt

- Keine echten Repositories (> 10 000 Dateien) und keine historischen Dateiversionen; Größen-/Zeitverhalten der Grenzen ist ungemessen.
- `hiddenFolderPaths` (serverinfo) und `hidden` (Baum) sind beide beobachtet; ihr Zusammenspiel bei echten Servern ist nicht belegt. Der erste Schritt behandelt beide als „nie Fremddateien löschen“.
- Verhalten bei `.zip`-Blättern: das **Format echter Server-Archive ist nicht belegt** (Eintragsname, Anzahl der Einträge, Kompressionsmethode). Die MIG-001-Fixtures enthalten keine ZIP-Nutzlast. Entpacken ist nur mit selbst erzeugten Archiven getestet (ADR-0008); die Grenzen dort sind konservativ gewählt, keine Beobachtung.
- Case-Sensitivity-Verhalten auf Linux (Altclient vergleicht Namen ohne Beachtung der Schreibung) ist nicht geprüft (Linux-Prüfung zurückgestellt, siehe Prüfregel).

## Rechte (RQ-01 und andere)

- **RQ-01** (Rechteinhaber/Lizenzfassung der Bestandsquellen) bleibt **offen** und verbietet weiterhin die Übernahme oder Portierung von Bestandslogik in AGPL-3.0-only-Dateien. Dieser Vertrag und der darauf aufbauende Parser sind **unabhängig geschrieben**: Grundlage sind die Formatbeschreibung oben, die Fixtures und die hier festgehaltenen Verhaltensregeln in eigenen Worten; weder Quelltext noch Struktur oder Kommentare der Baseline werden kopiert, und der Implementierer darf die Baseline-Quelle während des Schreibens nicht als Vorlage verwenden. Wire-Namen und Versionsnummern stehen nur in einer eng begrenzten Tabelle (ADR-0003). Ob die Verhaltensregeln selbst schützbar sind, wird hier **nicht** entschieden; sollte der Projektverantwortliche RQ-01 mit einer Einschränkung beantworten, wird der Parser auf dieser Grundlage neu bewertet.
- **RQ-02** (Jazsync/MD4): nicht berührt; `.zsync`/MD4 sind nicht Teil von MIG-030. Der Voll-Download-Fallback braucht beides nicht.
- **RQ-03/RQ-05** (JARs): nicht berührt. **RQ-06** (Rust-Abhängigkeiten): siehe ADR-0006, Lizenztabelle; Notice-Inventar wird mit der ersten Aufnahme in `THIRD_PARTY_NOTICES.md` fortgeschrieben. RQ-04, RQ-07, RQ-08: nicht berührt.
