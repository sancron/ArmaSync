# ADR-0008 — Gepackte Blätter (`.zip`): Entpacken mit festen Grenzen (MIG-030E)

Datum: 2026-10-25. Status: **angenommen** für MIG-030E; umgesetzt und lokal unter Windows getestet. Bezug: [SYNC_METADATA](../migration/SYNC_METADATA.md), [ADR-0006](0006-sync-metadata-and-http-transport.md) (Entscheidung 3), [ADR-0007](0007-transfer-ports.md), [COMPATIBILITY](../migration/COMPATIBILITY.md) („Parser- und Dateisicherheit“), [LOCAL_FS_THREATS](../migration/LOCAL_FS_THREATS.md), [RELEASE_BLOCKERS](../migration/RELEASE_BLOCKERS.md) (RB-01 bleibt offen).

## Kontext

Der Server markiert eine Datei im Metadatenbaum als `compressed`. Die unkomprimierte Datei hat Größe und SHA-1 im Baum, auf dem Server liegt stattdessen `<name>.zip` mit der Größe `compressedSize`. Bisher wurden solche Blätter nur gewarnt und nicht installiert. MIG-030E soll sie sicher installieren. Dabei ist der Server-Eingabe nicht zu trauen: Größen, Namen und Archivinhalt können beliebig sein.

Das **Format echter Server-Archive ist nicht belegt**. Die MIG-001-Fixtures enthalten keine ZIP-Nutzlast und `compressedSize = 0`. Eintragsname, Anzahl der Einträge und Kompressionsmethode sind deshalb nicht bekannt. Die Entscheidungen unten beschränken die Unterstützung auf das, was synthetische Tests belegen.

## Entscheidungen

1. **Bibliothek `zip` 7.3.0** (MIT, MSRV 1.83, unter der Workspace-MSRV 1.85), `default-features = false`, Feature `deflate-flate2`. Damit entstehen nur reine Rust-Abhängigkeiten `flate2` (mit `miniz_oxide`, MIT OR Zlib OR Apache-2.0), `crc32fast`, `adler2`, `simd-adler32` und `typed-path` (MIT bzw. MIT OR Apache-2.0). Nicht aktiviert: AES, bzip2, zstd, lzma, xz, ppmd, deflate64, zopfli (Komprimierung ist nicht nötig). **Nicht gewählt:** `zip` 9.1.0, weil es Rust 1.88 verlangt und die MSRV 1.85 bricht (gleiche Begründung wie für `sha2` in MIG-011). Kein Legacy-Code übernommen.
2. **Modell:** Das Manifest trägt pro gepacktem Eintrag die Archivgröße (`Manifest::with_archives`, Core, additiv). `FileEntry` bleibt unverändert, die Planung ist für gepackte und ungepackte Dateien identisch (Vergleich über SHA-1 und Größe der entpackten Datei). Das Archiv liegt unter `<Manifestpfad>` + `.zip` (`ARCHIVE_SUFFIX`).
3. **Ports (Core):** `LocalStore::spool(size)` liefert eine Schreib-/Lese-Temp-Datei am Ziel (Präfix `.armasync-`, nie gelistet, nach dem Schließen entfernt). `ArchiveReader::extract_single` entpackt genau einen Eintrag. Der Port ist in `armasync-adapters::archive::ZipUnpacker` implementiert, die Anwendung hängt nicht vom `zip`-Crate ab.
4. **Ablauf** (`armasync-application`): gepacktes Archiv vollständig in die Spool-Datei laden (Größe exakt `compressedSize`) → Staging am Ziel → begrenzt entpacken → Größe und SHA-1 des Ergebnisses prüfen → atomar committen. Jeder Fehler lässt vorhandene Dateien unverändert und entfernt Staging und Spool.
5. **Ablehnung vor dem ersten Datenbyte:** Der Entpacker prüft Metadaten des Eintrags, bevor Daten fließen:
   * genau **ein** Eintrag (kein Eintrag, mehrere, doppelte Namen → abgelehnt);
   * kein Verzeichnis, kein Link, keine Verschlüsselung;
   * Eintragsname als sicherer relativer Pfad (kein `..`, keine absoluten Pfade, kein `\`, leer). Der Name wird **nie** als Zielpfad benutzt; das Manifest bestimmt den Ort;
   * Methode nur **Stored** (Größen gleich) oder **Deflate**; alles andere → „nicht unterstützt“;
   * angekündigte Größe = Manifestgröße;
   * Deflate-Verhältnis: die angekündigte Größe darf `compressedSize × 1032 + 1024` nicht übersteigen (1032:1 ist die bekannte Obergrenze für DEFLATE).
6. **Größengrenze:** `MAX_PACKED_BYTES` = 2 GiB, für Archiv und entpackte Datei. Ein `compressedSize` darüber oder `0` (unbekannt) macht das Blatt nicht installierbar: es bleibt bei der bisherigen Behandlung (Warnung, `unsupported`, nie gelöscht, kein Download).
7. **Begrenzte Ausgabe:** Der Entpacker schreibt höchstens die angekündigte Größe. Ein Eintrag, der länger oder kürzer ist, scheitert. Die CRC des Eintrags wird am Ende gelesen (das Crate meldet Abweichungen als I/O-Fehler, abgebildet auf `Integrity`).
8. **Fehlerabbildung:** beschädigtes Archiv und Prüfsummen → `Integrity`; keine ZIP-Struktur oder abgelehnter Inhalt → `Format`; nicht unterstützt → `Unsupported`; Abbruch → `Cancelled`; Schreibfehler → `FileSystem`. Fehlermeldungen enthalten keine Namen oder Prüfsummen aus dem Archiv.

## Belegt durch Tests (Windows, lokal)

* Adapter (`tests/archive.rs`, 17 Fälle): gültig stored und deflate (über Transfer-Blockgrenzen), Name bestimmt das Ziel nicht; zwei Einträge, leeres Archiv, Verzeichnis, Link, unsichere Namen, falsche Größe, beschädigte Stored- und Deflate-Daten, abgeschnittenes Archiv, kein ZIP, Größenlimit vor dem Lesen, Abbruch vor und während des Entpackens, volles Ziel.
* Anwendung (`tests/archive_flow.rs`, 7 Fälle): Download, Entpacken, Prüfung, Commit; zweiter Lauf lädt nichts; beschädigtes Archiv und falsche Prüfsumme lassen die alte Datei; falsche Archivgröße (zu kurz und zu lang) → `Integrity`; zwei Einträge → nichts geschrieben; Abbruch vorher.
* Loopback-Ende-zu-Ende (`sync_e2e.rs`, 4 Fälle mit synthetischen Archiven und Metadaten): gültiges Archiv installiert, kein Rest im Ziel, zweiter Lauf ohne Download; beschädigtes Archiv (Exitcode 6) lässt die alte Datei; Traversal-Name wird abgelehnt und verlässt das Ziel nicht; 3-GiB-Ankündigung → kein Download, Warnung.
* Core: `with_archives` lehnt Archive ohne Eintrag ab.

## Nicht belegt / offen

* **Echtes Server-Format** (Eintragsname, Anzahl der Einträge, Methode, Verhalten bei echten Repositories): nicht geprüft. Die Annahme „genau ein Eintrag, Stored oder Deflate“ ist eine konservative Grenze, keine Beobachtung.
* **Freier Speicherplatz** wird vorab nicht geprüft (die Standardbibliothek bietet keine portable Abfrage); ein volles Laufwerk scheitert beim Schreiben.
* **Abbruch während eines gepackten Downloads** ist nur über die Anwendungs- und Adapter-Tests belegt, nicht als Ende-zu-Ende-Test mit Konsolensignal; der gemeinsame Abbruchpfad der HTTP-Übertragung ist für Dateien bereits getestet.
* **Linux/WSL:** nicht geprüft (Prüfregel: erst in der Abschlussvalidierung). RB-01 (Unix-Pfadrace) bleibt Release-Blocker.
* **Resume** eines Archivs (`Range`) gehört zu MIG-032 und ist nicht umgesetzt.
* **Lizenz:** `zip` und die Zusatzabhängigkeiten sind in `THIRD_PARTY_NOTICES.md` aufgenommen. Das ist keine Rechtsberatung; RQ-06 (vollständige Release-BOM) bleibt offen.

## Nicht gewählt

Entpacken mit Name als Zielpfad; Übernahme von Legacy-Code; Verschlüsselungs- und Komprimierungsmethoden ohne Tests; Entpacken im Speicher (Archive können groß sein); stille Installation bei unbekannter Größe.
