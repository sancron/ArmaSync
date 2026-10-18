#!/usr/bin/env python3
"""Loopback-only deterministic HTTP/WebDAV fixture server for MIG-003."""

from __future__ import annotations

import argparse
import base64
import hashlib
import http.server
import threading
import urllib.parse
import xml.etree.ElementTree as ET

SAMPLE = b"ArmaSync MIG-003 transport fixture\n" + bytes(range(256)) * 4
DAV_NS = "DAV:"


class State:
    def __init__(self) -> None:
        self.lock = threading.RLock()
        self.files = {"/files/sample.bin": SAMPLE}
        self.directories = {"/", "/files"}


class Handler(http.server.BaseHTTPRequestHandler):
    server_version = "ArmaSync-MIG003/1.0"
    protocol_version = "HTTP/1.1"

    @property
    def state(self) -> State:
        return self.server.state  # type: ignore[attr-defined]

    def log_message(self, fmt: str, *args: object) -> None:
        print("%s - %s" % (self.client_address[0], fmt % args), flush=True)

    def _route(self) -> str | None:
        """Decode the request path exactly once, normalise it and authenticate on that same form.

        Returns the storage path (``/auth`` prefix stripped), or ``None`` after a 400/401 response was sent.
        Routing and authentication share this single representation, so percent-encoded variants of
        ``/auth/`` cannot bypass the check.
        """
        decoded = urllib.parse.unquote(urllib.parse.urlsplit(self.path).path)
        segments = decoded.split("/")
        unsafe = (
            not decoded.startswith("/")
            or "\\" in decoded
            or "\x00" in decoded
            or any(part in (".", "..") for part in segments)
            or any(part == "" for part in segments[1:-1])  # no empty segments such as "//" inside the path
        )
        if unsafe:
            self._reject(400)
            return None
        path = decoded.rstrip("/") or "/"
        if path == "/auth" or path.startswith("/auth/"):
            expected = "Basic " + base64.b64encode(b"fixture-user:fixture-pass").decode("ascii")
            if self.headers.get("Authorization") != expected:
                self._reject(401, {"WWW-Authenticate": 'Basic realm="ArmaSync MIG-003 fixture"'})
                return None
            path = path[len("/auth") :] or "/"
        return path

    def _reject(self, status: int, headers: dict[str, str] | None = None) -> None:
        # The request body (if any) is not read on rejection; close so it cannot be parsed as the next request.
        self.close_connection = True
        self._send(status, headers={**(headers or {}), "Connection": "close"})

    def _send(self, status: int, body: bytes = b"", headers: dict[str, str] | None = None) -> None:
        self.send_response(status)
        for key, value in (headers or {}).items():
            self.send_header(key, value)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if self.command != "HEAD" and body:
            self.wfile.write(body)

    def do_GET(self) -> None:
        path = self._route()
        if path is None:
            return
        if path == "/health":
            self._send(200, b"ok\n", {"Content-Type": "text/plain"})
            return
        with self.state.lock:
            body = self.state.files.get(path)
        if body is None:
            self._send(404)
            return
        etag = '"' + hashlib.sha256(body).hexdigest() + '"'
        headers = {"Content-Type": "application/octet-stream", "Accept-Ranges": "bytes", "ETag": etag}
        range_value = self.headers.get("Range")
        if range_value:
            if not range_value.startswith("bytes=") or "," in range_value:
                self._send(416, headers={"Content-Range": f"bytes */{len(body)}"})
                return
            try:
                start_text, end_text = range_value[6:].split("-", 1)
                start = int(start_text)
                end = min(int(end_text), len(body) - 1) if end_text else len(body) - 1
            except (ValueError, TypeError):
                self._send(416, headers={"Content-Range": f"bytes */{len(body)}"})
                return
            if start < 0 or end < start or start >= len(body):
                self._send(416, headers={"Content-Range": f"bytes */{len(body)}"})
                return
            part = body[start : end + 1]
            headers["Content-Range"] = f"bytes {start}-{start + len(part) - 1}/{len(body)}"
            self._send(206, part, headers)
            return
        self._send(200, body, headers)

    def do_HEAD(self) -> None:
        self.do_GET()

    def do_OPTIONS(self) -> None:
        self._send(200, headers={"Allow": "GET, HEAD, OPTIONS, PROPFIND, PUT, DELETE, MKCOL"})

    def do_PROPFIND(self) -> None:
        path = self._route()
        if path is None:
            return
        with self.state.lock:
            exists = path in self.state.directories or path in self.state.files
        if not exists:
            self._send(404)
            return
        root = ET.Element(f"{{{DAV_NS}}}multistatus")
        response = ET.SubElement(root, f"{{{DAV_NS}}}response")
        ET.SubElement(response, f"{{{DAV_NS}}}href").text = path
        propstat = ET.SubElement(response, f"{{{DAV_NS}}}propstat")
        prop = ET.SubElement(propstat, f"{{{DAV_NS}}}prop")
        ET.SubElement(prop, f"{{{DAV_NS}}}displayname").text = path.rsplit("/", 1)[-1] or "root"
        resource = ET.SubElement(prop, f"{{{DAV_NS}}}resourcetype")
        with self.state.lock:
            is_dir = path in self.state.directories
            size = len(self.state.files.get(path, b""))
        if is_dir:
            ET.SubElement(resource, f"{{{DAV_NS}}}collection")
        else:
            ET.SubElement(prop, f"{{{DAV_NS}}}getcontentlength").text = str(size)
        ET.SubElement(propstat, f"{{{DAV_NS}}}status").text = "HTTP/1.1 200 OK"
        self._send(207, ET.tostring(root, encoding="utf-8", xml_declaration=True), {"Content-Type": "application/xml; charset=utf-8"})

    def do_MKCOL(self) -> None:
        path = self._route()
        if path is None:
            return
        if path == "/":
            self._reject(400)
            return
        parent = path.rsplit("/", 1)[0] or "/"
        with self.state.lock:
            if parent not in self.state.directories:
                self._send(409)
                return
            if path in self.state.directories or path in self.state.files:
                self._send(405)
                return
            self.state.directories.add(path)
        self._send(201)

    def do_PUT(self) -> None:
        path = self._route()
        if path is None:
            return
        if path == "/":
            self._reject(400)
            return
        parent = path.rsplit("/", 1)[0] or "/"
        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            self._send(400)
            return
        if length < 0 or length > 8 * 1024 * 1024:
            self._reject(413)
            return
        body = self.rfile.read(length)
        with self.state.lock:
            if parent not in self.state.directories:
                self._send(409)
                return
            self.state.files[path] = body
        self._send(201)

    def do_DELETE(self) -> None:
        path = self._route()
        if path is None:
            return
        if path == "/":
            self._reject(400)
            return
        with self.state.lock:
            if path in self.state.files:
                del self.state.files[path]
            elif path in self.state.directories:
                if any(p.startswith(path + "/") for p in (*self.state.files, *self.state.directories)):
                    self._send(409)
                    return
                self.state.directories.remove(path)
            else:
                self._send(404)
                return
        self._send(204)


class Server(http.server.ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True

    def __init__(self, address: tuple[str, int]):
        super().__init__(address, Handler)
        self.state = State()


def self_test() -> None:
    import http.client

    server = Server(("127.0.0.1", 0))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    host, port = server.server_address

    def request(method: str, path: str, body: bytes | None = None, headers: dict[str, str] | None = None):
        conn = http.client.HTTPConnection(host, port, timeout=3)
        conn.request(method, path, body=body, headers=headers or {})
        response = conn.getresponse()
        result = response.status, response.read(), dict(response.getheaders())
        conn.close()
        return result

    checks = 0

    def check(condition: bool, label: str = "") -> None:
        nonlocal checks
        assert condition, label
        checks += 1

    auth = {"Authorization": "Basic " + base64.b64encode(b"fixture-user:fixture-pass").decode("ascii")}
    try:
        check(request("GET", "/health")[0] == 200)
        status, part, headers = request("GET", "/files/sample.bin", headers={"Range": "bytes=5-12"})
        check(status == 206 and part == SAMPLE[5:13] and headers["Content-Range"].startswith("bytes 5-12/"))
        check(request("GET", "/files/sample.bin", headers={"Range": "bytes=99999-"})[0] == 416)
        check(request("GET", "/auth/files/sample.bin")[0] == 401)
        check(request("GET", "/auth/files/sample.bin", headers=auth)[0] == 200)
        check(request("PROPFIND", "/files/sample.bin")[0] == 207)
        check(request("MKCOL", "/files/upload")[0] == 201)
        check(request("PUT", "/files/upload/result.bin", b"test-upload")[0] == 201)
        check(request("GET", "/files/upload/result.bin")[1] == b"test-upload")
        check(request("DELETE", "/files/upload/result.bin")[0] == 204)
        check(request("GET", "/files/%2e%2e/outside")[0] == 400)
        # Regression: percent-encoded variants of /auth/ must not bypass authentication (routing and auth share
        # one decoded path); unauthenticated requests must not create or modify anything.
        for encoded in ("/%61uth/files/sample.bin", "/auth%2Ffiles/sample.bin", "/%61%75th/files/sample.bin"):
            check(request("GET", encoded)[0] == 401, encoded)
            check(request("GET", encoded, headers=auth)[0] == 200, encoded)
        check(request("GET", "/auth%2f..%2ffiles/sample.bin", headers=auth)[0] == 400)
        check(request("GET", "/auth//files/sample.bin", headers=auth)[0] == 400)
        check(request("PUT", "/%61uth/files/evil.bin", b"x")[0] == 401)
        check(request("PUT", "/auth%2Ffiles/evil.bin", b"x")[0] == 401)
        check(request("GET", "/files/evil.bin")[0] == 404)
        check(request("PUT", "/%61uth/files/sample.bin", b"overwrite")[0] == 401)
        check(request("DELETE", "/%61uth/files/sample.bin")[0] == 401)
        check(request("MKCOL", "/%61uth/files/newdir")[0] == 401)
        check(request("PROPFIND", "/%61uth/files/sample.bin")[0] == 401)
        check(request("GET", "/files/sample.bin")[1] == SAMPLE)
        check(request("PROPFIND", "/files/newdir")[0] == 404)
        check(request("PUT", "/%61uth/files/ok.bin", b"authed", auth)[0] == 201)
        check(request("GET", "/files/ok.bin")[1] == b"authed")
        check(request("DELETE", "/%61uth/files/ok.bin", headers=auth)[0] == 204)
        print(f"self-test: {checks} checks passed (HTTP range/auth + WebDAV + path guard + encoded-auth regression)")
    finally:
        server.shutdown()
        server.server_close()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", default="127.0.0.1", help="loopback address only (default: 127.0.0.1)")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if args.host != "127.0.0.1":
        parser.error("refusing non-loopback or ambiguous bind; allowed: 127.0.0.1")
    server = Server((args.host, args.port))
    print(f"MIG-003 fixture server listening on {args.host}:{server.server_address[1]} (loopback only)", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("stopping", flush=True)
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
