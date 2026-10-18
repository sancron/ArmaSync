# Rechte- und Asset-Inventur (MIG-002)

Stand: 2026-10-18. Geprüfte Basis: `b7753cea18b2ac2cc57e29f926901997a4de8570` (Bestandsinventar) und `adc986d` (Taskbasis). Diese Inventur dokumentiert sichtbare Lizenz- und Herkunftsbelege; sie ist keine Rechtsberatung und entscheidet keine Rechtefrage verbindlich. Ein fehlender Beleg wird ausdrücklich als offen geführt.

## Ergebnis für das Rust-Ziel

Die vorhandene `LICENCE.md` enthält den GNU GPLv3-Text. Sie belegt für sich allein nicht den Rechteinhaber oder den Geltungsbereich jeder Quelldatei. `LICENSES/AGPL-3.0-only.txt` ist laut `LICENSES/README.md` der unveränderte AGPLv3-Text und ist als Zielreferenz für neue, unabhängig erstellte Rust-Komponenten vorgesehen. Daraus folgt keine pauschale Umlizenzierung vorhandener Dateien. Rust-Ports von vorhandenem Code, insbesondere Jazsync/MD4, sowie jede Übernahme von Grafiken, Fonts, JAR-Code oder Installerteilen benötigen eine dokumentierte Rechte- und Lizenzentscheidung vor der Aufnahme in den AGPL-Zielbaum.

## Bestandsabdeckung

Das Baseline-Inventar `docs/migration/INVENTORY.md` zählt 515 getrackte Dateien. Die nachstehende Einteilung deckt alle dort ausgewiesenen Pfadgruppen ab; reine Projekttexte, Tests, Build-/Releasekonfigurationen und Quellcode ohne separat erkennbares Fremdmaterial sind keine Drittanbieter-Assets. Diese Zuordnung belegt keine Inhaberschaft. Originaldateien sollen für die Archivierung erhalten bleiben; eine Wiederverwendung in der neuen Anwendung ist separat zu prüfen.

| Baseline-Gruppe | Beleg / Befund | AGPL-Rust-Behandlung |
| --- | --- | --- |
| Eigene Java-/Go-Quellen, Tests, Ressourcen und Konfiguration | `LICENCE.md`, Dateiheader, `docs/migration/INVENTORY.md`; individuelle Header und Herkunftsbelege sind nicht durchgängig vorhanden | Neu geschriebene, unabhängige Rust-Dateien unter AGPL-3.0-only; Portierte Logik nicht automatisch übernehmen. Rechteinhaber und Herkunft betroffener Bestandsdateien vor Portierung klären. |
| `src/main/java/fr/soe/a3s/jazsync/**` | Die Java-Header nennen Casey Marshall, teils Free Software Foundation, und Tomas Hlavnicka und GNU GPL „version 2 or (at your option) any later version“. Einzelbelege z. B. `MD4.java`, `BrokenMD4.java`, `Generator.java`, `Util.java`, `RollingChecksum.java`, `MetaFileReader.java` | Eigenständige Neuimplementierung nach Formatbeschreibung oder mit gesonderter Rechtefreigabe. Nicht kopieren/übersetzen, bevor AGPL-Kompatibilität und abgeleiteter Werkstatus fachkundig geklärt sind. Historische Lizenz- und Copyright-Hinweise erhalten. |
| HTTP-`DataRange.java` | Header sagt „This file is a part of Jazsync“ und verweist auf GPLv2-or-later | Als möglicherweise Jazsync-abgeleitet behandeln; neu entwerfen oder Rights Review vor Portierung. |
| Direkte Gradle-Artefakte | `build.gradle`; `gradle dependencies --configuration runtimeClasspath` wurde lokal erfolgreich ausgewertet (MIG-002-Worktree, 2026-10-18) | Version und Lizenz des tatsächlich gewählten Crates neu inventarisieren. Java-Bibliotheken nicht in Rust übernehmen. Beigefügte Dritt-Lizenztexte/Notices gemäß jeweiliger Lizenz beibehalten, falls Binärbestandteile ausgeliefert würden. |
| Lokale, direkt eingebundene JARs | `build.gradle:44-46`: `jshortcut-0.4-oberzalek.jar`, `JTattoo-1.6.10.jar`, `junique-1.0.4.jar`; SHA-256 und JAR-Metadaten in der Tabelle unten | Gehören nicht in den Rust-Zielbuild. `JTattoo` und `junique` liefern im JAR keine auffindbaren LICENSE/NOTICE-Dateien; Lizenz/Provenienz offen. `jshortcut` enthält `LICENSE`, deren Nutzungsbedingungen vor etwaiger Weitergabe zu prüfen sind. |
| Weitere JARs unter `resources/lib/` | Inventar enthält historische/gebündelte Versionen und Laufzeitkopien; jede Datei hat SHA-256 im lokalen Analyseprotokoll, Versionsmetadaten und eingebettete Lizenztexte variieren | Nicht pauschal als aktuell verwendet betrachten. Für Release-BOM aufgelösten Gradle-Graphen verwenden; übrige Kopien als Altbestand markieren und vor neuer Paketierung einzeln bewerten. |
| Geist-Schriftdateien | `src/main/resources/fr/soe/a3s/ui/fonts/OFL.txt` nennt Geist-Autoren und SIL Open Font License 1.1; `build.gradle` zeigt, dass die Geist-Familie verwendet wird | Font darf nach OFL-Bedingungen gebündelt werden; OFL-Hinweis mit Font-Dateien ausliefern und Fontnamen/Reserved Font Names beachten. Nicht als AGPL-Code deklarieren. |
| Inter-Schriftdateien | Gleiches `OFL.txt` nennt Inter Project Authors und SIL OFL 1.1. `build.gradle:77-80` hält Inter als Designmaterial vor und schließt `Inter-*.ttf` aus dem Release-JAR aus | Nur als separaten Fontbestand mit OFL-Beleg behandeln; nicht in Pakete aufnehmen, solange ungenutzt. |
| SVG-Icons unter `src/main/resources/resources/icons/**` | SVG-Dateien enthalten keinen systematisch einheitlichen Urheber-/Lizenzkopf; `app.svg` wird als kanonisches Icon in `installer/nsis/README.md` bezeichnet. Einzelne Icons ähneln bekannten Icon-Sets, daraus wird keine Herkunft abgeleitet. | Provenienz jeder übernommenen Grafik klären oder eigenständige Lynxnode.it-Assets beauftragen/erstellen. Keine alleinige AGPL-Kennzeichnung ohne Urheberbeleg. |
| Gerenderte App- und Installerbilder `app-rendered.png`, `installer/assets/Arma3Sync.{png,ico}` | Build-/Installerdateien verwenden die Bilder (`build.gradle`, `installer/nsis/README.md`, Generator-Skript). Im Bestand wurde kein belastbarer Ursprungs-/Autorenbeleg für die Binärgrafiken gefunden. | Vor Wiederverwendung Herkunft und Rechte klären; andernfalls durch neu erstellte Lynxnode.it-Grafiken ersetzen. Hashes aus Git-Blobinventar identifizieren, lizenzieren sie nicht. |
| Sound-Dateien | In 515 Baseline-Pfaden fand sich keine separat ausgewiesene Sounddateigruppe | Derzeit nicht relevant; bei neuen Sounds Lizenz, Quelle und Attribution vor Aufnahme dokumentieren. |
| NSIS-Installer, Bootstrapper, Packaging, Winget- und Release-Metadaten | `installer/**`, `packaging/**`, `release/**`; enthält Produktnamen, Icon-Verweise, Installer-/Bootstrapper-Quellen und Paketmetadaten. Es gibt keinen Einzelbeleg, der sämtliche dieser Dateien unter eine gesicherte Rechtekette stellt. | Nicht automatisch in Rust-App übernehmen. Neu erstellte Skripte unter passender Lizenz; fremde Paketdefinitionen nach deren Upstream-Lizenz/Einreichungsbedingungen behandeln. Icon- und Publisherrechte separat klären. |
| Historische Dokumentation, Changelogs, Presets und Testmaterial | `docs/**`, `changelogs/**`, `fixtures/**`; umfasst eigene Dokumente sowie Beispiel-/synthetische Legacy-Formate | Eigene Dokumentation kann nach nachgewiesener Rechteinhaberschaft unter Projektlizenz stehen. Drittanbietertexte/-daten nicht ungeprüft übernehmen. MIG-001-Fixtures sind synthetisch; ihr Formatwert begründet keine Rechte an fremden Daten. |
| GPLv3-Lizenztext | `LICENCE.md` | Historischer Lizenztext bleibt im Legacy-Kontext. Nicht als alleiniger Lizenznachweis für neue Rust-Artefakte verwenden. |
| AGPLv3-Lizenztext | `LICENSES/AGPL-3.0-only.txt`, Prüfsumme in `LICENSES/README.md` | Ziel-Lizenztext; Geltung für neue, unabhängige Rust-Komponenten gemäß Projektentscheidung. |

## Gradle-Laufzeitgraph

Direkte Laufzeitabhängigkeiten aus `build.gradle` und aufgelöste Transitiven aus Gradle. Der Quellcode des Reports bewertet keine Lizenzkompatibilität verbindlich; Lizenzdateien der jeweiligen Artefaktversion bzw. deren veröffentlichte Metadaten sind bei der konkreten Release-BOM zu archivieren.

| Koordinate | Version | Quelle | Rechte-/Portierungshinweis |
| --- | --- | --- | --- |
| commons-io:commons-io | 2.15.1 | direkt | Apache Commons; neue Rust-Implementierung wählen und deren Lizenz separat erfassen |
| commons-logging:commons-logging | 1.2 | direkt/transitiv | Apache Commons; im Rust-Ziel ggf. nicht erforderlich |
| commons-net:commons-net | 3.10.0 | direkt | Apache Commons; FTP/FTPS-Verhalten als Spezifikation portieren, keinen Code kopieren |
| org.apache.sshd:sshd-sftp / sshd-core / sshd-common | 2.15.0 | direkt/transitiv | Apache MINA SSHD; neue SSH/SFTP-Bibliothek unabhängig prüfen |
| org.apache.httpcomponents:httpclient / httpcore | 4.5.14 / 4.4.16 | direkt/transitiv | Apache HttpComponents; ersetzt durch neu geprüfte Rust-Abhängigkeit |
| commons-codec:commons-codec | 1.16.0 | direkt/transitiv | Apache Commons; Gradle wählt 1.16.0 statt HttpClient-Transitiv 1.11 |
| com.formdev:flatlaf / flatlaf-extras | 3.6.1 | direkt | UI-Abhängigkeiten; kein Rust-Zielcode |
| com.github.weisj:jsvg | 2.0.0 | direkt/transitiv | SVG-Renderer; kein Rust-Zielcode |
| com.github.lookfirst:sardine | 5.12 | direkt | WebDAV-Client; kein Rust-Zielcode |
| jakarta.xml.bind:jakarta.xml.bind-api | 4.0.0 | transitiv | Sardine/JAXB-Laufzeit |
| jakarta.activation:jakarta.activation-api | 2.1.1 | transitiv | JAXB/Angus-Laufzeit |
| org.glassfish.jaxb:jaxb-runtime / jaxb-core / txw2 | 4.0.2 | transitiv | JAXB-Laufzeit; im Rust-Ziel nicht übernehmen |
| org.eclipse.angus:angus-activation | 2.0.0 | transitiv | Jakarta Activation-Implementierung |
| com.sun.istack:istack-commons-runtime | 4.1.1 | transitiv | JAXB-Hilfskomponente |
| org.slf4j:slf4j-api / slf4j-simple | 2.0.7 | transitiv | Logging-Brücke/Implementierung |

## Lokal gebündelte JARs

SHA-256 wurde am MIG-002-Worktree unter Windows mit Python `hashlib` über die getrackten `resources/lib/*.jar` berechnet. Die Hashes identifizieren die konkreten Dateien, nicht die Rechte daran. „Build direkt“ bedeutet Eintrag in `build.gradle`; die übrigen sind im Bestand, aber nicht im aktuellen Gradle-Runtime-Graph.

| Datei | SHA-256 | Verwendung / eingebetteter Rechtebeleg |
| --- | --- | --- |
| `jshortcut-0.4-oberzalek.jar` | `d29a4c7bfc19d0a277c8c6ce3d648a1406db8c0a90f09bd0878611168d683418` | Build direkt; enthält `LICENSE`, genaue Lizenzbedingungen und Herkunft für Weitergabe prüfen |
| `JTattoo-1.6.10.jar` | `4e2e5d8cd972478d5c2a5ad006407f9ba29ec4c5da234f597f4ea348c784223b` | Build direkt; keine LICENSE/NOTICE-Datei im JAR gefunden; Herkunft/Recht offen |
| `junique-1.0.4.jar` | `aa259f12cd4d6aeb22e0d35b55f6274c634f742008a45264133aa2db767ed678` | Build direkt; keine LICENSE/NOTICE-Datei im JAR gefunden; Herkunft/Recht offen |
| `commons-io-2.15.0.jar`, `commons-io-2.0.1.jar` | `a328dad730921d197b6a9b195dffa00e41c974c2dac8fe37e84d31706bca7792`, `2a3f5a206480863aae9dff03f53c930c3add6912f8785498d59442c7ebb98c5c` | Nicht aktuelle direkte Version (Build nutzt 2.15.1); enthalten Apache LICENSE/NOTICE |
| `commons-codec-1.16.0.jar`, `commons-codec-1.13.jar` | `56595fb20b0b85bc91d0d503dad50bb7f1b9afc0eed5dffa6c6bb25929000484d`, `61f7a3079e92b9fdd605238d0295af5fd11ac411a0a0af48deace1f6c5ffa072` | 1.16.0 ist aktuell; beide enthalten Apache LICENSE/NOTICE |
| `commons-net-3.10.0.jar`, `commons-net-3.4.jar` | `2230eec44ef4b8112ea09cbeb6de826977abe792e627cee2770e35ca8c39dce1`, `b2fe39162714d059c6b6f2623298a34c6a8a5608ee8abb3077132cbd94a4c01` | 3.10.0 aktuell; beide enthalten Apache LICENSE/NOTICE |
| `commons-logging-1.2.jar` | `daddea1ea0be0f56978ab3006b8ac92834afeefbd9b7e4e6316fca57df0fa636` | Apache LICENSE/NOTICE; Gradle-Artefakt ebenfalls verwendet |
| `httpclient-4.5.14.jar`, `httpclient-4.5.3.jar` | `c8bc7e1c51a6d4ce72f40d2ebbabf1c4b68bfe76e732104b04381b493478e9d6`, `db3d1b6c2d6a5e5ad47577ad61854e2f0e0936199b8e05eb541ed52349263135` | 4.5.14 aktuell; beide enthalten Apache LICENSE/NOTICE |
| `httpcore-4.4.16.jar`, `httpcore-4.4.6.jar` | `6c9b3dd142a09dc468e23ad39aad6f75a0f2b85125104469f026e52a474e464f`, `d7f853dee87680b07293d30855b39b9eb56c1297bd16ff1cd6f19ddb8fa745fb` | 4.4.16 aktuell; beide enthalten Apache LICENSE/NOTICE |
| `flatlaf-3.6.1.jar`, `flatlaf-3.1.1.jar`, `flatlaf-extras-3.6.1.jar` | `687be33e1e3eface1681fd194732d16e008126a47b6116b4301e7070c918c4c9`, `264ef9528627510ca20f3c680c4e3ac0129bcac235b8d4d9f014261f3cd6d56f`, `76686734e6f9fce97f639226e5fc25e5e543af72266ef787a8e1e4c42247f3c6` | Bundled LICENSE; current Gradle version 3.6.1 |
| `jsvg-2.0.0.jar` | `81fc9ada579fc3f4924ec35f904c043de13ee8fba280322e638c7092006f1b52` | Bundled LICENSE; transitiv und direkt |
| `sardine-5.12.jar`, `sardine-5.8.jar` | `368a8866bd10328c542a8a1a4ff183594f538105ca83551979d84f0f330eec07`, `ea4e8b253b8362b3f9a041fded9d6957066c5f0eb74d7c259d713e59665980bb` | 5.12 aktuell; JAR enthält keine benannte Lizenzdatei; Lizenznachweis aus Projektmetadaten für Release archivieren |
| `angus-activation-2.0.0.jar`, `jakarta.activation-api-2.1.1.jar`, `jakarta.xml.bind-api-4.0.0.jar`, `jaxb-core-4.0.2.jar`, `jaxb-runtime-4.0.2.jar`, `txw2-4.0.2.jar` | `3a12d321…079147`, `33bae3f0…d0491a04`, `57e3796a…a5515aa122`, `d7ff2954…c470ccd`, `1bc271e6…fe0e9a939`, `ea71912e…29a0cf0c` | Laufzeitartefakte enthalten LICENSE/NOTICE-Dateien; im aktuellen Gradle-Graph verwendet. Hashes hier zur Lesbarkeit gekürzt; vollständige SHA-256 siehe MIG-002-Bericht. |
| `jaxb-api-2.3.0.jar`, `istack-commons-runtime-4.1.1.jar` | `88300798…5101bb3d03`, `7e8148c5…3918dcd3` | Historische Bestandskopien; LICENSE teils eingebettet, Ist-Verwendung nicht im aktuellen Runtime-Graph |
| `slf4j-api-2.0.7.jar`, `slf4j-simple-2.0.7.jar` | `5d6298b9…85d946f4`, `50eae3f1…f3bc01a7` | Transitive aktuelle Laufzeit; LICENSE im JAR |

Die Tabelle erfasst alle 30 JAR-Pfade aus `resources/lib/`. Für kompakte Tabellenzeilen sind zehn Hashes auf Präfix und Suffix gekürzt; vollständige Werte stehen im MIG-002-Bericht. Für eine signierte Release-BOM sind SHA-256 und Lizenztexte aller tatsächlich gepackten Artefakte erneut maschinell zu erfassen.

## Offene Rechtefragen und Zuständigkeit

| ID | Offene Frage / Beleglücke | Nächster Schritt | Empfohlene Zuständigkeit |
| --- | --- | --- | --- |
| RQ-01 | Wer hält die Rechte an den eigenen Java-/Go-Dateien und welche `LICENCE.md`-Fassung gilt je Teilbaum? | Copyright-/Contributor-Historie und Dateiheader gegen Projektinhaber prüfen; Rechtekette dokumentieren | Projektverantwortlicher / Integrator |
| RQ-02 | Jazsync-/MD4-Header nennen GPLv2-or-later und mehrere Autoren. Dürfen Implementierung oder abgeleitete Rust-Portierung unter AGPL-3.0-only verteilt werden? | Keine Codeübernahme vor einer gesonderten Lizenz-/Derivatprüfung oder schriftlichen Freigabe; parallel unabhängige Neuimplementierung spezifizieren | Projektverantwortlicher mit Rechtsberatung |
| RQ-03 | JTattoo und junique haben im JAR keinen LICENSE-/NOTICE-Beleg; jshortcut verlangt Prüfung des eingebetteten LICENSE-Inhalts | Upstream-Projekt/Downloadherkunft, konkrete Lizenzversion und Weitergaberechte nachweisen oder im Rust-Ziel entfernen | Integrator / Paketierer |
| RQ-04 | Herkunft/Autoren der alten App-/Installericons und diverser SVG-Icons ist nicht ausreichend dokumentiert | Originaldesignquelle, Urheber und Lizenz nachweisen; sonst neue Lynxnode.it-Grafiken verwenden | Branding-Verantwortlicher |
| RQ-05 | Sardine-JAR ohne Lizenzdatei; Upstream-Lizenz für Version 5.12 ist nicht im Repo-Belegmaterial abgelegt | Versionierte Upstream-Lizenz samt URL/Hash in Release-BOM archivieren | Integrator |
| RQ-06 | Die aktiven Rust-Crates sind noch nicht ausgewählt | Vor Aufnahme in Cargo-Supply-Chain Versions-/Lizenzinventar und Notices erzeugen; GPL-/OFL-/Apache-Kombination im Release prüfen | Rust-Architekt / Integrator |
| RQ-07 | Historische Winget-Metadaten/Paketidentität und NSIS-/Bootstrapper-Copyrights sind für eine neue Publisher-Identität ungeklärt | Upstream-Verzeichnis-/Einreichungsregeln prüfen; neue Paketkennung und Publisherrechte vor Veröffentlichung bestätigen | Release-Verantwortlicher |
| RQ-08 | Die Bestandslizenzdatei heißt `LICENCE.md` und ist ein GPLv3-Volltext; das sagt nicht eindeutig, ob Bestandsquellen GPL-3.0-only oder -or-later sind | SPDX-/Lizenzhinweise je migriertem Modul festlegen, bevor Header/Dateien geändert werden | Projektverantwortlicher / Rechtsberatung |

## Hashes der visuellen Assets und Fonts

Die folgenden SHA-256-Werte wurden über die im MIG-002-Worktree ausgecheckten Dateien berechnet. Für 42 SVG-Icons und die alten App-/Installerbilder fand sich kein eindeutiger Herkunfts-/Lizenzbeleg im Repo. Die 12 Fontdateien liegen unter dem OFL-Hinweis im Fonts-Verzeichnis.

| Baseline-Pfad | SHA-256 |
| --- | --- |
| `src/main/resources/resources/icons/add.svg` | `cd152997536f84d3afba6febecde56c91644f6310756218509b70a69f3ce0f73` |
| `src/main/resources/resources/icons/app-rendered.png` | `fba4f2fbce9be3b5a780dd2925a58fb60bc097faca3a4f98c9d1e1425a6341e7` |
| `src/main/resources/resources/icons/app.svg` | `a3ff7405f927e27feef118ff36e8552a889dfd080f72db8b860ae79965ccb78e` |
| `src/main/resources/resources/icons/arrow-double-up.svg` | `49d2095c5ea91b6251592c650562f43cbf0f719c8e30f491733aed9c0f57ac3c` |
| `src/main/resources/resources/icons/arrow-down.svg` | `0a3ed22e6cc0cbef7a09b59feaecf444ac5ec5b1606d5badbb2d94dade218503` |
| `src/main/resources/resources/icons/arrow-up.svg` | `c0af26db35c14d5220365ce3bb09797bb6ccc974f84a1add0811a11f28980f6f` |
| `src/main/resources/resources/icons/check.svg` | `6d1affe102825147e96e320b24b7c45611f2c44ecdd264ea55b01711a4ef1a3c` |
| `src/main/resources/resources/icons/close-strong.svg` | `912fac32788193b32c19fcbc0f1b0d67beeda5dd293963ff271aaf94696ab97b` |
| `src/main/resources/resources/icons/close.svg` | `d5469549db971711f6ea59c149ea4b79c16a947c592f8592cb1e7ccc4c9358b4` |
| `src/main/resources/resources/icons/community.svg` | `fa7398cccbc25730ae436351c0151e30eadfe8d38efb8dc942b25d851ab5b515` |
| `src/main/resources/resources/icons/delete.svg` | `5b3b27f2f369df12f813c762f35d5f972ae6725236eb16e3d9ea62501416a600` |
| `src/main/resources/resources/icons/donate.svg` | `9c657dd5d7362a81d7a5ddce6aad8022fdf9ce51cf73f924765326decd13f8e8` |
| `src/main/resources/resources/icons/download.svg` | `26f691944e021c630ff2a203252cf20b01f534fc902988714d0166cc8a168eb5` |
| `src/main/resources/resources/icons/duplicate.svg` | `7aa595b8000405661b70f477575fd4bdb0e04a890c1a4269c965271bc1ec2116` |
| `src/main/resources/resources/icons/edit.svg` | `593934de80c0e75c26a7e0139dc7b67c558e96e63c80e1221cb800cf08c8c5ad` |
| `src/main/resources/resources/icons/events.svg` | `1ef699e119b2945b26b49ef737ba6a10dcd748644924c4e182b56c8941939bcb` |
| `src/main/resources/resources/icons/file.svg` | `3835382f7201c06efde19132124d4ee266d7b281d29f1f8e81b067ab652394bb` |
| `src/main/resources/resources/icons/folder-open.svg` | `b4703a36ab10020e2771e94370fe46001fa18d304976770f30f7619a9db3e557` |
| `src/main/resources/resources/icons/folder.svg` | `2a716337c8e61b1afdaf3d2f2aa9b5a24178ed778be4c002e64756a550e186f9` |
| `src/main/resources/resources/icons/globe.svg` | `298e1979f06ceaa3580ada620eb7c580efc409f5b34406f815e4b9d1b4fc9c06` |
| `src/main/resources/resources/icons/help.svg` | `cb920bd4be8c4535b5d296fff74452f7eb57a7e82f61bca0b04d0b0edd381700` |
| `src/main/resources/resources/icons/info.svg` | `0053e441daff038d46f55c1ac551873949c058f7a5642e06841b2a1912f759fc` |
| `src/main/resources/resources/icons/key.svg` | `cef16972ed4c94526244d6b50794efdebf14eeba4463f48b298884eaac96789a` |
| `src/main/resources/resources/icons/link.svg` | `28185c3f6d5bb785ebb9807109a061add4eac3d800ff6005fe845d40ffa31c94` |
| `src/main/resources/resources/icons/moon.svg` | `8f1c14a9417ceb2d0bda5dd0282a0ad60a274046381731399541f7f493c2e639` |
| `src/main/resources/resources/icons/package.svg` | `fb56e74ad01676cd5f68e8d99b533ff1f9c305263ac7cc456cc3d6cadf2e6d4e` |
| `src/main/resources/resources/icons/pause.svg` | `4a5b528b973716c05d34fa8c2eec014e0df913d250d27dfd4072a8d6eeb05d7c` |
| `src/main/resources/resources/icons/play.svg` | `8a914c488a14c3298c5c7b1370c14339323c99c2877c88490fd9f5dd22c7603e` |
| `src/main/resources/resources/icons/power.svg` | `938c468e0ffe50c26b014a631b894d8d6105f1c5603701c7876284e32b6c334c` |
| `src/main/resources/resources/icons/proxy.svg` | `1687136b5909308d0c9c4db51a8bc69b472fb46097887fb2f96edfee2c750fbf` |
| `src/main/resources/resources/icons/radio.svg` | `f2c1157a6662ac0dd9f6e5e260f780e85c537dc8ecc12e5fe2f2f19f7dc087b9` |
| `src/main/resources/resources/icons/refresh.svg` | `783f525554f159aae91dbe451a3fd000e6d88e2c2816291c7b3f304065999a67` |
| `src/main/resources/resources/icons/report.svg` | `aefa17715bfe2a04595d0b3c088bb30aae8df6ec029e51042c7c8f4483cc3603` |
| `src/main/resources/resources/icons/repository.svg` | `f32bf9852c85d7ac87b09b09a4ec9d1174d0d51bf89058d085a25106c54c759d` |
| `src/main/resources/resources/icons/save.svg` | `4626bf2528369b34c0a492a532c4f2ccf5f572a63b63ff12f99a855f4510a344` |
| `src/main/resources/resources/icons/settings.svg` | `d76af9aeb42130986ed7ec4f2725819e888fbfa2b0dad7346b5e800681886a81` |
| `src/main/resources/resources/icons/shield.svg` | `2dbaa6ff574d8112f92d7f65269c3f44ae8bc8cd21e8aeca47e15e0d2bb2209f` |
| `src/main/resources/resources/icons/shortcut.svg` | `31d14e7427c47b4b04bfd8aa29b232291175ee8fdaa805c9178b4e43f13ae3ce` |
| `src/main/resources/resources/icons/stop.svg` | `3f813ee5547d484953e3de2f323a968f97bd683f5ec8df5ae41070d3c8ad5599` |
| `src/main/resources/resources/icons/sun.svg` | `74b5bdb7b5686e6704e8d371bf9d0b1dd2774a40e1c0540d4acc7139ae0496b0` |
| `src/main/resources/resources/icons/update.svg` | `8e0ec7abb1c1364c3eb9eb1f674fc4c66107abff998c24823362dda295ecfe72` |
| `src/main/resources/resources/icons/upload.svg` | `bbf25ff852779d89969a16dc67463f9d289069a2422a920c1d2898f996dc14e3` |
| `src/main/resources/resources/icons/warning.svg` | `68e87700fb3336f769bce45e8e724b4223ffa45364ec226e8880958f410d9ad3` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-Bold.ttf` | `e6fd0e32993e9830a813e0c0083019ad7914cb45f3226328ea2b33750c12c1a4` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-BoldItalic.ttf` | `01475fb9fd1466c34db3dce4e8dd6bb782f0f6b78fb116b3a0c6d7beb1806ddc` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-Italic.ttf` | `0cf19ec6220127dcf6e0db6db852f0db32f287c6ce3cb4d470ef3773d8f65324` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-Medium.ttf` | `a9881c4185ed0ce54d980fd3a82a5e632c153e89c488d8389ee302759b89ae55` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-Regular.ttf` | `aaf98a18d72aadc29934de23d7e4d00951c28e7bf745969d92ef21986a4798ff` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-SemiBold.ttf` | `66a677c36ef1eb1be764485130a68c94595129e342eb16eab8e6be844047e566` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-Bold.ttf` | `288316099b1e0a47a4716d159098005eef7c0066921f34e3200393dbdb01947f` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-BoldItalic.ttf` | `948405a16cdc62701da5f4005ed068ca5f4d27061d98f7974ccfc37831d9581d` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-Italic.ttf` | `bbc051dd204b5019a1aa0bc0ae2aa8a05ab13e7a3f979fa357631dc7feb6833a` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-Medium.ttf` | `97ad806f526e41546d46365bb3a393145f75b7b1568913db74549ad8b8dba872` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-Regular.ttf` | `40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82` |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-SemiBold.ttf` | `78a843fade9d4612a5567302fb595b56976eb5fcebf4fea5a5912d638bafcde3` |
| `installer/assets/Arma3Sync.ico` | `2d5ff76f1767fb0c94a28e79a0181b625a5b7a2240ee3a15662a78f26fa48142` |
| `installer/assets/Arma3Sync.png` | `fba4f2fbce9be3b5a780dd2925a58fb60bc097faca3a4f98c9d1e1425a6341e7` |

## Verifikation und Grenzen

- `git show b7753ce:docs/migration/INVENTORY.md` ist die strukturelle Quelle für die 515 Baseline-Dateien und gruppiert Dokumentation, Quellen, UI-Assets, Fonts, JARs, Installer und Release-Dateien.
- `gradle dependencies --configuration runtimeClasspath` (Windows, Gradle 9.4.1) war erfolgreich; der Graph oben entspricht dem aufgelösten Runtime-Graph.
- Lokale JARs wurden als ZIP-Archive auf `META-INF/MANIFEST.MF`, LICENSE/NOTICE/COPYING/README-Einträge und SHA-256 geprüft. Das belegt nur Dateiinhalt/Identität, nicht die Rechtmäßigkeit der Beschaffung oder Weitergabe.
- Quellheader wurden in `src/main/java/fr/soe/a3s/jazsync/**` und `DataRange.java` geprüft. Bildursprung und Rechtekette konnten aus den Binärdateien/Repo-Metadaten nicht festgestellt werden.
- Kein individueller Rechtsrat, keine externe Upstream-Lizenzprüfung, keine Verifikation der Rechtekette von Projektcontributors und keine Entscheidung zur Zulässigkeit eines abgeleiteten Jazsync-Ports.
- Ausstehende Rechtefragen sind Gate-Befunde; MIG-002 allein schließt das P0-Gate daher nicht, solange RQ-02 bis RQ-05 ungeklärt sind.
