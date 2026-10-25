# ADR-0007 — Streaming-Übertragung, Staging und Schutzpfade (Ports für MIG-030B/C)

Datum: 2026-10-19. Status: **angenommen**; umgesetzt und getestet in MIG-030P (Core-Typen, Use-Case, In-Memory-Adapter). Bezug: [ADR-0004](0004-core-contracts.md), [ADR-0006](0006-sync-metadata-and-http-transport.md), [SYNC_METADATA](../migration/SYNC_METADATA.md). Ergänzt durch [ADR-0009](0009-resume-range.md): Resume mit `Range`; ein Netzabbruch behält die Teildaten (Abschnitt 5 dort).

## Kontext

`RepositorySource::read` lieferte ganze Dateien als `Vec<u8>`, `LocalStore::write` schrieb ganze Dateien. Mod-Dateien sind groß; außerdem veröffentlichen bestehende Server SHA-1, der Scan muss also mit der Hashart des Manifests rechnen, und Löschschritte dürfen komprimiert geführte oder versteckte Einträge nicht gefährden.

## Entscheidung (Ports und Pflichten)

| Port | Vertrag |
| --- | --- |
| `RepositorySource::fetch(path, expected_size, out, cancel, progress)` | Streamt in Stücken ≤ `TRANSFER_CHUNK` (64 KiB) in `out`; `cancel` vor jedem Stück und vor dem Verbinden; `progress(n)` je Stück; schreibt **nie** mehr als `expected_size` (sonst `Integrity` nach genau `expected_size` Bytes), kürzer endende Quelle = `Integrity`; `Cancelled`, `Network` (auch Auth/TLS), `FileSystem` (wenn `out` scheitert); bei jedem Fehler ist das Geschriebene ein unvollständiger Präfix, den der Aufrufer verwirft; keine stillen Wiederholungen oder Resume (MIG-032). Standardimplementierung über `read` + `copy_in_chunks`; Netzwerkadapter überschreiben sie. |
| `LocalStore::begin(path, expected_size) -> StagedFile` | Staging-Datei im selben Dateisystem wie das Ziel, unsichtbar für `list`; das Ziel ändert sich **nicht** vor `commit`; `Drop` ohne `commit` entfernt die Staging-Datei, Reste nach Absturz entfernt der Adapter beim nächsten `begin`/`list`; Elternverzeichnisse nur innerhalb der Wurzel; Pfade, die über Links/Junctions die Wurzel verlassen, werden abgelehnt. |
| `StagedFile::commit` | Flush auf stabilen Speicher, dann **atomarer** Austausch: danach altes oder vollständiges neues Ziel, nie ein Gemisch; scheitert der Austausch (z. B. Sperre), bleibt das alte Ziel unberührt und die Staging-Datei wird entfernt. |
| `LocalStore::list(algorithm, cancel)` | Hashes mit der Art des Manifests (sonst gälte jede Datei als geändert); abbrechbar; Staging-Reste nie aufgeführt. |
| `Hasher::digest` / `Digest` | Inkrementelle Hashes für Streaming; `hash` ist daraus abgeleitet. |
| `PlanOptions::protected_paths` | Dateien/Ordner, in denen **nie** gelöscht wird, auch nicht innerhalb von `managed_roots` (Vorrang); Warnung `ExtraKeptProtected`. `SyncMetadata::plan_options` füllt sie aus `protected` (versteckte Ordner) und `unsupported` (komprimierte Blätter, deren entpackte lokale Kopie nicht im Manifest steht). |

**Use-Case (`apply`):** `begin` → `fetch` in einen Tee (Staging-Datei + Digest, lehnt mehr als die angekündigte Größe ab) → Größe und Hash prüfen → `commit`. Jeder Fehler (Netz, Abbruch, Größe, Hash, volle Platte, fehlgeschlagener Austausch) lässt ein vorhandenes Ziel unberührt; bereits abgeschlossene Dateien bleiben. Fortschrittsereignisse höchstens alle 256 KiB und am Dateiende; Bytes einer abgebrochenen oder fehlgeschlagenen Datei zählen nicht als erledigt. Der Plan wird vor `apply` neu berechnet (ADR-0004).

## SHA-1-Prüfung (Auftrag 2)

SHA-1 kommt ausschließlich vor als: `HashAlgorithm::Sha1` (Core, Dokumentation „nur Integrität des Altprotokolls“), Digest-Implementierung im Adapter (`memory.rs`), Konvertierung der veröffentlichten Prüfsumme in `sync_meta`. Er wird **nicht** für Signaturen, Updates, Zertifikats- oder Vertrauensentscheidungen, Schlüssel oder Identität verwendet (Suche im Quelltext: keine weitere Verwendung). Vertrauen stützt sich auf TLS-Prüfung (ADR-0006); bei `http`/`ftp` bleibt die Warnung. Der Hasher-Port trägt den Hinweis; neue Verwendungen von `Sha1` sind im Review zu hinterfragen.

## Folgen für die Adapter

* **MIG-030B (HTTP, `ureq`):** implementiert `fetch` als echtes Streaming (kein Puffern der Datei), Header `Accept-Encoding: identity`, Größenprüfung gegen `expected_size`, Abbruch/Zeitgrenzen, keine Weiterleitungen ohne Grenze, Fehlerzuordnung; TLS immer geprüft.
* **MIG-030C (Dateisystem):** implementiert `list` (SHA-1/SHA-256-Scan, abbrechbar), `begin`/`commit` (Staging im Zielbaum, atomarer Austausch, Windows-Besonderheiten), `delete`, Containment.
* Beide Adapter müssen die hier festgelegten Verträge gegen die In-Memory-Referenz und eigene Fehlerfälle testen.

## Nicht entschieden

Resume/Teildateien (MIG-032), Parallelität, Größen-/Zeitverkürzung des Scans (nur mit Messung), Zugangsdaten für HTTP, ZIP-Blätter (MIG-030E), `list`-Hinweise zur Größe als Vorfilter.
