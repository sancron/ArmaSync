# MIG-003 loopback transport fixture

This dependency-free Python 3 server provides deterministic HTTP and a small WebDAV subset for migration experiments. It binds only to `127.0.0.1` by default and refuses non-loopback bind addresses. All file content is held in memory; it does not read, overwrite, or delete repository files. Its Basic Auth material (`fixture-user` / `fixture-pass`) is synthetic test data and must never be reused outside local tests.

## Run

Windows PowerShell:

```powershell
./migration-tools/transport-testserver/run.ps1
```

Linux:

```sh
sh migration-tools/transport-testserver/run.sh
```

The optional first argument changes the port (default `8765`). `Ctrl+C` stops the process. `GET /health` is a readiness check. `GET /files/sample.bin` serves a stable binary body, ETag, and single byte ranges. `/auth/files/sample.bin` requires the synthetic Basic Auth credentials. The WebDAV subset implements `OPTIONS`, `PROPFIND`, `MKCOL`, `PUT`, and `DELETE` against an in-memory tree.

Run the server's deterministic protocol and safety checks with `python3 migration-tools/transport-testserver/server.py --self-test` (on Windows, use `py -3 ...`). It starts an ephemeral loopback listener, runs 31 checks, then closes it. The suite checks byte ranges, 416 handling, auth, WebDAV operations, traversal rejection and a regression set for percent-encoded `/auth/` variants (`/%61uth/…`, `/auth%2F…`): they must return 401 without credentials, leave the in-memory state untouched (GET/PUT/DELETE/MKCOL/PROPFIND) and still work with the synthetic credentials. It is a fixture self-test, not proof that every legacy Java DAO interoperates with it.

This harness does not implement FTP/FTPS or SSH/SFTP, and it does not create TLS certificates or SSH host keys. Those protocols require protocol-native servers and certificate/host-key fixtures; the matrix records their current evidence and remaining interoperability work. No TLS/SSH validation is disabled by this tool.

Routing and authentication use one single-decoded, normalised request path. Paths containing `.`/`..` segments, empty inner segments, backslashes or NUL are answered with 400 and the connection is closed. Everything at or below `/auth` requires the synthetic Basic credentials; unauthenticated writes never change the in-memory tree.
