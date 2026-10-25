# Release-Blocker

Stand: 2026-10-25 (MIG-032R, MIG-032R2 und MIG-040A ändern keine Blocker; RB-01 bis RB-06 bleiben offen). Liste dessen, was ein Release (Windows oder Linux) **verhindert**, bis es behoben und nachgewiesen ist. Sie ergänzt [STATUS](STATUS.md) und die Rechtefragen ([RIGHTS_QUESTIONS](../tasks/RIGHTS_QUESTIONS.md)); Einträge werden nur mit Nachweis (Commit, Prüfprotokoll) geschlossen.

| ID | Blocker | Betrifft | Schließen durch |
| --- | --- | --- | --- |
| RB-01 | **Unix-Pfadrace im Dateisystemadapter** | Linux-Synchronisation und jede Aussage „race-freie Pfadgrenze unter Linux“ | siehe unten |
| RB-02 | **P1-GUI-/Accessibility-Gate**: NVDA, Orca und Sichtprüfung des hellen Themes nicht ausgeführt | jede Veröffentlichung einer GUI | Protokoll [GUI_SCREENREADER_PROTOCOL](GUI_SCREENREADER_PROTOCOL.md) ausführen und ausfüllen; danach ADR-0002 auf „angenommen“ |
| RB-03 | **Linux-/WSL-Abschlussvalidierung** (alle Plattformprüfungen sind bis zum Abschluss zurückgestellt) | Linux-Release, Plattformaussagen | vollständige Testläufe, Ende-zu-Ende und GUI-Start unter Linux (siehe unten) |
| RB-04 | **Rechtefragen RQ-01 bis RQ-08 offen** (RQ-06 teilweise) | Veröffentlichung von Code und Paketen, Portierung von Bestandslogik, Release-BOM | Entscheidung des Projektverantwortlichen, dokumentiert im Tracker |
| RB-05 | **Keine Prüfung gegen echte Altserver** (nur Loopback-Testserver) | Aussage „funktioniert mit bestehenden Repositories“ | kontrollierter Test gegen ein echtes Repository mit Einverständnis des Betreibers |
| RB-06 | GitHub-CI pausiert (PROC-002) | automatische Prüfung vor Releases | Wiederherstellung der Trigger laut [CI](CI.md), erster Lauf dokumentiert |

## RB-01 — Unix-Pfadrace (konkret)

**Angriffsweg:** Ein lokaler Prozess mit Schreibrecht im Zielverzeichnis (oder dessen Eltern) tauscht während eines Laufs einen bereits geprüften Ordner gegen einen symbolischen Link nach außen. Unter Windows ist das durch Ordner-Handles ohne Lösch-Freigabe ausgeschlossen (MIG-030H); unter Unix bietet die Standardbibliothek kein verzeichnisbezogenes, race-freies Öffnen, und ein geöffnetes Verzeichnis lässt sich umbenennen. Die vorhandene Prüfung (kanonischer Pfad vor/nach dem Anlegen der Temporärdatei und vor `rename`) verkleinert das Zeitfenster, schließt es nicht. Details: [LOCAL_FS_THREATS](LOCAL_FS_THREATS.md).

**Schutzgrenze heute:** Schutz vor Links, die *vor* dem Lauf existieren, und vor versehentlichem Verlassen der Wurzel; **kein** Schutz gegen einen gleichzeitig handelnden Prozess mit Schreibrecht im Ziel. Deshalb ist die Synchronisation unter Linux **nicht freigegeben**: die GUI ändert dort nichts (nur Plan), und die CLI verweigert `sync run` mit Exitcode 8 und dem Grund. **Es gibt in einem normalen Build – insbesondere im Release-Build – keine Option, das zu umgehen:** `--allow-unreleased-platform` existiert nur in Validierungsbuilds mit dem Cargo-Feature `unreleased-platform-override` (nicht Standard; kein Release-Bestandteil; PROC-003). Belegt durch Tests (`crates/armasync-cli/tests/platform_gate.rs`) und eine Prüfung der Release-Binärdatei (Option unbekannt, Exitcode 2, Text nicht in der Binärdatei).

**Behebung (Plan):**
1. Kapazitätsbasierter Zugriff auf das Ziel unter Unix: jede Komponente relativ zu einem geöffneten Verzeichnis-Handle mit `openat`/`O_NOFOLLOW`/`O_DIRECTORY` (bzw. `openat2` mit `RESOLVE_BENEATH`/`RESOLVE_NO_SYMLINKS`), Staging-Datei und Austausch mit `renameat` relativ zum verifizierten Elternhandle, Löschen mit `unlinkat`. Kandidaten: `rustix` (MSRV 1.65, Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT) oder `cap-std`; Lizenz-/MSRV-/Abhängigkeitsprüfung vor der Aufnahme (RQ-06), kein `unsafe` im eigenen Code (Workspace verbietet es).
2. Neue Aufgabe mit eigener Taskdatei; Tests mit konkurrierendem Angreifer-Thread (Symlink-Tausch) analog zum Windows-Test, inklusive Kontrollnachweis, dass der Tausch ohne Schutz gelingt.
3. Prüfung unter Linux in der Abschlussvalidierung (RB-03); erst danach darf die Plattformaussage von `NotReleased` auf `Verified` wechseln (`armasync_workflow::platform_status_for`, mit Test).

## RB-03 — Linux-/WSL-Abschlussvalidierung (konkret)

Auszuführen erst zum Programmabschluss (Prüfregel in [WORKFLOW](../ai/WORKFLOW.md)): `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace --locked` (inklusive der nur unter Unix kompilierten Tests: symbolische Links, Nicht-UTF-8-Namen, `flock`-Verhalten), MSRV 1.85 ohne GUI-Crate, `cli-gui-free.sh`, die CLI-Ende-zu-Ende-Tests als Validierungsbuild (`cargo test -p armasync-cli --features unreleased-platform-override`; ohne das Feature sind sie unter Linux abgeschaltet und nur `platform_gate` läuft, das die Sperre prüft), GUI-Start unter einem Linux-Desktop (AT-SPI/Orca: RB-02), der Angreifertest für RB-01; Ergebnis je Befehl dokumentieren. Bis dahin steht jede Linux-Aussage auf „nicht geprüft“.
