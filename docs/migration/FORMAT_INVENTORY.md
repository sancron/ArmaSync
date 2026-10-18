# Formatinventar der Altdaten (MIG-001)

Stand: 2026-10-18. Quelle: Baseline `b7753cea18b2ac2cc57e29f926901997a4de8570`, Quellcode der DAO-Aufrufstellen, mit dem Generator erzeugte synthetische Fixtures (`fixtures/legacy-formats/`) und deren JVM-freie Dekodierung (`migration-tools/fixture-generator/verify.py`). Alle Aussagen zu Verhalten sind durch Fixtures belegt, sofern nicht „nicht geprüft“ vermerkt ist. Die alten Klassennamen stehen hier ausschließlich als **Wire-Daten** gemäß [ADR-0003](../adr/0003-legacy-formats.md); sie sind keine Vorgabe für Rust-Typen.

## 1. Gemeinsamer Container

Jede Altdatei (außer der verschlüsselten Variante unten) ist `GZIP( Java-Object-Serialization-Stream )` mit genau einem Top-Level-Objekt. Gelesen wird ausschließlich über `A3SFilesAccessor.read` (`src/main/java/fr/soe/a3s/dao/A3SFilesAccessor.java`), geschrieben über `A3SFilesAccessor.write` (Temp-Datei + `ATOMIC_MOVE`). Metadaten-Uploads (`FtpDAO.uploadObjectFile`, SFTP/WebDAV analog) schreiben dasselbe Format ohne Temp-Datei.

Lesefilter der Baseline (`LEGACY_OBJECT_FILTER`): Tiefe ≤ 100, Referenzen ≤ 250 000, Streamgröße ≤ 256 MiB; Klassen nur aus dem Anwendungspaket, ausgewählte JDK-Typen (`String`, Wrapper, `Enum`, `ArrayList`, `HashMap`, `HashSet`, `LinkedHashMap`, `LinkedHashSet`, `TreeMap`, `TreeSet`, `Properties`, `Date`, `Map$Entry`) und `javax.crypto.SealedObject`. **Nicht** geprüft werden Array-Längen und die GZIP-Prüfsumme (siehe Abschnitt 6).

## 2. Dateitypen und Aufrufstellen

| Datei | Top-Level-Klasse | Lesen / Schreiben (Quelle) | Verschlüsselung | Fixture |
| --- | --- | --- | --- | --- |
| `a3s.cfg` | `domain.configration.Configuration` | `ConfigurationDAO` Z. 29 / 48 | nein | `valid/local/a3s.cfg` |
| `a3s.prefs` | `domain.Preferences` | `PreferencesDAO` Z. 20 / 39 | nein | `valid/local/a3s.prefs` |
| `<Name>.a3s.profile` | `domain.Profile` | `ProfileDAO` Z. 33 / 84 | nein | `valid/local/profiles/*` |
| `<Name>.a3s.repository` | `domain.repository.Repository` in `SealedObject` | `RepositoryDAO` Z. 118 / 226 | **ja: AES (JCE-Standard `"AES"`), statischer Schlüssel** in `EncryptionProvider` | `valid/local/repositories/*` |
| `export.a3s.autoconfig` | `domain.AutoConfig` (**nicht** `domain.repository.AutoConfig`) | `CommonDAO` Z. 28 / 43 | nein | `valid/local/autoconfig-export/*` |
| `.a3s/autoconfig` | `domain.repository.AutoConfig` | `RepositoryBuildProcessor` Z. 340; `RepositoryDAO` Z. 278; `AbstractConnexionDAO` Z. 194 | nein | `valid/server/*/.a3s/autoconfig` |
| `.a3s/serverinfo` | `ServerInfo` | `RepositoryBuildProcessor` Z. 332; `RepositoryDAO` Z. 258 | nein | `…/serverinfo` |
| `.a3s/sync` | `SyncTreeDirectory` (Baum mit `SyncTreeLeaf`) | `RepositoryBuildProcessor` Z. 328; `RepositoryDAO` Z. 248 | nein | `…/sync` |
| `.a3s/changelogs` | `Changelogs` | `RepositoryBuildProcessor` Z. 336; `RepositoryDAO` Z. 268 | nein | `…/changelogs` |
| `.a3s/events` | `Events` | `RepositoryBuildProcessor` Z. 344; `RepositoryDAO` Z. 286 / 304 | nein | `…/events` |
| `<datei>.zsync` | zsync-Text-Header + binäre Prüfsummen | `MetaFileMaker` (`jazsyncmake`) | nein | `valid/zsync/*` |

Verschlüsselung: **nur** `.a3s.repository` ist verschlüsselt (`A3SFilesAccessor.write(Serializable, Cipher, File)` wird ausschließlich von `RepositoryDAO` mit `EncryptionProvider` aufgerufen). Der Schlüssel ist eine Konstante im Quellcode (`EncryptionProvider.java`); er wird hier bewusst **nicht** kopiert. Das ist Obfuskation, kein Geheimnisschutz; nach dem Entschlüsseln liegen Passwörter des Repository-Objekts im Klartext vor. `AutoConfigURLAccessMethods.parseA3S` (Z. ~100) erzeugt einen Decryption-Cipher, nutzt ihn aber nicht und wird nicht aufgerufen (`parseStandard` ist aktiv).

Speicherorte (AppData/XDG/portabel) bestimmt `ApplicationPaths`; sie sind nicht Teil dieses Tasks.

## 3. Class-/UID-Inventar

Beobachtet in den Streams der Fixtures (`fixtures/legacy-formats/generated/descriptor-inventory.json`, abgeleitet) und gegen die Reflektion der Baseline-Klassen (`class-inventory.json`) abgeglichen: **0 Abweichungen**. Es gibt 44 Deskriptoren. Das Inventar umfasst genau die Typen, die in den Fixtures erreichbar sind; Typen, die kein Fixture speichert, fehlen und sind nicht geprüft.

| Klasse (Wire-Deskriptor) | serialVersionUID (hex, 64 Bit) | im Quellcode explizit | Flags | Felder | Superklasse | Fixtures |
| --- | --- | --- | --- | ---: | --- | ---: |
| `[B` | `acf317f8060854e0` | – | `0x02` | 0 | – | 2 |
| `fr.soe.a3s.constant.CheckRepositoriesFrequency` | `0000000000000000` | Enum (UID 0) | `0x12` | 0 | `java.lang.Enum` | 1 |
| `fr.soe.a3s.constant.IconResize` | `0000000000000000` | Enum (UID 0) | `0x12` | 0 | `java.lang.Enum` | 1 |
| `fr.soe.a3s.constant.LookAndFeel` | `0000000000000000` | Enum (UID 0) | `0x12` | 0 | `java.lang.Enum` | 1 |
| `fr.soe.a3s.constant.MinimizationType` | `0000000000000000` | Enum (UID 0) | `0x12` | 0 | `java.lang.Enum` | 1 |
| `fr.soe.a3s.constant.ModsetType` | `0000000000000000` | Enum (UID 0) | `0x12` | 0 | `java.lang.Enum` | 2 |
| `fr.soe.a3s.constant.ProtocolType` | `0000000000000000` | Enum (UID 0) | `0x12` | 0 | `java.lang.Enum` | 5 |
| `fr.soe.a3s.constant.StartWithOS` | `0000000000000000` | Enum (UID 0) | `0x12` | 0 | `java.lang.Enum` | 1 |
| `fr.soe.a3s.domain.AbstractProtocole` | `f3b306f9aa50b9dd` | ja | `0x02` | 9 | – | 5 |
| `fr.soe.a3s.domain.AutoConfig` | `59282a8550e6ed0e` | **nein (berechnet)** | `0x02` | 3 | – | 1 |
| `fr.soe.a3s.domain.Http` | `ffffffffffffffec` | ja | `0x02` | 0 | `fr.soe.a3s.domain.AbstractProtocole` | 3 |
| `fr.soe.a3s.domain.Preferences` | `619f5446da354550` | ja | `0x02` | 7 | – | 1 |
| `fr.soe.a3s.domain.Profile` | `c3017806ddd9c143` | ja | `0x02` | 7 | – | 3 |
| `fr.soe.a3s.domain.Sftp` | `0000000000000001` | ja | `0x02` | 0 | `fr.soe.a3s.domain.AbstractProtocole` | 1 |
| `fr.soe.a3s.domain.Socks` | `ffffffffffffffd8` | ja | `0x02` | 0 | `fr.soe.a3s.domain.AbstractProtocole` | 1 |
| `fr.soe.a3s.domain.TreeDirectory` | `c9a3a7b4c01584af` | ja | `0x02` | 8 | – | 3 |
| `fr.soe.a3s.domain.TreeLeaf` | `787bac2f5273ecda` | ja | `0x02` | 7 | – | 2 |
| `fr.soe.a3s.domain.configration.Acre2Options` | `8328589b41fd93a7` | ja | `0x02` | 2 | – | 1 |
| `fr.soe.a3s.domain.configration.AcreOptions` | `e9d8c6ee6bf2e3b3` | ja | `0x02` | 3 | – | 1 |
| `fr.soe.a3s.domain.configration.BikeyExtractOptions` | `0000000000003039` | ja | `0x02` | 2 | – | 1 |
| `fr.soe.a3s.domain.configration.Configuration` | `8cab416e8392729e` | ja | `0x02` | 17 | – | 1 |
| `fr.soe.a3s.domain.configration.ExternalApplication` | `5ceb0379912eb24e` | ja | `0x02` | 4 | – | 1 |
| `fr.soe.a3s.domain.configration.FavoriteServer` | `400581f7552a9bb4` | ja | `0x02` | 7 | – | 5 |
| `fr.soe.a3s.domain.configration.LauncherOptions` | `62b624e100df6213` | ja | `0x02` | 24 | – | 4 |
| `fr.soe.a3s.domain.configration.Proxy` | `a05e3412c919cd4e` | **nein (berechnet)** | `0x02` | 2 | – | 1 |
| `fr.soe.a3s.domain.configration.RptOptions` | `6b6ae634bf2d8da8` | ja | `0x02` | 1 | – | 1 |
| `fr.soe.a3s.domain.configration.TfarOptions` | `81b87db9ecb763cf` | ja | `0x02` | 3 | – | 1 |
| `fr.soe.a3s.domain.repository.AutoConfig` | `258ea3fb4104c73b` | ja | `0x02` | 3 | – | 3 |
| `fr.soe.a3s.domain.repository.Changelog` | `0a65709081a706b7` | ja | `0x02` | 7 | – | 3 |
| `fr.soe.a3s.domain.repository.Changelogs` | `65c3fddfb9c96a24` | ja | `0x02` | 1 | – | 3 |
| `fr.soe.a3s.domain.repository.Event` | `6779d60b73e3157d` | ja | `0x02` | 5 | – | 1 |
| `fr.soe.a3s.domain.repository.Events` | `b8a5364e9d49ae72` | ja | `0x02` | 1 | – | 4 |
| `fr.soe.a3s.domain.repository.Repository` | `8f01baef8859a3fe` | ja | `0x02` | 28 | – | 1 |
| `fr.soe.a3s.domain.repository.ServerInfo` | `6ad21056c345a7f9` | ja | `0x02` | 9 | – | 3 |
| `fr.soe.a3s.domain.repository.SyncTreeDirectory` | `d85feb3c784f59f8` | ja | `0x02` | 7 | – | 3 |
| `fr.soe.a3s.domain.repository.SyncTreeLeaf` | `7aced84d242712d7` | ja | `0x02` | 8 | – | 2 |
| `java.lang.Boolean` | `cd207280d59cfaee` | – | `0x02` | 1 | – | 2 |
| `java.lang.Enum` | `0000000000000000` | – | `0x12` | 0 | – | 7 |
| `java.util.ArrayList` | `7881d21d99c7619d` | – | `0x03` | 1 | – | 17 |
| `java.util.Date` | `686a81014b597419` | – | `0x03` | 0 | – | 6 |
| `java.util.HashMap` | `0507dac1c31660d1` | – | `0x03` | 2 | – | 2 |
| `java.util.HashSet` | `ba44859596b8b734` | – | `0x03` | 0 | – | 4 |
| `java.util.TreeSet` | `dd98509395ed875b` | – | `0x03` | 0 | – | 1 |
| `javax.crypto.SealedObject` | `3e363da6c3b75470` | – | `0x02` | 4 | – | 2 |

Auffälligkeiten:

- **Zwei Klassen ohne explizite `serialVersionUID`** (`domain.AutoConfig`, `domain.configration.Proxy`): ihre UID wird aus der Klassenstruktur berechnet und ändert sich bei Strukturänderungen. Ein Parser darf sie nicht blind festschreiben, sondern muss Feldlisten tolerant behandeln. Betroffen: `export.a3s.autoconfig` und der Proxy-Teil von `a3s.cfg`.
- Zwei verschiedene Klassen heißen `AutoConfig` (Paket `domain` und `domain.repository`) mit unterschiedlichem Inhalt. Neue Rust-Typen müssen sie unterscheiden (z. B. `ConfigurationExport` und `RepositoryAutoConfig`).
- Kleine „handgesetzte“ UIDs (`Http` = −20, `Sftp` = 1, `Socks` = −40, `BikeyExtractOptions` = 12345) zeigen, dass UIDs nicht einheitlich vergeben wurden; sie werden einzeln aus den Daten gelesen.
- Der `parent`-Verweis in `SyncTreeDirectory`/`SyncTreeLeaf`/`TreeDirectory`/`TreeLeaf` erzeugt **zyklische Objektgraphen**. Ein Parser muss Rückverweise auf noch nicht fertig gelesene Objekte unterstützen (Fixtures `sync`, `profile`; zusätzlich `valid/edge/events-self-referencing-list.bin`).
- Collection-Annotationsformate, die der Leser beherrschen muss (alle durch Fixtures abgedeckt): `ArrayList`, `HashMap`, `HashSet`, `TreeSet`, `Date`, Wrapper-Typen und Enums (UID 0, Flag `0x12`).
- Textdaten: Modified UTF-8 mit eingebettetem NUL (`C0 80`), Surrogatpaare (6 Byte) und Strings > 65 535 Bytes (`TC_LONGSTRING`); Fixture `valid/local/profiles/Unicode-Long.a3s.profile`.

## 4. Neutrale Erwartungswerte

Für jede gültige serialisierte Fixture liegt unter `expected/` ein Feld-Dump, erzeugt aus dem **von der echten Baseline wieder eingelesenen** Objekt (bei `.a3s.repository` also nach der Entschlüsselung). Regeln (Kopfkommentar von `FixtureGenerator.Dump`): Schlüssel sortiert, Mengen als `{"_set": […]}` (ungeordnet), Enums als Konstantenname, Daten als ISO-8601 UTC, `parent`-Felder weggelassen, Zyklen als `{"_cycle": true}`, jedes Objekt mit `_type` (einfacher Klassenname).

Dies ist die **Referenz dessen, was ein Parser extrahieren muss** (Altfeldnamen), nicht das spätere neutrale Importschema (`schema_version` laut COMPATIBILITY.md). Schema und Feldumbenennung entscheidet P2 (MIG-020/021). Sensible Felder, die der Export weglassen oder redigieren muss: `password` (alle Protokolltypen und `FavoriteServer`), `login`, URL-Bestandteile mit Zugangsdaten.

`verify.py` dekodiert alle 23 gültigen Serialisierungen ohne JVM und vergleicht sie mit diesen Erwartungswerten. Nur die zwei `SealedObject`-Dateien werden strukturell geprüft (Klasse, `sealAlg`, Blocklänge); ihre Inhalte stehen in den Erwartungsdateien aus dem Java-Lauf.

## 5. zsync und MD4

- Erzeugt mit der Baseline-`MetaFileMaker`: Header (`zsync: jazsync`, `Filename`, `MTime`, `Blocksize: 8192`, `Length`, `Hash-Lengths`, `URL`, `SHA-1`), Leerzeile, danach je Block `rsum_bytes` Weak-Sum-Bytes (Reihenfolge `[b1 b0 b3 b2]` bei 4 Byte, siehe `MetaFileMaker.intToBytes`) und die auf `strongSumLength` gekürzte **MD4**-Prüfsumme. Der letzte Block wird mit Nullen aufgefüllt. `verify.py` berechnet Weak- und Strong-Summen aus der Nutzlast neu und stimmt für alle 7 erzeugbaren Größen (1, 100, 8190, 8192, 8193, 16384, 20000 Byte) überein.
- `MD4` (`jazsync.MD4`) entspricht RFC 1320 für Eingaben < 512 MiB (im Padding steht nur die untere 32-Bit-Länge). `BrokenMD4` überspringt das Padding, wenn die Eingabelänge ein Vielfaches von 64 ist (auch bei leerer Eingabe) – rsync-Altverhalten. `valid/md4-vectors.json` enthält 13 Vektoren beider Varianten (u. a. Längen 0, 55, 56, 63, 64, 65, 128, 8191, 8192, 8193), unabhängig in Python nachgerechnet. `MetaFileMaker` verwendet `MD4`. Wo `BrokenMD4` im Download-Pfad greift, ist **nicht geprüft** (Jazsync-Clientpfad, MIG-041).
- Nicht erzeugt/geprüft: `Z-URL`/`Z-Map2` (`HeaderMaker` kennt die Felder, schreibt sie nicht), `.pbo.zip`-Paare, Dateien > 20 000 Byte, die Download-Seite (`FileMaker`).

## 6. Befunde an der Baseline (belegt, nicht behoben)

Die Baseline bleibt unverändert; die Befunde sind Testfälle für die Rust-Version.

1. **`MetaFileMaker` scheitert bei `size % 8192 == 8191`** (z. B. 8191, 16383): `Generator.generateSums(File)` liest mit negativer Länge (`IndexOutOfBoundsException`), und die Datei-Handles bleiben offen (Windows sperrt die Teildatei bis zur Garbage Collection). Im Manifest als `zsync-generation-failure` für `payload-8191.bin`. Betroffene Dateien erhalten in einem Repository-Build keine `.zsync`-Datei.
2. **GZIP-Trailer wird nicht gelesen:** `ObjectInputStream` endet nach dem Objekt, die CRC32-Prüfung entfällt. `invalid/gzip-bad-crc.bin` wird von der Baseline **akzeptiert** (Ergebnis `Profile`). Neuer Parser: bis EOF lesen und Trailer prüfen.
3. **Array-Länge ungeprüft:** `invalid/array-declared-1gib.bin` (47 Byte) löst in der Baseline `OutOfMemoryError` aus (gemessen mit `-Xmx768m`); der Filter wertet `info.arrayLength()` nicht aus. Neuer Parser: Längen vor der Allokation gegen Rest-Streamgröße und Limit prüfen.
4. **Dateihandle-Leck beim Ablehnen:** `A3SFilesAccessor.read` schließt den `FileInputStream` nicht, wenn `GZIPInputStream` den Header ablehnt (leere/Nicht-GZIP-Datei); unter Windows bleibt die Datei gesperrt.
5. **Zwei Klassen ohne explizite UID** (Abschnitt 3).

Alle übrigen ungültigen Fixtures lehnt die Baseline ab (Ergebnis je Datei in `manifest.json`, `baseline_reader_outcome`). `verify.py` lehnt alle 15 ungültigen Serialisierungs-/GZIP-Fixtures unter expliziten Grenzen ab (8 MiB entpackt, Tiefe 100, 250 000 Handles, Array ≤ 1 000 000, Deskriptortabelle). `invalid/zip-traversal.zip` wird nur als Fixture bereitgestellt; die Extraktionslogik entsteht in P3/P4.

## 7. Exportstrategie (Spezifikation, nicht implementiert)

Gilt zusätzlich zu COMPATIBILITY.md:

1. **Server-Metadaten (`.a3s/*`)** und alle unverschlüsselten lokalen Dateien lassen sich ohne JVM mit einem begrenzten Leser lesen – belegt durch `verify.py` (Leserteil rund 290 Zeilen, nur Standardbibliothek). Empfehlung: direkter Rust-Parser in `armasync-compat`, keine JVM-Exporthilfe für diese Typen.
2. **`.a3s.repository` (verschlüsselt)** ist der einzige Fall, der einen Schlüssel erfordert. Offen für ADR-0003/MIG-021: (a) Rust implementiert die AES-Entschlüsselung mit dem aus dem Altquelltext stammenden Schlüssel als dokumentierte Kompatibilitätskonstante (Zugangsdaten gelangen **nicht** in den neutralen Export) oder (b) ein einmaliger JVM-Offline-Export. Beide lesen dieselben Strukturen.
3. **Redaktion:** Der Export enthält weder `password` noch `login` oder URLs mit Benutzerinfo; stattdessen `CredentialRef` bzw. Neuanmeldung.
4. **Pfade/Namen:** Der Parser liefert Namen unverändert (Fixture `valid/server/hostile-paths`: `..`, `../x`, `C:\…`, UNC, `CON`, `x:stream`, NUL, Prozent-Kodierung, Case- und NFC/NFD-Kollisionen). Die Pfadvalidierung ist eine **eigene, nachgeschaltete Schicht**; keine stille Zusammenführung.
5. **Limits** (Startwerte aus Baseline und Fixtures, in P2 an großen echten Strukturen zu validieren): entpackt ≤ 256 MiB, Tiefe ≤ 100, Handles ≤ 250 000, Array-/Stringlängen gegen Rest-Streamgröße, Laufzeitbudget, GZIP vollständig bis EOF.
6. **Schreiben** ist nicht Teil von MIG-001 (Schreibentscheidung in ADR-0003, P4).

## 8. Nicht geprüft / offen

- Keine großen echten Repositories (> 10 000 Dateien) und keine realen Nutzerdateien (bewusst); Größenverhalten der Limits ist unvalidiert.
- Nur JDK 25.0.2 unter Windows ausgeführt. GZIP-Bytes können mit anderem JDK/zlib abweichen; `payload_sha256` ist die stabile Identität.
- `Properties`, `LinkedHashSet`, `TreeMap`, `Map$Entry` stehen auf der Filter-Allowlist, kommen in den Baseline-Klassen aber in keinem gespeicherten Feld vor; der Python-Leser kennt `TreeMap`/`LinkedHashSet`, ohne Fixture.
- Ältere Dateiversionen mit entfernten Feldern/Klassen (Klassenevolution zwischen Releases): keine historischen Dateien vorhanden; `Profile`/`Configuration` enthalten als veraltet markierte Felder, die ältere Dateien füllen können.
- Launcher-Preset-HTML (`docs/ARMA3_LAUNCHER_PRESET_EXPORT.md`) → P5 (MIG-050).
- Rechtebasis der Jazsync-/Jarsync-Algorithmen → MIG-002.
