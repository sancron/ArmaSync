# Vollständiges Dateiinventar des Ausgangsstands

Stand 2026-10-18. Quelle: `git ls-tree -r --long b7753cea18b2ac2cc57e29f926901997a4de8570`. Alle 515 getrackten Dateien dieses Commits, ohne generierte Builds oder neue Migrationsdokumente. Größen sind Git-Blobgrößen in Bytes, nicht Windows-Dateigrößen nach Zeilenumbruchkonvertierung. Ein Eintrag ist eine strukturelle Zuordnung, keine Behauptung einer vollständigen Inhalts- oder Rechteprüfung jeder Datei.

Exakte Dateiinhalte über `git show legacy:<pfad>` prüfen. Der Baseline-Commit identifiziert sämtliche hier aufgeführten Blobs eindeutig. Handlungsempfehlungen sind Zielzustände nach den Gates in ROADMAP.md, keine bereits ausgeführten Änderungen.

| Baseline-Pfad | Bytes | Bereich / Migrationsbehandlung |
| --- | ---: | --- |
| `.github/workflows/publish-winget.yml` | 913 | CI: Branch-/Build-/Releaseprüfungen für neues Produkt einrichten |
| `.gitignore` | 661 | Konfiguration/Repositorydatei: Nutzung, Branding und Cutover einzeln prüfen |
| `BUILD.md` | 4689 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `CHANGELOG.md` | 196 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `CHANGELOGS_DE.md` | 4510 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `CHANGELOGS_EN.md` | 4064 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `LICENCE.md` | 34599 | Lizenz: unverändert erhalten, Geltungsbereich nachweisen |
| `MANIFEST_A3S.MF` | 506 | Build/Version: bis Cutover erhalten, Cargo/native Auslieferung ablösen |
| `MIGRATION_NOTES.md` | 1864 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `PERFORMANCE_PLAN.md` | 3097 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `README.md` | 10631 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `TESTING.md` | 2007 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `UPDATER.md` | 5970 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `a3s.xml` | 88 | Konfiguration/Repositorydatei: Nutzung, Branding und Cutover einzeln prüfen |
| `build.gradle` | 10820 | Build/Version: bis Cutover erhalten, Cargo/native Auslieferung ablösen |
| `changelogs/2026.1.1.md` | 1871 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `changelogs/2026.2.5.md` | 2916 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `changelogs/2026.2.6.md` | 856 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `changelogs/2026.3.14.md` | 2567 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `changelogs/2026.4.13.md` | 8578 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `docs/ARMA3_LAUNCHER_PRESET_EXPORT.md` | 4726 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `docs/ICON_GUIDE.md` | 4016 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `docs/JAVA_RUNTIME_PACKAGING.md` | 4135 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `installer/assets/Arma3Sync.ico` | 370070 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/assets/Arma3Sync.png` | 40553 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/Arma3Sync.nsi` | 8972 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/README.md` | 4117 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/bootstrapper/go.mod` | 39 | Go/Java-Bootstrapper: nach Cutover aus aktivem Release entfernen |
| `installer/nsis/bootstrapper/main.go` | 12181 | Go/Java-Bootstrapper: nach Cutover aus aktivem Release entfernen |
| `installer/nsis/bootstrapper/main_test.go` | 2253 | Go/Java-Bootstrapper: nach Cutover aus aktivem Release entfernen |
| `installer/nsis/build-bootstrapper.ps1` | 1275 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/build-installer.ps1` | 4021 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/create-launcher.ps1` | 5496 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/generate-icons.ps1` | 1563 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/include/Version.nsh` | 271 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/payload/Arma3Sync-console.bat` | 78 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/payload/Arma3Sync-console.sh` | 113 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/payload/Arma3Sync-noSingleInstancelock.bat` | 77 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/payload/Arma3Sync.bat` | 69 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/payload/Arma3Sync.sh` | 104 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/payload/updater.toml` | 1230 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `installer/nsis/payload/version.txt` | 31 | Installer/Assets: native Pakete und Lynxnode.it-Identität |
| `packaging/winget/README.md` | 1999 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `packaging/winget/vPzBrig21.Arma3Sync/2026.4.13/vPzBrig21.Arma3Sync.installer.yaml` | 633 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `packaging/winget/vPzBrig21.Arma3Sync/2026.4.13/vPzBrig21.Arma3Sync.locale.en-US.yaml` | 994 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `packaging/winget/vPzBrig21.Arma3Sync/2026.4.13/vPzBrig21.Arma3Sync.yaml` | 223 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `packaging/winget/vPzBrig21.Arma3Sync/2026.4.14/vPzBrig21.Arma3Sync.installer.yaml` | 633 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `packaging/winget/vPzBrig21.Arma3Sync/2026.4.14/vPzBrig21.Arma3Sync.locale.en-US.yaml` | 1013 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `packaging/winget/vPzBrig21.Arma3Sync/2026.4.14/vPzBrig21.Arma3Sync.yaml` | 223 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `packaging/winget/vPzBrig21.Arma3Sync/2026.4.15/vPzBrig21.Arma3Sync.installer.yaml` | 633 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `packaging/winget/vPzBrig21.Arma3Sync/2026.4.15/vPzBrig21.Arma3Sync.locale.en-US.yaml` | 1013 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `packaging/winget/vPzBrig21.Arma3Sync/2026.4.15/vPzBrig21.Arma3Sync.yaml` | 223 | Paketregistrierung: neue Identität und Upgradepfad prüfen |
| `release/README.md` | 6526 | Release: native Artefakte/Channels/Signaturen und Branchgates |
| `release/RELEASE_NOTES_2026.4.14.md` | 648 | Release: native Artefakte/Channels/Signaturen und Branchgates |
| `release/RELEASE_NOTES_2026.4.15.md` | 604 | Release: native Artefakte/Channels/Signaturen und Branchgates |
| `release/beta-release.ps1` | 3397 | Release: native Artefakte/Channels/Signaturen und Branchgates |
| `release/generate-version.ps1` | 1598 | Release: native Artefakte/Channels/Signaturen und Branchgates |
| `release/release-components.json` | 970 | Release: native Artefakte/Channels/Signaturen und Branchgates |
| `release/release.ps1` | 8243 | Release: native Artefakte/Channels/Signaturen und Branchgates |
| `resources/lib/JTattoo-1.6.10.jar` | 1133022 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/angus-activation-2.0.0.jar` | 27439 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/commons-codec-1.13.jar` | 344339 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/commons-codec-1.16.0.jar` | 360738 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/commons-io-2.0.1.jar` | 159509 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/commons-io-2.15.0.jar` | 500628 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/commons-logging-1.2.jar` | 61829 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/commons-net-3.10.0.jar` | 322780 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/commons-net-3.4.jar` | 283502 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/flatlaf-3.1.1.jar` | 790619 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/flatlaf-3.6.1.jar` | 927031 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/flatlaf-extras-3.6.1.jar` | 118666 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/httpclient-4.5.14.jar` | 785639 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/httpclient-4.5.3.jar` | 747794 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/httpcore-4.4.16.jar` | 327891 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/httpcore-4.4.6.jar` | 323824 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/istack-commons-runtime-4.1.1.jar` | 26147 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/jakarta.activation-api-2.1.1.jar` | 66651 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/jakarta.xml.bind-api-4.0.0.jar` | 127111 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/jaxb-api-2.3.0.jar` | 125632 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/jaxb-core-4.0.2.jar` | 138596 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/jaxb-runtime-4.0.2.jar` | 908151 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/jshortcut-0.4-oberzalek.jar` | 136087 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/jsvg-2.0.0.jar` | 723652 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/junique-1.0.4.jar` | 10262 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/sardine-5.12.jar` | 138330 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/sardine-5.8.jar` | 134837 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/slf4j-api-2.0.7.jar` | 63635 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/slf4j-simple-2.0.7.jar` | 15972 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `resources/lib/txw2-4.0.2.jar` | 73267 | Binäre Drittquelle: Lizenz/Provenienz prüfen, nicht ins Rust-Paket übernehmen |
| `settings.gradle` | 249 | Build/Version: bis Cutover erhalten, Cargo/native Auslieferung ablösen |
| `src/main/java/fr/soe/a3s/console/CommandConsole.java` | 22725 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/console/CommandGeneral.java` | 16740 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/console/CommandLine.java` | 3264 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/constant/CheckRepositoriesFrequency.java` | 915 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/ConsoleCommands.java` | 151 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/DefaultProfileName.java` | 271 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/DownloadStatus.java` | 86 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/EncryptionMode.java` | 614 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/GameDLCs.java` | 1434 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/GameExecutables.java` | 441 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/GameSystemFolders.java` | 196 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/GameVersions.java` | 264 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/IconResize.java` | 992 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/LookAndFeel.java` | 637 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/MaxMemoryValues.java` | 237 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/MinimizationType.java` | 573 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/ModsetType.java` | 76 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/ProtocolType.java` | 1651 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/RepositoryStatus.java` | 317 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/StartWithOS.java` | 447 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/constant/TimeOutValues.java` | 247 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/controller/ObservableCountErrors.java` | 187 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObservableCountInt.java` | 172 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObservableDownload.java` | 692 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObservableEnd.java` | 149 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObservableError.java` | 203 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObservableErrorMessage.java` | 243 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObservableText.java` | 164 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObservableUncompress.java` | 186 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObservableUpload.java` | 445 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverConnectionLost.java` | 99 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverCountInt.java` | 102 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverCountLong.java` | 104 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverDownload.java` | 497 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverEnd.java` | 86 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverError.java` | 135 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverErrorMessage.java` | 158 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverProceed.java` | 93 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverStart.java` | 90 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverText.java` | 100 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverUncompress.java` | 218 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/controller/ObserverUpload.java` | 344 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/A3SFilesAccessor.java` | 6993 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/AddonDAO.java` | 1178 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/AfficheurFlux.java` | 701 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/ApplicationPaths.java` | 12059 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/CommonDAO.java` | 3091 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/ConfigurationDAO.java` | 5774 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/DataAccessConstants.java` | 1955 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/EncryptionProvider.java` | 1030 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/FileAccessMethods.java` | 8619 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/LauncherDAO.java` | 14585 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/PreferencesDAO.java` | 2385 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/ProfileDAO.java` | 3358 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/StreamReader.java` | 647 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/WinRegistry.java` | 5208 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/connection/AbstractConnexionDAO.java` | 22967 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/connection/AutoConfigURLAccessMethods.java` | 5646 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/connection/RemoteFile.java` | 1854 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/connection/ftp/FtpDAO.java` | 25648 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/connection/http/DataRange.java` | 1607 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/connection/http/HttpDAO.java` | 21234 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/connection/sftp/SftpDAO.java` | 24242 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/connection/webdav/WebdavDAO.java` | 15309 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/repository/RepositoryBuildProcessor.java` | 23456 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/repository/RepositoryDAO.java` | 13304 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/repository/RepositoryDeleteZSyncProcessor.java` | 2730 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/repository/RepositorySHA1Processor.java` | 5843 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/repository/RepositoryZsyncProcessor.java` | 3058 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/zip/DeleteZipBatchProcessor.java` | 2493 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/zip/UnZipFlowProcessor.java` | 3588 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/zip/ZipBatchProcessor.java` | 2862 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/dao/zip/ZipDAO.java` | 4834 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/domain/AbstractProtocole.java` | 3529 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/AbstractProtocoleFactory.java` | 1204 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/Addon.java` | 615 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/AutoConfig.java` | 695 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/Ftp.java` | 665 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/Http.java` | 660 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/Preferences.java` | 2872 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/Profile.java` | 2126 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/Proxy.java` | 590 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/Sftp.java` | 605 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/Socks.java` | 612 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/TreeDirectory.java` | 2920 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/TreeLeaf.java` | 1946 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/TreeNode.java` | 443 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/Webdav.java` | 671 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/Acre2Options.java` | 564 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/AcreOptions.java` | 782 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/AiAOptions.java` | 634 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/BikeyExtractOptions.java` | 653 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/Configuration.java` | 3928 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/ExternalApplication.java` | 1209 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/FavoriteServer.java` | 1619 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/LauncherOptions.java` | 4562 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/Proxy.java` | 568 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/RptOptions.java` | 349 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/configration/TfarOptions.java` | 783 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/AutoConfig.java` | 988 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/Changelog.java` | 1323 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/Changelogs.java` | 363 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/Event.java` | 1453 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/Events.java` | 356 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/FileAttributes.java` | 549 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/Repository.java` | 11587 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/ServerInfo.java` | 2467 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/Sync.java` | 393 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/SyncTreeDirectory.java` | 3180 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/SyncTreeLeaf.java` | 2893 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/SyncTreeNode.java` | 572 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/domain/repository/SyncTreeNodeMethods.java` | 1721 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/AutoConfigDTO.java` | 454 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/ChangelogDTO.java` | 823 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/DownloadReportDTO.java` | 61 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/EventDTO.java` | 1456 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/ProtocolDTO.java` | 1491 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/RepositoryDTO.java` | 2169 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/ServerInfoDTO.java` | 764 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/TreeDirectoryDTO.java` | 2855 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/TreeLeafDTO.java` | 3368 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/TreeNodeDTO.java` | 388 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/configuration/ConfigurationDTO.java` | 1560 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/configuration/ExternalApplicationDTO.java` | 718 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/configuration/FavoriteServerDTO.java` | 1058 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/configuration/LauncherOptionsDTO.java` | 4176 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/configuration/PreferencesDTO.java` | 2021 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/configuration/ProxyDTO.java` | 474 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/sync/SyncTreeDirectoryDTO.java` | 3870 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/sync/SyncTreeLeafDTO.java` | 3713 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/sync/SyncTreeNodeDTO.java` | 957 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/dto/sync/SyncTreeNodeDTOMethods.java` | 2267 | Core: neutrale Modelle/Verträge und Datengrenzen ableiten |
| `src/main/java/fr/soe/a3s/exception/ApplicationException.java` | 374 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/CheckException.java` | 173 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/ConnectionExceptionFactory.java` | 2399 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/CreateDirectoryException.java` | 499 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/DeleteDirectoryException.java` | 499 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/IncompleteFileTransferException.java` | 481 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/LaunchException.java` | 164 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/LoadingException.java` | 210 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/ProfileException.java` | 164 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/WritingException.java` | 220 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/remote/RemoteAutoconfigFileNotFoundException.java` | 393 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/remote/RemoteChangelogsFileNotFoundException.java` | 393 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/remote/RemoteEventsFileNotFoundException.java` | 381 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/remote/RemoteRepositoryException.java` | 236 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/remote/RemoteServerInfoFileNotFoundException.java` | 384 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/remote/RemoteSyncFileNotFoundException.java` | 374 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/AutoConfigFileNotFoundException.java` | 308 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/ChangelogsFileNotFoundExeption.java` | 306 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/EventsFileNotFoundException.java` | 296 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/RepositoryDefaultDownloadLocationNotFoundException.java` | 318 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/RepositoryException.java` | 222 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/RepositoryMainFolderLocationNotFoundException.java` | 297 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/RepositoryNotFoundException.java` | 238 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/RepositoryServerInfoReadException.java` | 577 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/ServerInfoNotFoundException.java` | 308 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/exception/repository/SyncFileNotFoundException.java` | 291 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/jazsync/BrokenMD4.java` | 4653 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/ChainingHash.java` | 3853 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/ChecksumPair.java` | 7071 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/Configuration.java` | 4988 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/FileMaker.java` | 15236 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/Generator.java` | 9353 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/JarsyncProvider.java` | 1667 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/MD4.java` | 11018 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/MetaFileReader.java` | 10935 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/RollingChecksum.java` | 3348 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/Rsum.java` | 4229 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsync/Util.java` | 5056 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsyncmake/HeaderMaker.java` | 4103 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/jazsyncmake/MetaFileMaker.java` | 6197 | Delta: Golden Vectors und Herkunft sichern, gesonderter Port |
| `src/main/java/fr/soe/a3s/main/ArmA3Sync.java` | 10040 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/main/Version.java` | 1657 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/service/AddonService.java` | 22139 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/Arma3LauncherPresetExporter.java` | 15099 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/Arma3LauncherPresetImporter.java` | 13968 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/CommonService.java` | 5021 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/ConfigurationService.java` | 13857 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/ConnectionService.java` | 19779 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/EventExtractor.java` | 3533 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/LaunchService.java` | 14503 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/ObjectDTOtransformer.java` | 18213 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/PreferencesService.java` | 2496 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/ProfileService.java` | 21265 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/RepositoryService.java` | 55064 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/SslValidationPolicy.java` | 2125 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/UpdaterProcess.java` | 3776 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/WorkshopMetadataStore.java` | 4709 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/administration/FilesUploadManager.java` | 3332 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/administration/RepositoryBuildProcessor.java` | 1931 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/administration/RepositoryCheckProcessor.java` | 3502 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/administration/RepositoryUploadProcessor.java` | 12781 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/connection/AbstractConnectionProcessor.java` | 2766 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/connection/ConnectionCheckProcessor.java` | 5453 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/connection/ConnectionCompletionProcessor.java` | 3973 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/connection/ConnectionDeleteProcessor.java` | 1225 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/connection/ConnectionDownloadProcessor.java` | 6371 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/connection/ConnectionUploadProcessor.java` | 8515 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/synchronization/FileSynchronizationConnectionWaitingListener.java` | 1670 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/synchronization/FilesCheckProcessor.java` | 2764 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/synchronization/FilesCompletionProcessor.java` | 3005 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/synchronization/FilesSynchronizationDiskUsageListerner.java` | 2907 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/synchronization/FilesSynchronizationManager.java` | 8021 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/synchronization/FilesSynchronizationProcessor.java` | 12036 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/service/synchronization/FilesSynchronizationReportManager.java` | 6125 | Core/Application/Adapter: Verantwortlichkeiten aufteilen |
| `src/main/java/fr/soe/a3s/ui/AbstractDialog.java` | 2089 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/AbstractProgressDialog.java` | 2665 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/CheckBoxList.java` | 2311 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/ColumnsAutoSizer.java` | 3646 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/ErrorLogDialog.java` | 2602 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/ExtensionFilter.java` | 875 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/Facade.java` | 3184 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/FontInstaller.java` | 24938 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/ImagePanel.java` | 859 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/ImageResizer.java` | 2216 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/ThemeManager.java` | 2160 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/UIConstants.java` | 1048 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/UiBootstrap.java` | 1189 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/UiColors.java` | 1860 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/UiStyle.java` | 9175 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/components/ContrastProgressBarUI.java` | 2551 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/groups/AbstractGroupDialog.java` | 2155 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/groups/AddGroupDialog.java` | 2314 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/groups/DuplicateGroupDialog.java` | 2445 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/groups/RenameGroupDialog.java` | 1587 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/help/AboutCreditsDialog.java` | 6790 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/help/AboutDialog.java` | 5002 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/help/AutoConfigExportDialog.java` | 9763 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/help/AutoConfigImportDialog.java` | 4603 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/help/PreferencesDialog.java` | 16816 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/help/ProxyConfigurationDialog.java` | 5159 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/icon/IconState.java` | 94 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/icon/IconTone.java` | 137 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/icon/Icons.java` | 4200 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/icon/UiIcon.java` | 2535 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/AddonOptionsPanel.java` | 13870 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/AddonsPanel.java` | 64851 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/ExternalApplicationsPanel.java` | 12277 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/InfoPanel.java` | 1199 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/LaunchPanel.java` | 12662 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/LauncherOptionsPanel.java` | 39627 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/MainPanel.java` | 53460 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/OnlinePanel.java` | 11951 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/SyncPanel.java` | 22058 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/dialogs/ExternalApplicationsEditionDialog.java` | 6837 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/dialogs/InfoUpdatedRepositoryDialog.java` | 2810 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/dialogs/ModdsetsSelectionDialog.java` | 5235 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/dialogs/WelcomeDialog.java` | 5327 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/tasks/TaskCheckRepositories.java` | 7018 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/tasks/TasksManager.java` | 625 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/tree/AddonTreeModel.java` | 3460 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/tree/CheckTreeCellRenderer.java` | 2341 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/tree/IndeterminateIcon.java` | 922 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/tree/MyRenderer.java` | 1275 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/main/tree/TreeDnD2.java` | 8866 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/profiles/ProfileEditionDialog.java` | 3892 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/profiles/ProfileSelectionDialog.java` | 9108 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/AdminPanel.java` | 33615 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/DownloadPanel.java` | 50822 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/EventsPanel.java` | 35963 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/RepositoryPanel.java` | 4973 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/BuildRepositoryOptionsDialog.java` | 22471 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/ChangelogPanel.java` | 8463 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/ConnectionLostDialog.java` | 2057 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/DownloadSettingsDialog.java` | 11956 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/ReportDialog.java` | 3725 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/connection/ConnectionPanel.java` | 8792 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/connection/DescriptionPanel.java` | 10519 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/connection/OptionsPanel.java` | 2473 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/connection/ProtocolPanel.java` | 4158 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/connection/ProxyPanel.java` | 1052 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/connection/RepositoryEditionDialog.java` | 6320 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/connection/UploadEventsConnectionDialog.java` | 4483 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/connection/UploadRepositoryConnectionDialog.java` | 6135 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/error/ErrorsListDialog.java` | 3481 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/error/HeaderErrorDialog.java` | 2502 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/error/UnexpectedErrorDialog.java` | 2566 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/progress/ProgressConnectionAsAdminDialog.java` | 3993 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/progress/ProgressModsetsSelectionDialog.java` | 2665 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/progress/ProgressSynchronizationDialog.java` | 4924 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/progress/ProgressTestPartiaFileTransfer.java` | 2724 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/dialogs/progress/ProgressUploadEventsDialog.java` | 2886 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/events/AbstractEventDialog.java` | 3443 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/events/EventAddDialog.java` | 1573 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/events/EventRenameDialog.java` | 2043 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/tree/AddonSyncTreeModel.java` | 3642 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/tree/CheckTreeCellRendererRepository.java` | 1845 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/tree/MyRendererRepository.java` | 2340 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/workers/AddonsAutoUpdater.java` | 3474 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/workers/AddonsChecker.java` | 8378 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/workers/AddonsDownloader.java` | 14467 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/workers/RepositoryBuilder.java` | 5495 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/workers/RepositoryChecker.java` | 6730 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/workers/RepositoryUploader.java` | 9004 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/workers/SwingUi.java` | 381 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/repository/workers/UserconfigUpdater.java` | 3730 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/theme/ThemeDefinition.java` | 739 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/theme/ThemeLibrary.java` | 2373 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/theme/ThemeMetrics.java` | 2004 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/theme/ThemePalette.java` | 3859 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/theme/ThemeShadow.java` | 888 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/theme/ThemeTokens.java` | 10545 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/WizardDialog.java` | 5159 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/acre2/FirstPageACRE2InstallerDialog.java` | 11215 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/acre2/SecondPageACRE2InstallerDialog.java` | 6003 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/bikey/BiKeyExtractorDialog.java` | 8968 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/bikey/ExtractProgressDialog.java` | 1929 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/rpt/LogFileTailer.java` | 3717 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/rpt/LogFileTailerListener.java` | 357 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/rpt/RptViewerPanel.java` | 3494 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/rpt/Tail.java` | 2299 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/tfar/FirstPageTFARInstallerPanel.java` | 17994 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/ui/tools/tfar/SecondPageTFARInstallerPanel.java` | 9213 | GUI: Use-Cases/UI neu entwickeln, Accessibility nachweisen |
| `src/main/java/fr/soe/a3s/utils/DebugLogger.java` | 5841 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/utils/RepositoryConsoleErrorPrinter.java` | 578 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/java/fr/soe/a3s/utils/UnitConverter.java` | 1878 | Einstieg/CLI/Utilities: in getrennte Frontends/Adapter übertragen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-Bold.ttf` | 128624 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-BoldItalic.ttf` | 130400 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-Italic.ttf` | 128088 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-Medium.ttf` | 127468 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-OFL.txt` | 4387 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-Regular.ttf` | 125856 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Geist-SemiBold.ttf` | 127676 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-Bold.ttf` | 420428 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-BoldItalic.ttf` | 425296 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-Italic.ttf` | 417388 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-Medium.ttf` | 417300 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-Regular.ttf` | 411640 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/Inter-SemiBold.ttf` | 419744 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/fr/soe/a3s/ui/fonts/OFL.txt` | 4377 | Font: Wiederverwendung nur mit zugehörigen OFL-Hinweisen |
| `src/main/resources/resources/icons/add.svg` | 148 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/app-rendered.png` | 40553 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/app.svg` | 15223 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/arrow-double-up.svg` | 426 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/arrow-down.svg` | 334 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/arrow-up.svg` | 336 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/check.svg` | 179 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/close-strong.svg` | 282 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/close.svg` | 207 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/community.svg` | 250246 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/delete.svg` | 217 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/donate.svg` | 850 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/download.svg` | 150 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/duplicate.svg` | 386 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/edit.svg` | 441 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/events.svg` | 317 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/file.svg` | 196 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/folder-open.svg` | 278 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/folder.svg` | 203 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/globe.svg` | 986 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/help.svg` | 618 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/info.svg` | 380 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/key.svg` | 843 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/link.svg` | 608 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/moon.svg` | 383 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/package.svg` | 252 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/pause.svg` | 143 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/play.svg` | 132 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/power.svg` | 532 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/proxy.svg` | 529 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/radio.svg` | 420 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/refresh.svg` | 320 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/report.svg` | 353 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/repository.svg` | 611 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/save.svg` | 304 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/settings.svg` | 580 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/shield.svg` | 266 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/shortcut.svg` | 283 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/stop.svg` | 130 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/sun.svg` | 1073 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/update.svg` | 431 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/upload.svg` | 156 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/main/resources/resources/icons/warning.svg` | 235 | Asset: Herkunft/Branding prüfen, neue Assets gezielt auswählen |
| `src/test/java/fr/soe/a3s/dao/A3SFilesAccessorTest.java` | 2611 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/dao/ApplicationPathsTest.java` | 3688 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/dao/FileAccessMethodsTest.java` | 1805 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/dao/PreferencesDAOTest.java` | 2736 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/dao/connection/AutoConfigURLAccessMethodsTest.java` | 1123 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/dao/connection/sftp/SftpDAOTest.java` | 877 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/domain/ProtocolSupportTest.java` | 1244 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/domain/RepositoryUploadSettingsTest.java` | 961 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/main/VersionTest.java` | 320 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/service/Arma3LauncherPresetExporterTest.java` | 9293 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/service/Arma3LauncherPresetImporterTest.java` | 4786 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/service/SslValidationPolicyTest.java` | 858 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/ui/main/tasks/TaskCheckRepositoriesTest.java` | 1435 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `src/test/java/fr/soe/a3s/ui/theme/ThemeTokensTest.java` | 897 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `updater/README.md` | 6619 | Dokumentation: historisch erhalten, neue Rust-Anleitung getrennt führen |
| `updater/build.gradle` | 2419 | Build/Version: bis Cutover erhalten, Cargo/native Auslieferung ablösen |
| `updater/settings.gradle` | 39 | Build/Version: bis Cutover erhalten, Cargo/native Auslieferung ablösen |
| `updater/src/main/java/fr/soe/a3sUpdater/config/TomlConfigDAO.java` | 4076 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/config/UpdateConfig.java` | 6506 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/console/Console.java` | 4973 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/controller/Observable.java` | 178 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/controller/Observateur.java` | 121 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/dao/DataAccessConstants.java` | 1306 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/dao/DownloadCountingOutputStream.java` | 1466 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/dao/FtpDAO.java` | 18508 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/dao/GitHubReleaseDAO.java` | 5347 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/dao/HttpDAO.java` | 7951 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/dao/JsonManifestDAO.java` | 10482 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/dao/XmlDAO.java` | 2945 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/exception/FinderException.java` | 152 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/exception/FtpException.java` | 230 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/exception/WritingException.java` | 242 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/exception/XmlException.java` | 230 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/main/ArmA3SyncUpdater.java` | 3252 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/model/UpdateManifest.java` | 456 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/model/UpdateSource.java` | 195 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/service/DiagnosticLog.java` | 3475 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/service/ElevationSupport.java` | 22537 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/service/Service.java` | 12248 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/service/VersionComparator.java` | 983 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/ui/Facade.java` | 489 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/ui/ImageResizer.java` | 513 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/ui/UIConstants.java` | 704 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/main/java/fr/soe/a3sUpdater/ui/Updater.java` | 9495 | Updater: Trustmodell und nativen Dateiaustausch neu entwickeln |
| `updater/src/test/java/fr/soe/a3sUpdater/config/UpdateConfigTest.java` | 2795 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `updater/src/test/java/fr/soe/a3sUpdater/dao/FtpDAOManagedFilesTest.java` | 5916 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `updater/src/test/java/fr/soe/a3sUpdater/dao/FtpDAOSafetyTest.java` | 5193 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `updater/src/test/java/fr/soe/a3sUpdater/dao/GitHubReleaseDAOTest.java` | 2291 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `updater/src/test/java/fr/soe/a3sUpdater/dao/JsonManifestDAOTest.java` | 1939 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `updater/src/test/java/fr/soe/a3sUpdater/service/ElevationSupportTest.java` | 5432 | Tests: fachliche Erwartungen/Fixtures sichern, Rust-Parität prüfen |
| `version.properties` | 82 | Build/Version: bis Cutover erhalten, Cargo/native Auslieferung ablösen |
