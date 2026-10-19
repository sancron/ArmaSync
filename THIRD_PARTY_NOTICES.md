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

## Rust-Abhängigkeiten des HTTP-Adapters (MIG-030B, Stand 2026-10-19, Prüfung nach RQ-06; keine Rechtsberatung)

Aufgenommen in `armasync-adapters`: `ureq` 3.4.2 mit `rustls` und `platform-verifier`, `sha1` 0.11.0. Normalbaum (aus `cargo tree`): `ureq`, `ureq-proto`, `http`, `httparse`, `base64`, `bytes`, `log`, `percent-encoding`, `once_cell`, `getrandom`, `itoa`, `cfg-if`, `rustls-pki-types`, `rustls-platform-verifier`, `utf8-zero`, `zeroize`, `windows-link`, `windows-sys`, `sha1`: **MIT OR Apache-2.0** (bzw. MIT); `rustls`: **Apache-2.0 OR ISC OR MIT**; `rustls-webpki`, `untrusted`: **ISC**; `subtle`: **BSD-3-Clause**; **`ring` 0.17: Apache-2.0 AND ISC** (beide Lizenztexte beilegen; enthält C/Assembler); **`webpki-roots` 1.0: CDLA-Permissive-2.0** (Mozilla-Wurzelzertifikatsdaten, im Baum, aber vom Client nicht zur Prüfung genutzt, solange der Plattformverifier gewählt ist; Notice trotzdem beilegen). Nur Tests (nicht ausgeliefert): `rcgen` (MIT OR Apache-2.0) mit `time`, `yasna`, `pem`. Die vollständige Release-BOM (alle Pakete, Texte, Systembestandteile) steht noch aus (RQ-06 bleibt offen).

**Nachtrag MIG-030H (2026-10-19):** `fs4` 1.1.0 (MIT OR Apache-2.0; Dateisperren) in `armasync-adapters`; keine weiteren Normalabhängigkeiten.

Neue Übernahmen erhalten Original-Commit/URL, Lizenz, Rechtebeleg, Zielpfad und Prüfdatum. Vor Rust-Release vollständigen Bericht aus Cargo.lock und ausgelieferten Assets/Systembestandteilen erzeugen. Die Java-Credits stimmen nicht vollständig mit Gradle überein und sind keine verlässliche Abhängigkeitsinventur.
