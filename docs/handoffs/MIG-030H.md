# Handoff MIG-030H

- Status: reviewbereit; Claude; 2026-10-19; Basis `e0589dc`. Bericht: [MIG-030H](../changes/2026-10-19-MIG-030H.md); Analyse [LOCAL_FS_THREATS](../migration/LOCAL_FS_THREATS.md).
- Offen: Linux/WSL (u. a. `flock`-Verhalten; Unix-Race = Release-Blocker für „race-freie Pfadgrenze“), GitHub-CI pausiert.
- Nächster Schritt: MIG-030D (CLI `sync plan`/`sync run`; `open_exclusive` und `plan_options` verwenden).
