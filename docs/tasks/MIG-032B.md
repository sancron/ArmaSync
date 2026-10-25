# MIG-032B — Freiplatzprüfung und Ende-zu-Ende-Abbruch für große und komprimierte Downloads

- Status: **integriert** (2026-10-25; [Bericht](../changes/2026-10-25-MIG-032B.md), [Handoff](../handoffs/MIG-032B.md)). Eigentümer und Integrator: Claude. Basis-Branch `development`, Basis-SHA `603909e11b0f9b15f9eff0d9ff4f22a3b7ef8ac4`. Eigener Branch `claude/mig-032b-freiplatz` (Hauptcheckout).
- Teil von: MIG-032 (Fehlerfälle, Resume, Authentifizierung). Abgegrenzt: **Resume** ist eigener Umfang [MIG-032R](MIG-032R.md); **Authentifizierung** ist ausgesetzt (siehe unten).
- Voraussetzungen: MIG-030E integriert ([Bericht](../changes/2026-10-25-MIG-030E.md), [ADR-0008](../adr/0008-packed-files-zip.md)), MIG-032A (Abbruch-Pfad, Konsolensignal belegt), [LOCAL_FS_THREATS](../migration/LOCAL_FS_THREATS.md), [CLI_CONTRACT](../migration/CLI_CONTRACT.md).

## Ziel

1. **Begrenzte Freiplatzprüfung** vor jeder Datei: Ist auf dem Ziellaufwerk genug Platz für das Staging (Datei plus, bei gepackten Blättern, das Archiv im Spool), wird die Datei nicht begonnen und es entsteht ein klarer Fehler `FileSystem` mit Pfad, statt eines Fehlers erst beim Schreiben.
2. **Ende-zu-Ende-Abbruchtests** für große und komprimierte Downloads, ohne Konsolensignal: Abbruch über den gemeinsamen `CancelToken` während eines gepackten Archivs und einer großen Datei; vorhandene Dateien bleiben unverändert, kein `.zip`, keine Staging- oder Spool-Datei bleibt zurück, ein Folgelauf gelingt.

## Vertrag (Entwurf, vor Implementierung festzuschreiben)

* **Port:** `LocalStore::free_space(&self) -> Result<Option<u64>, Error>` mit Standardimplementierung `Ok(None)` (unbekannt). `None` bedeutet „nicht prüfbar“: der Lauf fährt fort und ein volles Laufwerk scheitert beim Schreiben wie bisher. Kein stiller Erfolg.
* **Bedarf je Datei:** plain `size`; gepackt `archive_size + size` (Spool und Staging gleichzeitig). Ein vorhandenes Ziel gilt nicht als frei, weil es erst beim Commit ersetzt wird (konservativ).
* **Reserve:** konstante Reserve (Vorschlag 16 MiB) als Heuristik, dokumentiert und getestet; keine Reserve ohne Begründung.
* **Ablehnung:** `FileSystem`-Fehler „nicht genug freier Platz für <relativer Pfad>“ vor dem ersten Byte dieser Datei; bereits fertige Dateien bleiben (wie beim Abbruch). Kein Ablehnen des gesamten Plans, solange kein Gesamtbedarf verlässlich bekannt ist.
* **Plattform:** `fs4::available_space` (bereits Abhängigkeit des Adapters, MIT OR Apache-2.0, keine neue Crate) liefert den für Nutzer verfügbaren Platz unter Windows (`GetDiskFreeSpaceEx`) und Unix (`statvfs`). **Belegt wird nur Windows.** Unix-Verhalten ist ungeprüft (RB-03), die Funktion darf unter Linux nichts behaupten; ein Fehler der Abfrage ergibt `None`.

## Abbruchtests (ohne Konsolensignal)

* Anwendung/Adapter: Abbruch in der Mitte eines gepackten Downloads (Testübertragung mit Pausen) und mitten im Entpacken (vorhanden: Adapter-Test); neu: Spool und Staging nach Abbruch leer.
* Loopback-GUI oder -CLI: ein absichtlich langsam ausgelieferter `.zip`- bzw. großer Dateidownload; Abbruch über den `CancelToken` des Screens (GUI-Testmuster `cancelling_a_running_transfer_keeps_existing_files_and_leaves_nothing_partial`). Dafür kann das Testkit `Published` um eine langsame Auslieferung ergänzen.
* Konsolen-Ctrl+C bleibt der bestehende manuelle Test (`--ignored`), keine neue Behauptung.

## Nicht im Scope

* Resume/`Range` (siehe [MIG-032R](MIG-032R.md)).
* **Authentifizierung ausgesetzt:** Zugangsdaten und Schlüsselbund (ADR-0005) sind nicht entschieden; bis dahin keine Anmeldung, keine Speicherung von Geheimnissen, keine Änderung an Logs oder Prozessargumenten.
* Linux/WSL-Läufe, GitHub-Actions, reale Serverarchive.
* Gesamtbedarf-Vorprüfung über alle Dateien (siehe oben, nicht verlässlich).

## Prüfungen (geplant, lokal Windows)

fmt, clippy `-D warnings`, `cargo test --workspace --locked`, `cargo +1.85.0 test --workspace --exclude armasync-gui --locked`, GUI-Tests. Tests: Freiplatz über `InMemoryStore`-Fake mit gesetztem Wert; echte Platte nur über den Windows-Pfad (ohne Fülltests, die Festplatten belasten).

## Offen / Risiken

* Die Heuristik kann bei sehr großen Dateien auf knappen Datenträgern zu früh ablehnen oder ein zu spätes Scheitern zulassen; es ist keine Garantie.
* Netzlaufwerke und Quoten: Verhalten nicht belegt.
