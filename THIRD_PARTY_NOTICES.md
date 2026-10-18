# Herkunft und Drittbestandteile

**Vorläufiges Herkunftsregister**, kein abgeschlossener Lizenzscan. Neue Marke/Publisher: **Lynxnode.it**; Produkt: **ArmaSync**.

| Bestandteil | Beobachteter Nachweis | Behandlung |
| --- | --- | --- |
| Java-Anwendung | `legacy` bei `b7753cea18b2ac2cc57e29f926901997a4de8570`, `LICENCE.md`, `main/ArmA3Sync.java` | GPLv3 erhalten; ursprünglicher Autor Major_Shepard / Sons of Exiled und weitere Mitwirkende als Herkunft dokumentiert. |
| Jazsync / Jarsync / GNU-Crypto-Ableitungen | Header in `jazsync/` und `jazsyncmake/` | GPL-2.0-or-later; u. a. Tomas Hlavnicka, Casey Marshall und Free Software Foundation. Portierung gesondert prüfen. |
| Fork-Wartung | README/Credits auf `legacy`, Git-Verlauf | Unter anderem squadalpha und vPzBrig21; daraus keine neue Publisheridentität ableiten. |
| Apache Commons, HttpComponents, MINA SSHD | `build.gradle` | Lizenztexte über aufgelöste Artefakte samt Transitiven prüfen; keine automatische Rust-Übernahme. |
| FlatLaf / Extras / JSVG / Sardine | `build.gradle`, Artefakte | Lizenz und Nutzung prüfen. |
| JTattoo, JUnique, JShortcut | lokale JARs unter `resources/lib/` | Provenienz/Lizenz prüfen; nicht als freigegeben voraussetzen. |
| Geist / Inter | `src/main/resources/fr/soe/a3s/ui/fonts/*OFL.txt` | OFL-Texte bei Wiederverwendung beilegen; Fonts nicht umbenennen. |
| Alte Logos / SVGs / Installericons | `installer/assets/`, Icon-Ressourcen | Herkunft prüfen; unabhängig erstellte Lynxnode.it-Assets vorsehen. |
| Arma 3, DLC/CDLC und externe Tools | Spieleintegration | Sachlich erforderliche Bezeichnungen erhalten; keine Markenrechte behaupten. |

Neue Übernahmen erhalten Original-Commit/URL, Lizenz, Rechtebeleg, Zielpfad und Prüfdatum. Vor Rust-Release vollständigen Bericht aus Cargo.lock und ausgelieferten Assets/Systembestandteilen erzeugen. Die Java-Credits stimmen nicht vollständig mit Gradle überein und sind keine verlässliche Abhängigkeitsinventur.
