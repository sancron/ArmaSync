# ADR-0003 — Daten zuerst, Quellnamen danach

Datum: 2026-10-18. Status: **angenommen für Import-/Sicherheitsgrenzen**, Repository-Schreibformat in P0/P4 noch zu entscheiden.

## Kontext

GZIP-komprimierte Java-Serialisierung enthält alte Klassennamen und UIDs. Ein reines Umbenennen verletzt Datenkompatibilität. Bestehende Repositoryserver liefern auch nach einmaligem Konfigurationsimport weiterhin Altmetadaten.

## Entscheidung

Lokale Altdateien über einen separaten kontrollierten Offline-Export und neutralen versionierten Rust-Import übernehmen. Direkte Servermetadaten über einen begrenzten Parser lesen, der keine Java-Klassen ausführt. Eigene Rust-Typen bleiben neutral benannt; dokumentierte Wire-Deskriptoren sind Daten, keine neue Klassenidentität. [COMPATIBILITY.md](../migration/COMPATIBILITY.md) ist der Arbeitsvertrag.

Nicht gewählt: globales Package-Replacement; unbeschränkte Deserialisierung; JVM als Dauerabhängigkeit; stiller Verlust alter Profile oder bestehender Serverkompatibilität.

## Stand nach MIG-021 (2026-10-18)

Der lokale Weg ist umgesetzt: Offline-Exporter (nur lesend, unverschlüsselte Dateien) → neutrales `armasync.import` → Rust-Import ([IMPORT_SCHEMA](../migration/IMPORT_SCHEMA.md)). **Nicht entschieden und nicht umgesetzt:** das Entschlüsseln der `.a3s.repository`-Dateien (Rust-Konstante aus dem Altquelltext oder einmaliger JVM-Export, siehe FORMAT_INVENTORY Abschnitt 7) – abhängig von der Herkunfts-/Rechtefrage RQ-01. Der Exporter listet diese Dateien nur als übersprungen.

## Stand nach MIG-022 (2026-10-19)

Der begrenzte Leser für Server-Metadaten ist für `.a3s/autoconfig` umgesetzt (`armasync-compat`, [SERVER_METADATA_IMPORT](../migration/SERVER_METADATA_IMPORT.md)): ohne JVM, ohne Klassenladen, mit Grenzen, Tabelle bekannter Wire-Deskriptoren und ohne Geheimnisse. Er liest nur Name und Adresse eines Repositorys. **Nicht entschieden und nicht gelöst:** das Entschlüsseln der lokalen `.a3s.repository`-Dateien (weiterhin RQ-01), Einstellungen und Addon-Listen. MIG-022 ist keine Lösung dafür. Lesen von `serverinfo`/`sync`/`changelogs`/`events` folgt mit dem Sync (ab MIG-030).

## Offene Schreibentscheidung

Vor P4-Repository-Publisher verbindlich wählen: (A) legacy-kompatible Ausgabe mit Gegentests in Java oder (B) neues Manifestformat mit versioniertem Servercutover, Export und ausdrücklich erklärten Clientgrenzen. Lesekompatibilität allein beweist keine Schreibkompatibilität.

Strikt null alte Wirezeichenfolgen im neuen Parser und gleichzeitig vollständige direkte Altformatunterstützung sind keine gleichbedeutenden Anforderungen. Wenn auch Formatdeskriptoren entfallen sollen, den Servercutover ausdrücklich entscheiden; keine stillschweigende Löschung dieser Fähigkeit.
