# Branches und Veröffentlichungsfluss

## Definition und Ausgangsstand

| Branch | Bedeutung | Startpunkt |
| --- | --- | --- |
| `legacy` | eingefrorener unveränderter Stand **dieses Repositories vor Migrationsbeginn** | `b7753cea18b2ac2cc57e29f926901997a4de8570` |
| `master` | Releasecode; keine aktive Featureentwicklung | zunächst gleicher Java-Bestand |
| `development` | aktive Integration, Migrationsplanung und spätere Rust-Entwicklung | Baseline plus Dokumentationspaket MIG-000 |
| `testing` | Staging/Releasekandidaten; Quelle für Master-Freigaben | zunächst gleiche Baseline |

`legacy` ist bewusst kein rekonstruierter früherer Upstream-Urstand. Es ist die eindeutig nachprüfbare Version, die bei Beginn dieses Auftrags vorlag. Kein Umbennen, Rebase, neuer Commit oder Force-Update auf diesem Branch.

```text
codex/<task> ──┐
              ├──> development ──> testing ──> master ──> Release-Tag
claude/<task> ─┘                         │
                       Fehler → Fix auf development → neuer RC

legacy = unveränderte Referenz, kein Ziel für Merges
```

## Entwicklung und Kandidaten

Features/Fixes aus `development` abzweigen und dorthin integrieren. Für RCs nach `testing` übernehmen, eine konkrete Kandidaten-SHA und Version fixieren und daraus bauen. Während Prüfung darf `development` weiterlaufen; spätere Commits sind damit nicht automatisch Teil desselben Kandidaten.

RC-Fehler regulär auf einem Task-Branch von `development` beheben und gezielt einen neuen Kandidaten erstellen. Bei abweichendem Developmentstand einen begrenzten RC-Fix aus `testing` nur mit Integratorentscheidung erlauben und **denselben Fix danach nach development rückübernehmen**. Neue RC-SHA braucht erneute betroffene Checks. Keine ungeprüften Änderungen nach bestandenem RC-Gate einschieben.

## testing → master

Freigegebene `testing`-SHA über nachvollziehbaren Merge/PR nach `master` übernehmen. Fast-forward ist möglich, wenn Plattformregeln es erlauben; ansonsten den entstandenen Master-Mergecommit prüfen. Build/Prüfungen müssen zum tatsächlich getaggten Commit gehören. Bei abweichendem Mergecommit dessen Tree gegen Kandidaten vergleichen und erforderliche Checks wiederholen. Keine geprüften RC-Artefakte fälschlich einem anderen Quellstand zuordnen.

Stabile Tags nur auf `master`, Prerelease-Tags auf der geprüften `testing`-SHA. Aktuelle Java-Version `2026.4.15`; erste Rust-Version und zukünftiges Versionsschema in P1/P6 entscheiden, keine ungeprüfte SemVer-/Kalendernummer erfinden. Releasequellen, SBOM, Signaturen und Release Notes an denselben Tag binden.

## Empfohlene Repositoryregeln

In GitHub für `master` und `testing`: PR erforderlich, bestandene benannte Statuschecks, Review, keine Force-Pushes/Branchlöschung; Admin-Bypass bewusst begrenzen. Für `legacy`: Updates/Deletion untersagen. `development`: Tests und Review für Integration, keine Force-Pushes. Default-Branch vorzugsweise `development`, dokumentierte Releasecodequelle bleibt `master`.

PR-Quellregel: nach `master` nur `testing`, nach `testing` `development` oder ausdrücklich benannter RC-Fix. Ein zukünftiger CI-Check soll Quell- und Zielbranch sowie Fork-Herkunft prüfen. Erst tatsächlich vorhandene Checknamen als Required Checks eintragen. Release-Publishing an geschützte Tags/Environment binden.

**Diese Markdownregeln aktivieren keinen serverseitigen Branchschutz.** Dieser Auftrag legt Branches an und dokumentiert die Regeln; GitHub-Rulesets, Default-Branch-Wechsel und Release-Freigabeumgebungen sind gesonderte Administrationsschritte in MIG-013. Ihren Zustand vor erster Veröffentlichung prüfen und Ergebnis dokumentieren.

## Überprüfen

```text
git rev-parse legacy
git rev-parse master
git log --oneline --decorate --all -8
git ls-remote --heads origin master development legacy testing
```

`legacy` muss dauerhaft die Baseline-SHA ergeben. Branches nur ohne Force pushen. Ein bereits anders belegter Remote-Branch wird nicht überschrieben; Unterschiede prüfen. Remoteexistenz ersetzt keine Branchschutzprüfung.
