# Rust-Toolchain, MSRV und Abhängigkeiten (MIG-010)

Stand: 2026-10-18. Gepflegt vom Eigentümer des Workspace (Integrator); Änderungen über einen Task.

## Pins und Regeln

| Gegenstand | Wert | Beleg / Regel |
| --- | --- | --- |
| Toolchain | exakt `1.99.1` (`rust-toolchain.toml`, Profil `minimal`, Komponenten `rustfmt`, `clippy`) | Verifiziert: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1`, Windows `x86_64-pc-windows-msvc`; Linux (WSL2 Ubuntu) `x86_64-unknown-linux-gnu`. `rustup` wählt die Version im Repo automatisch. Aktualisierung nur bewusst per Task. |
| MSRV | `rust-version = "1.85"` (`[workspace.package]`) | Untergrenze durch Edition 2024 (ab 1.85) und `clap` 4.6 (`rust-version` 1.85) bestimmt. Geprüft: `cargo +1.85.0 test --workspace --locked` auf Windows bestanden. MSRV nur mit Begründung und erneuter Prüfung anheben. |
| Edition / Resolver | Edition 2024, `resolver = "3"` | Resolver 3 wählt Versionen kompatibel zur MSRV; `Cargo.lock` wurde so erzeugt („highest Rust 1.85 compatible versions“). |
| Lockdatei | `Cargo.lock` eingecheckt; alle Prüfungen mit `--locked` | Anwendung, nicht Bibliothek (AGENTS.md). Pflege ausschließlich durch den Integrator. |
| Lizenz | `AGPL-3.0-only` (`license` im Workspace, SPDX-Header in allen Quelldateien) | Neue, unabhängig geschriebene Quellen; keine Bestandsquellen übernommen. |
| Lints | `unsafe_code = forbid`, Clippy `all` + `unwrap_used`/`expect_used` als Warnung (in CI/Prüfbefehl als Fehler) | Tests dürfen `expect` einsetzen (lokales `allow`). |
| Zielplattformen | Windows x86_64 MSVC, Linux x86_64 GNU | ARM/macOS sind keine Zusage (ARCHITECTURE). |

## Struktur

```text
crates/armasync-core         Produktidentität (Namen, technische ID, Version); später Modelle, Regeln, Ports
crates/armasync-application  Use-Case-Schicht; heute nur read-only `version_info()`; hängt nur von core ab
crates/armasync-adapters     `Platform` (cfg target_os); später FS/Netz/OS-Adapter; hängt nur von core ab
crates/armasync-cli          Binary `armasync` (+ Lib `armasync_cli`): clap, JSON-Envelope, Exitcodes
```

Abhängigkeitsrichtung: `cli → application → core`, `cli → adapters → core`; keine Zyklen. Es gibt **kein GUI-Crate**: `armasync-gui` entsteht mit MIG-012 nach der Toolkit-Entscheidung (ADR-0002), um kein leeres Gerüst zu schaffen. Der CLI-Baum enthält nachweislich keine GUI-/Windowing-Crates (siehe unten).

## CLI-Basis (Umfang MIG-010)

`armasync version [--json] [--quiet]`, `--version`, `--help`, globale Flags `--json`, `--no-input`, `--quiet` (auch nach dem Subcommand). Strukturiertes Ergebnis auf stdout, Fehler/Usage auf stderr, Exitcodes nach [CLI_CONTRACT](CLI_CONTRACT.md) (0, 2, 3, 4, 5, 6, 7, 8, 130 als Typ definiert; benutzt werden derzeit 0, 2 und 5 bei Schreibfehler). JSON-Envelope `schema_version: 1`, `operation_id: null` (echte Operation-IDs kommen mit MIG-011). Keine Synchronisations-, Repository- oder Importbefehle; es wird keine Funktion behauptet, die nicht existiert.

## Abhängigkeitsinventar (extern, aus `Cargo.lock`)

Direkt deklariert: `clap` 4 (Features `std derive help usage error-context suggestions`; ohne `color`, `env`, `wrap_help`) und `serde_json` 1 – beide nur in `armasync-cli`. Übrige sind transitive Pakete. Lizenzangaben stammen aus den Crate-Metadaten (`cargo metadata`); **sie sind kein Rechtsgutachten**. Die Release-BOM mit Lizenztexten/Notices ist noch zu erzeugen (offen: RQ-06).

| Crate | Version | Lizenz (Metadaten) | rust-version | Zweck |
| --- | --- | --- | --- | --- |
| clap | 4.6.7 | MIT OR Apache-2.0 | 1.85 | Argumentparser (direkt) |
| clap_builder | 4.6.7 | MIT OR Apache-2.0 | 1.85 | clap-Kern |
| clap_derive | 4.6.7 | MIT OR Apache-2.0 | 1.85 | Derive-Makro (Build-Zeit) |
| clap_lex | 1.1.1 | MIT OR Apache-2.0 | 1.85 | Argument-Lexer |
| anstyle | 1.0.14 | MIT OR Apache-2.0 | 1.66.0 | Stil-Typen für Hilfe/Fehler |
| strsim | 0.11.1 | MIT | 1.56 | Vorschläge bei Tippfehlern |
| heck | 0.5.0 | MIT OR Apache-2.0 | 1.56 | Namenskonvertierung (Derive) |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | 1.71 | Makro-Infrastruktur (Build-Zeit) |
| quote | 1.0.47 | MIT OR Apache-2.0 | 1.71 | Makro-Infrastruktur (Build-Zeit) |
| syn | 3.0.9 | MIT OR Apache-2.0 | 1.71 | Makro-Parser (Build-Zeit) |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | 1.71 | Bezeichnerregeln (Build-Zeit) |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | 1.71 | JSON-Ausgabe (direkt) |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | 1.56 | Serde-Kerntypen |
| serde / serde_derive | 1.0.229 | MIT OR Apache-2.0 | 1.56 / 1.71 | im Lockfile (Feature-Auflösung), nicht im normalen Laufzeitbaum |
| itoa | 1.0.18 | MIT OR Apache-2.0 | 1.68 | Zahlenformatierung |
| memchr | 2.8.3 | Unlicense OR MIT | 1.61 | Bytesuche |
| zmij | 1.0.23 | MIT | 1.71 | Gleitkomma-Formatierung (serde_json) |

Keine dieser Crates stammt aus dem Java-Bestand; es wurden **kein** Jazsync-/MD4-Code, **keine** Icons/Installergrafiken/JARs übernommen (Auflagen RQ-02/03/04 eingehalten).

## Prüfbefehle (Repository-Root)

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build -p armasync-cli --locked
cargo tree -p armasync-cli --locked -e normal --prefix none      # keine GUI-/Windowing-Crates
cargo +1.85.0 test --workspace --locked                          # MSRV (Toolchain 1.85.0 installiert)
```

Auf Linux zusätzlich in einem LF-Checkout (nicht auf NTFS-Arbeitskopie über `/mnt`) ausführen.
