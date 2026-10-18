# SPDX-License-Identifier: AGPL-3.0-only
"""MIG-001 independent verifier for fixtures/legacy-formats.

Runs without a JVM and without any baseline class. It contains a small, bounded reader for the
GZIP + Java object serialization wire format, written from the public stream grammar, and checks:

  1. manifest sha256 / payload_sha256 of every file,
  2. every valid serialized fixture decodes to exactly the expected neutral field dump produced
     from the baseline reader (sealed repository files: structure only, see FORMAT_INVENTORY.md),
  3. zsync metafiles against block checksums recomputed here, MD4/BrokenMD4 against vectors,
  4. the invalid fixtures are rejected by this reader under explicit limits,
  5. descriptor-inventory.json (derived) matches and agrees with class-inventory.json.

Usage: python -I verify.py <fixtureDir> [--write-inventory]
"""
import gzip
import hashlib
import json
import os
import struct
import sys
import zlib

# ----------------------------------------------------------------------------- limits

MAX_DECOMPRESSED = 8 * 1024 * 1024  # fixtures are tiny; real limits are a P2 decision
MAX_DEPTH = 100
MAX_HANDLES = 250_000
MAX_ARRAY = 1_000_000


class FormatError(Exception):
    pass


class LimitError(FormatError):
    pass


# ----------------------------------------------------------------------------- stream reader

TC_NULL, TC_REFERENCE, TC_CLASSDESC, TC_OBJECT, TC_STRING, TC_ARRAY, TC_CLASS = 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76
TC_BLOCKDATA, TC_ENDBLOCKDATA, TC_RESET, TC_BLOCKDATALONG, TC_EXCEPTION = 0x77, 0x78, 0x79, 0x7A, 0x7B
TC_LONGSTRING, TC_PROXYCLASSDESC, TC_ENUM = 0x7C, 0x7D, 0x7E
SC_WRITE_METHOD, SC_SERIALIZABLE, SC_EXTERNALIZABLE, SC_BLOCK_DATA, SC_ENUM = 0x01, 0x02, 0x04, 0x08, 0x10
BASE_HANDLE = 0x7E0000


def decode_mutf8(raw):
    units = []
    i = 0
    n = len(raw)
    while i < n:
        b = raw[i]
        if b < 0x80:
            units.append(b)
            i += 1
        elif b >> 5 == 0b110:
            if i + 1 >= n or raw[i + 1] >> 6 != 0b10:
                raise FormatError("bad modified UTF-8")
            units.append(((b & 0x1F) << 6) | (raw[i + 1] & 0x3F))
            i += 2
        elif b >> 4 == 0b1110:
            if i + 2 >= n or raw[i + 1] >> 6 != 0b10 or raw[i + 2] >> 6 != 0b10:
                raise FormatError("bad modified UTF-8")
            units.append(((b & 0x0F) << 12) | ((raw[i + 1] & 0x3F) << 6) | (raw[i + 2] & 0x3F))
            i += 3
        else:
            raise FormatError("bad modified UTF-8 lead byte")
    return struct.pack(">%dH" % len(units), *units).decode("utf-16-be", "surrogatepass")


class ClassDesc:
    def __init__(self):
        self.name = None
        self.suid = None
        self.flags = 0
        self.fields = []  # (typecode, name, classname-or-None)
        self.sup = None


class JObj:
    def __init__(self, desc):
        self.desc = desc
        self.fields = {}  # class name -> {field: value}
        self.annot = {}  # class name -> list of annotation items
        self.enum_name = None


class Reader:
    def __init__(self, data):
        self.d = data
        self.p = 0
        self.handles = []
        self.depth = 0
        if self._u2() != 0xACED:
            raise FormatError("bad stream magic")
        if self._u2() != 5:
            raise FormatError("unsupported stream version")

    # primitives
    def _take(self, n):
        if n < 0 or self.p + n > len(self.d):
            raise FormatError("unexpected end of stream")
        v = self.d[self.p:self.p + n]
        self.p += n
        return v

    def _u1(self):
        return self._take(1)[0]

    def _u2(self):
        return struct.unpack(">H", self._take(2))[0]

    def _i4(self):
        return struct.unpack(">i", self._take(4))[0]

    def _i8(self):
        return struct.unpack(">q", self._take(8))[0]

    def _utf(self):
        return decode_mutf8(self._take(self._u2()))

    def _handle(self, obj):
        if len(self.handles) >= MAX_HANDLES:
            raise LimitError("too many handles")
        self.handles.append(obj)
        return len(self.handles) - 1

    def at_end(self):
        return self.p == len(self.d)

    # grammar
    def read_content(self):
        tc = self._u1()
        return self._by_tc(tc)

    def _by_tc(self, tc):
        self.depth += 1
        if self.depth > MAX_DEPTH:
            raise LimitError("nesting too deep")
        try:
            if tc == TC_NULL:
                return None
            if tc == TC_REFERENCE:
                h = self._i4() - BASE_HANDLE
                if h < 0 or h >= len(self.handles):
                    raise FormatError("invalid back reference")
                return self.handles[h]
            if tc == TC_STRING:
                s = self._utf()
                self._handle(s)
                return s
            if tc == TC_LONGSTRING:
                n = self._i8()
                if n < 0 or n > MAX_DECOMPRESSED:
                    raise LimitError("string too long")
                s = decode_mutf8(self._take(n))
                self._handle(s)
                return s
            if tc == TC_OBJECT:
                return self._object()
            if tc == TC_ENUM:
                desc = self._classdesc()
                o = JObj(desc)
                self._handle(o)
                name = self.read_content()
                o.enum_name = name
                return o
            if tc == TC_ARRAY:
                return self._array()
            if tc in (TC_CLASSDESC, TC_PROXYCLASSDESC):
                return self._classdesc(tc)
            if tc == TC_CLASS:
                desc = self._classdesc()
                self._handle(desc)
                return desc
            raise FormatError("unsupported type code 0x%02x" % tc)
        finally:
            self.depth -= 1

    def _classdesc(self, tc=None):
        if tc is None:
            tc = self._u1()
        if tc == TC_NULL:
            return None
        if tc == TC_REFERENCE:
            h = self._i4() - BASE_HANDLE
            if h < 0 or h >= len(self.handles) or not isinstance(self.handles[h], ClassDesc):
                raise FormatError("invalid class descriptor reference")
            return self.handles[h]
        if tc == TC_PROXYCLASSDESC:
            raise FormatError("proxy classes are not part of the legacy formats")
        if tc != TC_CLASSDESC:
            raise FormatError("expected class descriptor")
        d = ClassDesc()
        d.name = self._utf()
        d.suid = self._i8()
        self._handle(d)
        d.flags = self._u1()
        if d.flags & SC_EXTERNALIZABLE:
            raise FormatError("externalizable classes are not part of the legacy formats")
        n = self._u2()
        for _ in range(n):
            code = chr(self._u1())
            name = self._utf()
            cls = None
            if code in "L[":
                cls = self.read_content()
                if not isinstance(cls, str):
                    raise FormatError("field type must be a string")
            d.fields.append((code, name, cls))
        # class annotation
        while True:
            tc2 = self._u1()
            if tc2 == TC_ENDBLOCKDATA:
                break
            self._skip_annotation_item(tc2)
        d.sup = self._classdesc()
        return d

    def _skip_annotation_item(self, tc):
        if tc == TC_BLOCKDATA:
            self._take(self._u1())
        elif tc == TC_BLOCKDATALONG:
            self._take(self._i4())
        else:
            self._by_tc(tc)

    def _prim(self, code):
        if code == "B":
            return struct.unpack(">b", self._take(1))[0]
        if code == "C":
            return chr(self._u2())
        if code == "D":
            return struct.unpack(">d", self._take(8))[0]
        if code == "F":
            return struct.unpack(">f", self._take(4))[0]
        if code == "I":
            return self._i4()
        if code == "J":
            return self._i8()
        if code == "S":
            return struct.unpack(">h", self._take(2))[0]
        if code == "Z":
            return self._u1() != 0
        raise FormatError("bad primitive code")

    def _object(self):
        desc = self._classdesc()
        if desc is None:
            raise FormatError("object without class descriptor")
        o = JObj(desc)
        self._handle(o)
        chain = []
        c = desc
        while c is not None:
            chain.append(c)
            c = c.sup
        for c in reversed(chain):
            if not c.flags & SC_SERIALIZABLE:
                continue
            vals = {}
            for code, name, _ in c.fields:
                if code in "L[":
                    vals[name] = self.read_content()
                else:
                    vals[name] = self._prim(code)
            o.fields[c.name] = vals
            if c.flags & SC_WRITE_METHOD:
                items = []
                while True:
                    tc = self._u1()
                    if tc == TC_ENDBLOCKDATA:
                        break
                    if tc == TC_BLOCKDATA:
                        items.append(("block", self._take(self._u1())))
                    elif tc == TC_BLOCKDATALONG:
                        items.append(("block", self._take(self._i4())))
                    else:
                        items.append(("obj", self._by_tc(tc)))
                o.annot[c.name] = items
        return o

    def _array(self):
        desc = self._classdesc()
        n = self._i4()
        if n < 0 or n > MAX_ARRAY:
            raise LimitError("array length %d outside limits" % n)
        arr = []
        self._handle(arr)
        comp = desc.name[1]
        if comp in "BCDFIJSZ":
            size = {"B": 1, "C": 2, "D": 8, "F": 4, "I": 4, "J": 8, "S": 2, "Z": 1}[comp]
            if self.p + n * size > len(self.d):
                raise FormatError("unexpected end of stream")
            for _ in range(n):
                arr.append(self._prim(comp))
        else:
            for _ in range(n):
                arr.append(self.read_content())
        return arr


def read_serialization(raw):
    r = Reader(raw)
    top = r.read_content()
    if not r.at_end():
        raise FormatError("trailing bytes after top-level object")
    return top, r


def gunzip_bounded(raw):
    d = zlib.decompressobj(31)
    try:
        out = d.decompress(raw, MAX_DECOMPRESSED + 1)
        if len(out) > MAX_DECOMPRESSED:
            raise LimitError("decompressed size exceeds limit")
        if d.unconsumed_tail:
            raise LimitError("decompressed size exceeds limit")
        if not d.eof:
            raise FormatError("truncated GZIP stream")
    except zlib.error as e:
        raise FormatError("GZIP error: %s" % e)
    if d.unused_data:
        raise FormatError("trailing bytes after GZIP member")
    return out


# ----------------------------------------------------------------------------- neutral dump (mirrors FixtureGenerator.Dump)

def simple_name(cls):
    return cls.rsplit(".", 1)[-1].rsplit("$", 1)[-1]


def iso(ms):
    import datetime
    t = datetime.datetime(1970, 1, 1, tzinfo=datetime.timezone.utc) + datetime.timedelta(milliseconds=ms)
    return t.strftime("%Y-%m-%dT%H:%M:%S.") + "%03dZ" % (ms % 1000)


def blocks(items):
    return b"".join(x for k, x in items if k == "block")


def objs(items):
    return [x for k, x in items if k == "obj"]


WRAPPERS = ("java.lang.Boolean", "java.lang.Byte", "java.lang.Short", "java.lang.Integer", "java.lang.Long", "java.lang.Float", "java.lang.Double", "java.lang.Character")


def neutral(o, path=None):
    path = path or []
    if o is None or isinstance(o, (str, bool, int, float)):
        return o
    if isinstance(o, list):
        return [neutral(x, path) for x in o]
    name = o.desc.name
    if o.desc.flags & SC_ENUM:
        return o.enum_name
    if any(o is p for p in path):
        return {"_cycle": True}
    path = path + [o]
    if name in WRAPPERS:
        return o.fields[name]["value"]
    if name == "java.util.Date":
        (ms,) = struct.unpack(">q", blocks(o.annot[name]))
        return iso(ms)
    if name == "java.util.ArrayList":
        items = objs(o.annot[name])
        return [neutral(x, path) for x in items]
    if name in ("java.util.HashMap", "java.util.LinkedHashMap", "java.util.TreeMap"):
        base = "java.util.HashMap" if name != "java.util.TreeMap" else name
        items = objs(o.annot[base])
        out = {}
        for k, v in zip(items[0::2], items[1::2]):
            out[str(neutral(k, path))] = neutral(v, path)
        return out
    if name in ("java.util.HashSet", "java.util.LinkedHashSet", "java.util.TreeSet"):
        base = name if name != "java.util.LinkedHashSet" else "java.util.HashSet"
        items = objs(o.annot[base])
        if name == "java.util.TreeSet":
            items = items[1:]  # comparator slot
        return {"_set": [neutral(x, path) for x in items]}
    # application class: all serializable fields along the chain, "parent" omitted
    out = {"_type": simple_name(name)}
    for cname, vals in o.fields.items():
        for k, v in vals.items():
            if k == "parent":
                continue
            out[k] = neutral(v, path)
    return out


def canon(v):
    """Canonical JSON text; _set arrays compared as multisets."""
    def norm(x):
        if isinstance(x, dict):
            if set(x) == {"_set"}:
                return {"_set": sorted(json.dumps(norm(i), sort_keys=True) for i in x["_set"])}
            return {k: norm(i) for k, i in x.items()}
        if isinstance(x, list):
            return [norm(i) for i in x]
        return x
    return json.dumps(norm(v), sort_keys=True, ensure_ascii=True)


# ----------------------------------------------------------------------------- MD4 / zsync

def _md4(data, broken):
    def rol(x, s):
        x &= 0xFFFFFFFF
        return ((x << s) | (x >> (32 - s))) & 0xFFFFFFFF
    a, b, c, d = 0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476
    msg = bytearray(data)
    if not (broken and len(data) % 64 == 0):
        msg.append(0x80)
        while len(msg) % 64 != 56:
            msg.append(0)
        msg += struct.pack("<Q", (len(data) * 8) & 0xFFFFFFFF)  # baseline writes only the low 32 bits
    for off in range(0, len(msg), 64):
        x = struct.unpack("<16I", bytes(msg[off:off + 64]))
        aa, bb, cc, dd = a, b, c, d
        f = lambda x_, y_, z_: (x_ & y_) | (~x_ & z_)
        g = lambda x_, y_, z_: (x_ & y_) | (x_ & z_) | (y_ & z_)
        h = lambda x_, y_, z_: x_ ^ y_ ^ z_
        for i in range(16):
            s = (3, 7, 11, 19)[i % 4]
            t = [a, b, c, d]
            k = i
            a, b, c, d = d, rol(a + f(b, c, d) + x[k], s), b, c
        for i in range(16):
            s = (3, 5, 9, 13)[i % 4]
            k = (i % 4) * 4 + i // 4
            a, b, c, d = d, rol(a + g(b, c, d) + x[k] + 0x5A827999, s), b, c
        order = (0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15)
        for i in range(16):
            s = (3, 9, 11, 15)[i % 4]
            k = order[i]
            a, b, c, d = d, rol(a + h(b, c, d) + x[k] + 0x6ED9EBA1, s), b, c
        a, b, c, d = [(p + q) & 0xFFFFFFFF for p, q in zip((a, b, c, d), (aa, bb, cc, dd))]
    return struct.pack("<4I", a, b, c, d)


def payload(length, seed):
    out = bytearray(length)
    x = (0x9E3779B97F4A7C15 ^ seed) & 0xFFFFFFFFFFFFFFFF
    for i in range(length):
        x = (x * 6364136223846793005 + 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
        out[i] = x >> 56
    return bytes(out)


def rsum(block):
    a = b = 0
    n = len(block)
    for i, byte in enumerate(block):
        a = (a + byte) & 0xFFFF
        b = (b + (n - i) * byte) & 0xFFFF
    return a | (b << 16)


def weak_bytes(value, nbytes):
    b0, b1, b2, b3 = value & 0xFF, (value >> 8) & 0xFF, (value >> 16) & 0xFF, (value >> 24) & 0xFF
    if nbytes == 2:
        return bytes([b3, b2])
    if nbytes == 3:
        return bytes([b0, b3, b2])
    return bytes([b1, b0, b3, b2])


def check_zsync(meta_path, payload_path):
    raw = open(meta_path, "rb").read()
    head_end = raw.index(b"\n\n") + 2
    head = {}
    for line in raw[:head_end].decode("ascii").splitlines():
        if ": " in line:
            k, v = line.split(": ", 1)
            head[k] = v
    data = open(payload_path, "rb").read()
    bs = int(head["Blocksize"])
    seq, rb, cb = [int(x) for x in head["Hash-Lengths"].split(",")]
    assert int(head["Length"]) == len(data), "Length header"
    assert head["SHA-1"] == hashlib.sha1(data).hexdigest(), "SHA-1 header"
    assert head["Filename"] == os.path.basename(payload_path), "Filename header"
    nblocks = -(-len(data) // bs)
    body = raw[head_end:]
    assert len(body) == nblocks * (rb + cb), "body length %d != %d" % (len(body), nblocks * (rb + cb))
    for i in range(nblocks):
        blk = data[i * bs:(i + 1) * bs].ljust(bs, b"\0")
        entry = body[i * (rb + cb):(i + 1) * (rb + cb)]
        assert entry[:rb] == weak_bytes(rsum(blk), rb), "weak sum block %d" % i
        assert entry[rb:] == _md4(blk, False)[:cb], "strong sum block %d" % i
    return nblocks


# ----------------------------------------------------------------------------- descriptor inventory

def collect_descriptors(reader, into):
    seen = set()
    for h in reader.handles:
        if isinstance(h, ClassDesc) and h.name not in seen:
            seen.add(h.name)
            into.setdefault(h.name, {"suid": h.suid, "flags": h.flags, "fields": [[c, n, t] for c, n, t in h.fields],
                                     "super": h.sup.name if h.sup else None, "seen_in": []})


# ----------------------------------------------------------------------------- main

def sha256(b):
    return hashlib.sha256(b).hexdigest()


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    root = argv[1]
    write_inv = "--write-inventory" in argv
    manifest = json.load(open(os.path.join(root, "manifest.json"), encoding="utf-8"))
    problems = []
    notes = []

    def fail(msg):
        problems.append(msg)
        print("FAIL", msg)

    inventory = {}
    deferred = []
    decoded = 0
    for row in manifest["files"]:
        rel = row["path"]
        path = os.path.join(root, rel)
        kind = row["kind"]
        if kind == "zsync-generation-failure":
            continue
        if not os.path.isfile(path):
            fail("missing " + rel)
            continue
        raw = open(path, "rb").read()
        if sha256(raw) != row["sha256"] or len(raw) != row["bytes"]:
            fail("hash/size mismatch " + rel)
            continue
        if "expected" in row:
            ex = open(os.path.join(root, row["expected"]), "rb").read()
            if sha256(ex) != row["expected_sha256"]:
                fail("expected file hash mismatch " + row["expected"])
        if kind in ("invalid", "invalid-zip"):
            if kind == "invalid-zip":
                continue
            try:
                inner = gunzip_bounded(raw)
                _, rd_inv = read_serialization(inner)
            except FormatError as e:
                notes.append("rejected %-34s %s" % (rel, e))
            else:
                if os.path.basename(rel).startswith("class-"):
                    deferred.append((rel, rd_inv))  # structurally valid; rejected by descriptor table below
                else:
                    fail("invalid fixture accepted " + rel)
            continue
        if raw[:2] == b"\x1f\x8b":
            inner = gunzip_bounded(raw)
            if "payload_sha256" in row and sha256(inner) != row["payload_sha256"]:
                fail("payload hash mismatch " + rel)
                continue
            try:
                top, rd = read_serialization(inner)
            except FormatError as e:
                fail("valid fixture not decodable %s: %s" % (rel, e))
                continue
            decoded += 1
            collect_descriptors(rd, inventory)
            for name in {h.name for h in rd.handles if isinstance(h, ClassDesc)}:
                inventory[name]["seen_in"].append(rel)
            if kind == "repository-sealed":
                if top.desc.name != "javax.crypto.SealedObject":
                    fail("sealed fixture has wrong top-level class " + rel)
                f = top.fields["javax.crypto.SealedObject"]
                if f["sealAlg"] != "AES" or len(f["encryptedContent"]) % 16 != 0:
                    fail("unexpected sealed object parameters " + rel)
                continue
            got = canon(neutral(top))
            want = canon(json.loads(open(os.path.join(root, row["expected"]), encoding="utf-8").read()))
            if got != want:
                fail("decoded value differs from expected for " + rel)
                open(os.path.join(os.environ.get("TEMP", "."), "mig001-got.json"), "w").write(got)
                open(os.path.join(os.environ.get("TEMP", "."), "mig001-want.json"), "w").write(want)
        elif kind == "zsync-metafile":
            n = check_zsync(path, os.path.join(root, rel[:-len(".zsync")]))
            notes.append("zsync %-40s %d blocks recomputed" % (rel, n))
        elif kind == "md4-vectors":
            for v in json.loads(raw)["vectors"]:
                data = v["ascii"].encode("ascii") if v["ascii"] is not None else payload(v["length"], v["seed"] or 0)
                if _md4(data, False).hex() != v["md4"] or _md4(data, True).hex() != v["broken_md4"]:
                    fail("md4 vector mismatch length=%d" % v["length"])
        elif kind == "invalid-zip":
            pass

    # derived inventory + cross-check with reflection inventory
    ci = {c["name"]: c for c in json.load(open(os.path.join(root, "class-inventory.json")))["classes"]}
    for name, d in sorted(inventory.items()):
        if name in ci and ci[name]["serial_version_uid"] != d["suid"]:
            fail("suid differs between stream and reflection for " + name)
    for rel, rd_inv in deferred:
        reason = None
        for h in rd_inv.handles:
            if isinstance(h, ClassDesc) and (h.name not in inventory or inventory[h.name]["suid"] != h.suid):
                reason = "descriptor %s not in table or serialVersionUID differs" % h.name
                break
        if reason:
            notes.append("rejected %-34s %s" % (rel, reason))
        else:
            fail("invalid fixture accepted by descriptor table " + rel)
    inv_path = os.path.join(root, "descriptor-inventory.json")
    out = {"description": "Derived by verify.py from the decoded fixtures; not part of manifest.json.",
           "classes": [dict(name=n, suid=d["suid"], suid_hex="%016x" % (d["suid"] & 0xFFFFFFFFFFFFFFFF), flags=d["flags"],
                            super=d["super"], fields=d["fields"], seen_in=sorted(set(d["seen_in"])),
                            explicit_uid_in_source=(ci[n]["declared_explicitly"] if n in ci else None))
                       for n, d in sorted(inventory.items())]}
    text = json.dumps(out, indent=1, sort_keys=False) + "\n"
    if write_inv:
        open(inv_path, "w", encoding="utf-8", newline="\n").write(text)
    elif os.path.isfile(inv_path):
        if open(inv_path, encoding="utf-8").read() != text:
            fail("descriptor-inventory.json is stale (rerun with --write-inventory)")
    else:
        fail("descriptor-inventory.json missing (run with --write-inventory)")

    for n in notes:
        print(n)
    print("decoded %d serialized fixtures; %d descriptors; %d problems" % (decoded, len(inventory), len(problems)))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
