# ADR-0003 — Daten zuerst, Quellnamen danach

Datum: 2026-10-18. Status: **angenommen für Import-/Sicherheitsgrenzen**, Repository-Schreibformat in P0/P4 noch zu entscheiden.

## Kontext

GZIP-komprimierte Java-Serialisierung enthält alte Klassennamen und UIDs. Ein reines Umbenennen verletzt Datenkompatibilität. Bestehende Repositoryserver liefern auch nach einmaligem Konfigurationsimport weiterhin Altmetadaten.

## Entscheidung

Lokale Altdateien über einen separaten kontrollierten Offline-Export und neutralen versionierten Rust-Import übernehmen. Direkte Servermetadaten über einen begrenzten Parser lesen, der keine Java-Klassen ausführt. Eigene Rust-Typen bleiben neutral benannt; dokumentierte Wire-Deskriptoren sind Daten, keine neue Klassenidentität. [COMPATIBILITY.md](../migration/COMPATIBILITY.md) ist der Arbeitsvertrag.

Nicht gewählt: globales Package-Replacement; unbeschränkte Deserialisierung; JVM als Dauerabhängigkeit; stiller Verlust alter Profile oder bestehender Serverkompatibilität.

## Offene Schreibentscheidung

Vor P4-Repository-Publisher verbindlich wählen: (A) legacy-kompatible Ausgabe mit Gegentests in Java oder (B) neues Manifestformat mit versioniertem Servercutover, Export und ausdrücklich erklärten Clientgrenzen. Lesekompatibilität allein beweist keine Schreibkompatibilität.

Strikt null alte Wirezeichenfolgen im neuen Parser und gleichzeitig vollständige direkte Altformatunterstützung sind keine gleichbedeutenden Anforderungen. Wenn auch Formatdeskriptoren entfallen sollen, den Servercutover ausdrücklich entscheiden; keine stillschweigende Löschung dieser Fähigkeit.
