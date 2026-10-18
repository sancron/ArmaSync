// SPDX-License-Identifier: AGPL-3.0-only
//
// MIG-001 fixture generator. Builds SYNTHETIC legacy-format files by using the
// unmodified baseline classes and the real baseline writer (A3SFilesAccessor), so
// the output is what the legacy application would write. It contains no user data.
//
// Usage (see run.sh / run.ps1):
//   java -cp <baseline-classes>:<this-dir> FixtureGenerator generate <outDir>
//   java -cp <baseline-classes>:<this-dir> FixtureGenerator verify   <fixtureDir>
//
// The class names that appear below belong to the frozen baseline (wire format data).
import fr.soe.a3s.constant.ModsetType;
import fr.soe.a3s.constant.ProtocolType;
import fr.soe.a3s.dao.A3SFilesAccessor;
import fr.soe.a3s.dao.EncryptionProvider;
import fr.soe.a3s.domain.AbstractProtocole;
import fr.soe.a3s.domain.AbstractProtocoleFactory;
import fr.soe.a3s.domain.Preferences;
import fr.soe.a3s.domain.Profile;
import fr.soe.a3s.domain.TreeDirectory;
import fr.soe.a3s.domain.TreeLeaf;
import fr.soe.a3s.domain.configration.Configuration;
import fr.soe.a3s.domain.configration.ExternalApplication;
import fr.soe.a3s.domain.configration.FavoriteServer;
import fr.soe.a3s.domain.repository.AutoConfig;
import fr.soe.a3s.domain.repository.Changelog;
import fr.soe.a3s.domain.repository.Changelogs;
import fr.soe.a3s.domain.repository.Event;
import fr.soe.a3s.domain.repository.Events;
import fr.soe.a3s.domain.repository.Repository;
import fr.soe.a3s.domain.repository.ServerInfo;
import fr.soe.a3s.domain.repository.SyncTreeDirectory;
import fr.soe.a3s.domain.repository.SyncTreeLeaf;
import fr.soe.a3s.jazsync.JarsyncProvider;
import fr.soe.a3s.jazsyncmake.MetaFileMaker;

import java.io.*;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.security.Security;
import java.text.SimpleDateFormat;
import java.util.*;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import java.util.zip.*;
import javax.crypto.Cipher;

public class FixtureGenerator {

	static final String BASELINE_SHA = "b7753cea18b2ac2cc57e29f926901997a4de8570";
	static final long FIXED_TIME = 1767225600000L; // 2026-01-01T00:00:00Z
	static final Date FIXED_DATE = new Date(FIXED_TIME);

	static Path out;
	/** manifest rows: path -> row */
	static final TreeMap<String, Map<String, Object>> rows = new TreeMap<>();

	public static void main(String[] args) throws Exception {
		TimeZone.setDefault(TimeZone.getTimeZone("UTC"));
		Locale.setDefault(Locale.ROOT);
		if (args.length != 2) {
			System.err.println("usage: FixtureGenerator generate|verify <dir>");
			System.exit(2);
		}
		if (args[0].equals("generate")) {
			generate(Paths.get(args[1]));
		} else if (args[0].equals("verify")) {
			System.exit(verify(Paths.get(args[1])) ? 0 : 1);
		} else {
			System.err.println("unknown command " + args[0]);
			System.exit(2);
		}
	}

	// ------------------------------------------------------------------ generate

	static void generate(Path dir) throws Exception {
		out = dir;
		if (Files.exists(out) && Files.list(out).findAny().isPresent()) {
			throw new IOException("output directory must be empty: " + out);
		}
		Files.createDirectories(out);
		Security.addProvider(new JarsyncProvider());

		// --- local data (written by the DAOs as GZIP + Java serialization) ---
		plain("valid/local/a3s.cfg", "configuration", configuration());
		plain("valid/local/a3s.prefs", "preferences", preferences());
		plain("valid/local/profiles/Alpha.a3s.profile", "profile", profileAlpha());
		plain("valid/local/profiles/Unicode-Long.a3s.profile", "profile", profileUnicodeLong());
		sealed("valid/local/repositories/Example-Sftp.a3s.repository", "repository-sealed", repository("Example SFTP",
				ProtocolType.SFTP, "sftp.example.invalid", "22"));
		sealed("valid/local/repositories/Example-Http.a3s.repository", "repository-sealed", repository("Example HTTP",
				ProtocolType.HTTPS, "repo.example.invalid", "443"));
		plain("valid/local/autoconfig-export/export.a3s.autoconfig", "autoconfig-export", autoConfigExport());

		// --- server metadata (.a3s folder) ---
		serverMetadata("valid/server/empty", emptySync(), 1, 0L);
		serverMetadata("valid/server/small", smallSync(), 3, 2L);
		serverMetadata("valid/server/hostile-paths", hostileSync(), 1, 0L);

		// --- edge cases that are VALID serializations ---
		plain("valid/edge/events-self-referencing-list.bin", "edge-events", selfReferencingEvents());

		// --- zsync ---
		zsyncFixtures();
		md4Vectors();

		// --- invalid inputs ---
		invalidFixtures();
		zipFixtures();

		writeClassInventory();
		writeManifest();
		System.out.println("generated " + rows.size() + " files in " + out);
	}

	// ------------------------------------------------------------------ builders

	static AbstractProtocole protocol(ProtocolType type, String host, String port) {
		// synthetic, non-secret placeholder credentials
		return AbstractProtocoleFactory.getProtocol(host, port, "fixture-user", "fixture-password-not-a-secret", type, true);
	}

	static FavoriteServer favorite(String name, String ip, int port, String modset, String repo) {
		FavoriteServer f = new FavoriteServer();
		f.setDescription(name);
		f.setIpAddress(ip);
		f.setPort(port);
		f.setPassword("fixture-server-password-not-a-secret");
		f.setModsetName(modset);
		f.setRepositoryName(repo);
		return f;
	}

	static Configuration configuration() {
		Configuration c = new Configuration();
		c.setHeight(720);
		c.setWidth(1280);
		c.setProfileName("Alpha");
		c.setServerName("Example Server");
		c.setDefaultModset("Core");
		c.setViewModeTree(false);
		c.getFavoriteServers().add(favorite("Example Server", "192.0.2.10", 2302, "Core", "Example SFTP"));
		c.getFavoriteServers().add(favorite("Second", "192.0.2.11", 2402, null, null));
		ExternalApplication app = new ExternalApplication();
		app.setName("Example Tool");
		app.setExecutablePath("C:\\Example\\tool.exe");
		app.setParameters("--flag \"two words\"");
		app.setEnable(true);
		c.getExternalApplications().add(app);
		c.getAddonSearchDirectoryPaths().add("D:\\Example\\Mods");
		c.getAddonSearchDirectoryPaths().add("/home/example/mods");
		c.getProxy().setEnableProxy(true);
		c.getProxy().setProxyProtocol(protocol(ProtocolType.SOCKS5, "proxy.example.invalid", "1080"));
		c.getLauncherOptions().setGameProfile("Alpha");
		c.getLauncherOptions().setNoPause(true);
		return c;
	}

	static Preferences preferences() {
		Preferences p = new Preferences();
		p.setPreferGitHubUpdates(true);
		return p;
	}

	static Profile profileAlpha() {
		Profile p = new Profile("Alpha");
		p.setAdditionalParameters("-world=empty");
		TreeDirectory root = p.getTree();
		TreeDirectory modset = new TreeDirectory("Core", root);
		modset.setModsetType(ModsetType.REPOSITORY);
		modset.setModsetRepositoryName("Example SFTP");
		root.addTreeNode(modset);
		TreeLeaf a = new TreeLeaf("@example_core", modset);
		a.setSelected(true);
		modset.addTreeNode(a);
		TreeLeaf w = new TreeLeaf("@workshop_item", modset);
		w.setWorkshopId("1234567890");
		w.setOptional(true);
		modset.addTreeNode(w);
		TreeLeaf d = new TreeLeaf("@dlc_item", root);
		d.setDlcAppId("9999999");
		root.addTreeNode(d);
		p.getAddonNamesByPriority().add("@example_core");
		p.getAddonNamesByPriority().add("@workshop_item");
		p.getAddonSearchDirectories().add("D:\\Example\\Mods");
		p.setLastAddedAddonSearchDirectory("D:\\Example\\Mods");
		p.getLauncherOptions().setGameProfile("Alpha");
		return p;
	}

	static Profile profileUnicodeLong() {
		// modified UTF-8 edge cases: NUL (C0 80), 2/3-byte BMP, supplementary (surrogate pair, 6 bytes), >65535 byte string
		Profile p = new Profile("Ünïcödé 日本語 \uD83D\uDE00 nul\u0000mid");
		p.setAdditionalParameters("x".repeat(70000));
		return p;
	}

	@SuppressWarnings({ "rawtypes", "unchecked" })
	static Events selfReferencingEvents() {
		Events e = new Events();
		((List) e.getList()).add(e.getList()); // list contains itself -> back reference to an object under construction
		return e;
	}

	static Repository repository(String name, ProtocolType type, String host, String port) {
		Repository r = new Repository(name, protocol(type, host, port));
		r.setRevision(7);
		r.setPath("/srv/example/" + name.replace(' ', '-'));
		r.setAutoConfigURL(host + "/.a3s/autoconfig");
		r.setDefaultDownloadLocation("D:\\Example\\Mods");
		r.getMapEventsDownloadLocation().put("Event One", "E:\\Events");
		r.setNotify(true);
		r.setAuto(false);
		r.setExactMatch(true);
		r.setNoAutoDiscover(false);
		r.setNumberOfConnections(3);
		r.setCompressed(true);
		r.setUploadProtocole(protocol(ProtocolType.SFTP, "upload.example.invalid", "22"));
		r.setParallelUploadConnections(5);
		r.getHiddenFolderPath().add("hidden/folder");
		r.getExcludedFilesFromBuild().add("*.tmp");
		r.getExcludedFoldersFromSync().add("userconfig");
		r.getFavoriteServersSetToAutoconfig().add(favorite("Auto Server", "192.0.2.20", 2302, "Core", name));
		return r;
	}

	static AutoConfig autoConfigServer(String repoName) {
		AutoConfig a = new AutoConfig();
		a.setRepositoryName(repoName);
		a.setProtocole(protocol(ProtocolType.HTTPS, "repo.example.invalid", "443"));
		a.getFavoriteServers().add(favorite("Auto Server", "192.0.2.20", 2302, "Core", repoName));
		return a;
	}

	static fr.soe.a3s.domain.AutoConfig autoConfigExport() {
		fr.soe.a3s.domain.AutoConfig a = new fr.soe.a3s.domain.AutoConfig();
		a.getProfiles().add(profileAlpha());
		a.getFavoriteServers().add(favorite("Auto Server", "192.0.2.20", 2302, "Core", "Example SFTP"));
		a.getRepositories().add(repository("Example SFTP", ProtocolType.SFTP, "sftp.example.invalid", "22"));
		return a;
	}

	static SyncTreeDirectory emptySync() {
		return new SyncTreeDirectory("racine", null);
	}

	static SyncTreeLeaf leaf(SyncTreeDirectory parent, String name, long size, String sha1) {
		SyncTreeLeaf l = new SyncTreeLeaf(name, parent);
		l.setSize(size);
		l.setSha1(sha1);
		parent.addTreeNode(l);
		return l;
	}

	static SyncTreeDirectory smallSync() throws Exception {
		SyncTreeDirectory root = new SyncTreeDirectory("racine", null);
		SyncTreeDirectory addon = new SyncTreeDirectory("@example_core", root);
		addon.setMarkAsAddon(true);
		root.addTreeNode(addon);
		SyncTreeDirectory addons = new SyncTreeDirectory("addons", addon);
		addon.addTreeNode(addons);
		leaf(addons, "example_core.pbo", 8193, sha1Hex(payload(8193, 3)));
		leaf(addons, "example_core.pbo.zip", 100, sha1Hex(payload(100, 4))).setCompressed(true);
		SyncTreeDirectory keys = new SyncTreeDirectory("keys", addon);
		addon.addTreeNode(keys);
		leaf(keys, "example.bikey", 20, sha1Hex(payload(20, 5)));
		SyncTreeDirectory second = new SyncTreeDirectory("@second", root);
		second.setMarkAsAddon(true);
		second.setHidden(true);
		root.addTreeNode(second);
		return root;
	}

	static SyncTreeDirectory hostileSync() {
		// Data a hostile or broken server might publish. A parser must return these verbatim; a later
		// path-validation layer must reject or neutralise them. Names are deliberately NOT sanitised here.
		SyncTreeDirectory root = new SyncTreeDirectory("racine", null);
		String[] names = { "..", "../escape.pbo", "a/../../b.pbo", "C:\\Windows\\x.pbo", "\\\\host\\share\\x.pbo",
				"/absolute.pbo", "CON", "NUL.txt", "file.pbo:stream", "trailing-dot.", "trailing-space ",
				"nul\u0000byte.pbo", "back\\slash.pbo", "percent%2e%2e%2fx.pbo", "Case.pbo", "case.PBO",
				"caf\u00e9.pbo", "cafe\u0301.pbo" };
		for (String n : names) {
			leaf(root, n, 1, "0000000000000000000000000000000000000000");
		}
		return root;
	}

	static ServerInfo serverInfo(SyncTreeDirectory sync, int revision, long files) {
		ServerInfo s = new ServerInfo();
		s.setRevision(revision);
		s.setBuildDate(FIXED_DATE);
		s.setNumberOfFiles(files);
		long total = 0;
		for (SyncTreeLeaf l : sync.getDeepSearchLeafsList()) {
			total += l.getSize();
		}
		s.setTotalFilesSize(total);
		s.setNumberOfConnections(4);
		s.getHiddenFolderPaths().add("@second");
		s.setCompressedPboFilesOnly(false);
		return s;
	}

	static Changelogs changelogs(int revisions) {
		Changelogs cs = new Changelogs();
		for (int i = 1; i <= revisions; i++) {
			Changelog c = new Changelog();
			c.setRevision(i);
			c.setBuildDate(new Date(FIXED_TIME + i * 86400000L));
			c.setContentUpdated(i > 1);
			c.getNewAddons().add("@example_core");
			if (i > 1) c.getUpdatedAddons().add("@example_core");
			if (i > 2) c.getDeletedAddons().add("@removed");
			c.getAddons().add("@example_core");
			cs.getList().add(c);
		}
		return cs;
	}

	static Events events(boolean withEntry) {
		Events es = new Events();
		if (withEntry) {
			Event e = new Event("Event One");
			e.setDescription("Synthetic event");
			e.getAddonNames().put("@example_core", Boolean.TRUE);
			e.getAddonNames().put("@second", Boolean.FALSE);
			e.getDlcNames().put("DLC One", Boolean.TRUE);
			e.getUserconfigFolderNames().put("userconfig_a", Boolean.TRUE);
			es.getList().add(e);
		}
		return es;
	}

	// ------------------------------------------------------------------ writers

	static void plain(String rel, String kind, Serializable obj) throws Exception {
		Path p = out.resolve(rel);
		Files.createDirectories(p.getParent());
		A3SFilesAccessor.write(obj, p.toFile());
		Object back = A3SFilesAccessor.read(p.toFile()); // the real legacy reader must accept it
		record(rel, kind, true, Dump.toJson(back), null);
	}

	static void sealed(String rel, String kind, Serializable obj) throws Exception {
		Path p = out.resolve(rel);
		Files.createDirectories(p.getParent());
		Cipher enc = EncryptionProvider.getEncryptionCipher();
		A3SFilesAccessor.write(obj, enc, p.toFile());
		Object back = A3SFilesAccessor.read(EncryptionProvider.getDecryptionCipher(), p.toFile());
		record(rel, kind, true, Dump.toJson(back), null);
	}

	static void serverMetadata(String dir, SyncTreeDirectory sync, int revisions, long files) throws Exception {
		String base = dir + "/.a3s/";
		plain(base + "sync", "server-sync", sync);
		plain(base + "serverinfo", "server-serverinfo", serverInfo(sync, revisions, files));
		plain(base + "changelogs", "server-changelogs", changelogs(revisions));
		plain(base + "events", "server-events", events(revisions > 1));
		plain(base + "autoconfig", "server-autoconfig", autoConfigServer("Example HTTP"));
	}

	// ------------------------------------------------------------------ zsync / MD4

	/** Deterministic pseudo-random bytes (LCG), independent of the JDK. */
	static byte[] payload(int length, int seed) {
		byte[] b = new byte[length];
		long x = 0x9E3779B97F4A7C15L ^ seed;
		for (int i = 0; i < length; i++) {
			x = x * 6364136223846793005L + 1442695040888963407L;
			b[i] = (byte) (x >>> 56);
		}
		return b;
	}

	static void zsyncFixtures() throws Exception {
		int[] sizes = { 1, 100, 8190, 8191, 8192, 8193, 16384, 20000 };
		for (int size : sizes) {
			String rel = "valid/zsync/payload-" + size + ".bin";
			Path p = out.resolve(rel);
			Files.createDirectories(p.getParent());
			Files.write(p, payload(size, size));
			p.toFile().setLastModified(FIXED_TIME);
			record(rel, "zsync-payload", true, null, null);
			Path z = out.resolve(rel + ".zsync");
			String url = "https://repo.example.invalid/files/payload-" + size + ".bin";
			Map<String, Object> extra = new LinkedHashMap<>();
			extra.put("payload_sha1", sha1Hex(Files.readAllBytes(p)));
			// Work in a scratch directory: on failure the baseline code leaks open file handles
			// (Windows keeps them locked until GC), so nothing in the fixture tree may be touched.
			Path scratch = Files.createTempDirectory("mig001-zsync");
			Path sp = scratch.resolve(p.getFileName().toString());
			Files.copy(p, sp);
			sp.toFile().setLastModified(FIXED_TIME);
			Path sz = scratch.resolve(z.getFileName().toString());
			try {
				new MetaFileMaker(sp.toFile(), sz.toFile(), url, sha1Hex(Files.readAllBytes(p)));
				Files.copy(sz, z);
			} catch (Throwable t) {
				// Baseline defect: Generator.generateSums(File) computes a negative read length when
				// size % blocksize == blocksize - 1 (here 8191, 16383, ...). No .zsync can be produced.
				Map<String, Object> failure = new LinkedHashMap<>();
				failure.put("baseline_defect", "MetaFileMaker throws " + t.getClass().getSimpleName() + " when size % 8192 == 8191");
				failure.put("payload_path", rel);
				failure.put("size", (long) size);
				rows.put(rel + ".zsync", orderedRow(rel + ".zsync", "zsync-generation-failure", failure));
				continue;
			} finally {
				for (Path q : new Path[] { sz, sp, scratch }) {
					try {
						Files.deleteIfExists(q);
					} catch (IOException ignore) {
						q.toFile().deleteOnExit();
					}
				}
			}
			record(rel + ".zsync", "zsync-metafile", true, null, extra);
		}
	}

	static void md4Vectors() throws Exception {
		StringBuilder sb = new StringBuilder("{\n  \"description\": \"Digest outputs of the two baseline digest classes. 'MD4' pads like RFC 1320; "
				+ "'BrokenMD4' is rsync-compatible and does not pad when the input length is a multiple of 64.\",\n  \"vectors\": [\n");
		byte[][] inputs = { new byte[0], "a".getBytes(StandardCharsets.US_ASCII), "abc".getBytes(StandardCharsets.US_ASCII),
				"message digest".getBytes(StandardCharsets.US_ASCII), payload(55, 1), payload(56, 1), payload(63, 1), payload(64, 1),
				payload(65, 1), payload(128, 1), payload(8191, 1), payload(8192, 1), payload(8193, 1) };
		for (int i = 0; i < inputs.length; i++) {
			byte[] in = inputs[i];
			sb.append("    {\"length\": ").append(in.length).append(", \"seed\": ").append(i < 4 ? "null" : "1")
					.append(", \"ascii\": ").append(i < 4 ? Dump.str(new String(in, StandardCharsets.US_ASCII)) : "null")
					.append(", \"md4\": \"").append(hex(MessageDigest.getInstance("MD4").digest(in))).append("\", \"broken_md4\": \"")
					.append(hex(MessageDigest.getInstance("BrokenMD4").digest(in))).append("\"}")
					.append(i + 1 < inputs.length ? ",\n" : "\n");
		}
		sb.append("  ],\n  \"payload_generator\": \"LCG: x = 0x9E3779B97F4A7C15 ^ seed; per byte x = x*6364136223846793005 + 1442695040888963407 (mod 2^64); byte = x >>> 56\"\n}\n");
		raw("valid/md4-vectors.json", "md4-vectors", sb.toString().getBytes(StandardCharsets.UTF_8), true, null);
	}

	// ------------------------------------------------------------------ invalid

	static byte[] gzip(byte[] data) throws IOException {
		ByteArrayOutputStream bo = new ByteArrayOutputStream();
		try (GZIPOutputStream g = new GZIPOutputStream(bo)) {
			g.write(data);
		}
		return bo.toByteArray();
	}

	static byte[] gunzip(byte[] data) throws IOException {
		try (GZIPInputStream g = new GZIPInputStream(new ByteArrayInputStream(data))) {
			return g.readAllBytes();
		}
	}

	static byte[] serialize(Object o) throws IOException {
		ByteArrayOutputStream bo = new ByteArrayOutputStream();
		try (ObjectOutputStream oo = new ObjectOutputStream(bo)) {
			oo.writeObject(o);
		}
		return bo.toByteArray();
	}

	/** Not on the baseline allow-list. */
	public static class UnknownFixtureType implements Serializable {
		private static final long serialVersionUID = 1L;
		String value = "x";
	}

	static int indexOf(byte[] hay, byte[] needle) {
		outer: for (int i = 0; i + needle.length <= hay.length; i++) {
			for (int j = 0; j < needle.length; j++) {
				if (hay[i + j] != needle[j]) continue outer;
			}
			return i;
		}
		return -1;
	}

	static void invalid(String name, String why, byte[] data) throws Exception {
		String rel = "invalid/" + name;
		Path tmp = Files.createTempFile("mig001", ".bin");
		Files.write(tmp, data);
		String outcome;
		try {
			Object o = A3SFilesAccessor.read(tmp.toFile());
			outcome = "ACCEPTED:" + (o == null ? "null" : o.getClass().getSimpleName());
		} catch (Throwable t) {
			outcome = "REJECTED:" + t.getClass().getSimpleName() + (t.getCause() != null ? "/" + t.getCause().getClass().getSimpleName() : "");
		} finally {
			// A3SFilesAccessor.read leaks its FileInputStream when GZIPInputStream rejects the header,
			// so on Windows the file may stay locked until GC; best effort only.
			try {
				Files.deleteIfExists(tmp);
			} catch (IOException ignore) {
				tmp.toFile().deleteOnExit();
			}
		}
		Map<String, Object> extra = new LinkedHashMap<>();
		extra.put("why", why);
		extra.put("baseline_reader_outcome", outcome);
		raw(rel, "invalid", data, false, extra);
	}

	static void invalidFixtures() throws Exception {
		byte[] good = serialize(profileAlpha());
		invalid("empty.bin", "zero-length file", new byte[0]);
		invalid("not-gzip.bin", "plain text, no GZIP magic", "hello, this is not gzip\n".getBytes(StandardCharsets.US_ASCII));
		byte[] gz = gzip(good);
		invalid("gzip-truncated.bin", "valid GZIP cut after half of its bytes", Arrays.copyOf(gz, gz.length / 2));
		byte[] badCrc = gz.clone();
		badCrc[badCrc.length - 8] ^= 0x55;
		invalid("gzip-bad-crc.bin", "GZIP trailer CRC32 corrupted", badCrc);
		invalid("gzip-not-serialization.bin", "valid GZIP whose payload is not a Java serialization stream",
				gzip("{\"json\": true}".getBytes(StandardCharsets.US_ASCII)));
		byte[] badVersion = good.clone();
		badVersion[3] = 0x07; // stream version 7
		invalid("serialization-bad-version.bin", "STREAM_VERSION patched from 5 to 7", gzip(badVersion));
		invalid("serialization-truncated.bin", "serialization stream cut in the middle of an object",
				gzip(Arrays.copyOf(good, good.length / 2)));
		invalid("class-not-allowlisted-jdk.bin", "java.net.URI is a JDK class outside the baseline allow-list",
				gzip(serialize(new java.net.URI("https://example.invalid/"))));
		invalid("class-unknown.bin", "application class outside the baseline package/allow-list", gzip(serialize(new UnknownFixtureType())));
		byte[] uidSwap = good.clone();
		byte[] nameBytes = "fr.soe.a3s.domain.Profile".getBytes(StandardCharsets.US_ASCII);
		int at = indexOf(uidSwap, nameBytes);
		for (int i = 0; i < 8; i++) {
			uidSwap[at + nameBytes.length + i] ^= 0x7F; // serialVersionUID of top-level class flipped
		}
		invalid("class-uid-mismatch.bin", "serialVersionUID of the top-level Profile descriptor altered", gzip(uidSwap));
		Object nest = new ArrayList<Object>();
		Object cur = nest;
		for (int i = 0; i < 150; i++) {
			ArrayList<Object> n = new ArrayList<Object>();
			@SuppressWarnings("unchecked")
			ArrayList<Object> c = (ArrayList<Object>) cur;
			c.add(n);
			cur = n;
		}
		invalid("depth-150.bin", "ArrayList nested 150 levels (baseline limit is depth 100)", gzip(serialize(nest)));
		// declared array length of 1 GiB with no data following
		ByteArrayOutputStream arr = new ByteArrayOutputStream();
		DataOutputStream d = new DataOutputStream(arr);
		d.writeShort(0xACED);
		d.writeShort(5);
		d.writeByte(0x75); // TC_ARRAY
		d.writeByte(0x72); // TC_CLASSDESC
		d.writeUTF("[B");
		d.writeLong(0xACF317F8060854E0L); // serialVersionUID of byte[]
		d.writeByte(0x02); // SC_SERIALIZABLE
		d.writeShort(0);
		d.writeByte(0x78); // TC_ENDBLOCKDATA
		d.writeByte(0x70); // TC_NULL superclass
		d.writeInt(0x40000000);
		invalid("array-declared-1gib.bin", "byte[] declared with 1 GiB length and no content; a parser must reject before allocating",
				gzip(arr.toByteArray()));
		// highly compressible: GZIP expands to 64 MiB of zeros (not a serialization stream)
		byte[] zeros = new byte[64 * 1024 * 1024];
		invalid("gzip-expands-64mib.bin", "small GZIP expanding to 64 MiB; decompression must be bounded", gzip(zeros));
	}

	static void zipFixtures() throws Exception {
		ByteArrayOutputStream bo = new ByteArrayOutputStream();
		try (ZipOutputStream z = new ZipOutputStream(bo)) {
			String[] names = { "ok/file.txt", "../escape.txt", "/absolute.txt", "dir\\backslash.txt", "ok/../../up.txt" };
			for (String n : names) {
				ZipEntry e = new ZipEntry(n);
				e.setTime(FIXED_TIME);
				z.putNextEntry(e);
				z.write(("synthetic " + n).getBytes(StandardCharsets.UTF_8));
				z.closeEntry();
			}
		}
		Map<String, Object> extra = new LinkedHashMap<>();
		extra.put("why", "ZIP with traversal, absolute and backslash entry names; extraction must stay inside the target root");
		extra.put("entries", Arrays.asList("ok/file.txt", "../escape.txt", "/absolute.txt", "dir\\backslash.txt", "ok/../../up.txt"));
		raw("invalid/zip-traversal.zip", "invalid-zip", bo.toByteArray(), false, extra);
	}

	// ------------------------------------------------------------------ class inventory

	static void writeClassInventory() throws Exception {
		// Reflect over every baseline class that is Serializable and in the app package.
		Path classes = Paths.get(System.getProperty("baseline.classes", "build/classes/java/main"));
		List<String> names = new ArrayList<>();
		try (Stream<Path> s = Files.walk(classes)) {
			for (Path p : (Iterable<Path>) s.filter(x -> x.toString().endsWith(".class"))::iterator) {
				String n = classes.relativize(p).toString().replace(File.separatorChar, '.');
				names.add(n.substring(0, n.length() - 6));
			}
		}
		Collections.sort(names);
		StringBuilder sb = new StringBuilder("{\n  \"baseline_sha\": \"" + BASELINE_SHA + "\",\n  \"classes\": [\n");
		boolean first = true;
		for (String n : names) {
			Class<?> c;
			try {
				c = Class.forName(n, false, FixtureGenerator.class.getClassLoader());
			} catch (Throwable t) {
				continue;
			}
			if (!Serializable.class.isAssignableFrom(c) || c.isInterface() || c.isAnonymousClass()) continue;
			if (Throwable.class.isAssignableFrom(c)) continue;
			java.io.ObjectStreamClass osc = java.io.ObjectStreamClass.lookup(c);
			if (osc == null) continue;
			boolean explicit = false;
			try {
				Field f = c.getDeclaredField("serialVersionUID");
				explicit = Modifier.isStatic(f.getModifiers());
			} catch (NoSuchFieldException ignore) {
			}
			if (!first) sb.append(",\n");
			first = false;
			sb.append("    {\"name\": ").append(Dump.str(n)).append(", \"serial_version_uid\": ").append(osc.getSerialVersionUID())
					.append(", \"declared_explicitly\": ").append(explicit).append(", \"enum\": ").append(c.isEnum()).append("}");
		}
		sb.append("\n  ]\n}\n");
		raw("class-inventory.json", "class-inventory", sb.toString().getBytes(StandardCharsets.UTF_8), true, null);
	}

	// ------------------------------------------------------------------ manifest

	static void raw(String rel, String kind, byte[] data, boolean valid, Map<String, Object> extra) throws Exception {
		Path p = out.resolve(rel);
		Files.createDirectories(p.getParent());
		Files.write(p, data);
		record(rel, kind, valid, null, extra);
	}

	static void record(String rel, String kind, boolean valid, String expectedJson, Map<String, Object> extra) throws Exception {
		byte[] data = Files.readAllBytes(out.resolve(rel));
		Map<String, Object> row = new LinkedHashMap<>();
		row.put("path", rel);
		row.put("kind", kind);
		row.put("valid", valid);
		row.put("bytes", (long) data.length);
		row.put("sha256", sha256Hex(data));
		boolean gz = data.length > 2 && (data[0] & 0xFF) == 0x1F && (data[1] & 0xFF) == 0x8B;
		if (gz && valid) {
			try {
				byte[] payload = gunzip(data);
				row.put("payload_bytes", (long) payload.length);
				row.put("payload_sha256", sha256Hex(payload));
			} catch (IOException ignore) {
			}
		}
		if (expectedJson != null) {
			String ex = "expected/" + rel.replace('/', '.') + ".json";
			Path ep = out.resolve(ex);
			Files.createDirectories(ep.getParent());
			Files.write(ep, (expectedJson + "\n").getBytes(StandardCharsets.UTF_8));
			row.put("expected", ex);
			row.put("expected_sha256", sha256Hex(Files.readAllBytes(ep)));
		}
		if (extra != null) row.putAll(extra);
		rows.put(rel, row);
	}

	static Map<String, Object> orderedRow(String rel, String kind, Map<String, Object> extra) {
		Map<String, Object> row = new LinkedHashMap<>();
		row.put("path", rel);
		row.put("kind", kind);
		row.put("valid", false);
		row.put("sha256", "");
		row.putAll(extra);
		return row;
	}

	static void writeManifest() throws Exception {
		StringBuilder sb = new StringBuilder();
		sb.append("{\n  \"manifest_version\": 1,\n");
		sb.append("  \"task\": \"MIG-001\",\n");
		sb.append("  \"baseline_sha\": \"").append(BASELINE_SHA).append("\",\n");
		sb.append("  \"generator\": \"migration-tools/fixture-generator/FixtureGenerator.java\",\n");
		sb.append("  \"generated_with_java\": ").append(Dump.str(System.getProperty("java.version"))).append(",\n");
		sb.append("  \"fixed_time\": \"2026-01-01T00:00:00Z\",\n");
		sb.append("  \"license\": \"AGPL-3.0-only (generator and synthetic data); contains no personal data or real credentials\",\n");
		sb.append("  \"note\": \"sha256 covers the file bytes (JDK/zlib dependent for GZIP). payload_sha256 covers the decompressed serialization stream and is the stable identity across JDKs.\",\n");
		sb.append("  \"files\": [\n");
		boolean first = true;
		for (Map<String, Object> row : rows.values()) {
			if (!first) sb.append(",\n");
			first = false;
			sb.append("    ").append(Dump.toJsonValue(row, false));
		}
		sb.append("\n  ]\n}\n");
		Files.write(out.resolve("manifest.json"), sb.toString().getBytes(StandardCharsets.UTF_8));
	}

	// ------------------------------------------------------------------ verify

	static boolean verify(Path dir) throws Exception {
		String text = new String(Files.readAllBytes(dir.resolve("manifest.json")), StandardCharsets.UTF_8);
		// Minimal line-oriented reading of our own writer's output: one file object per line.
		int bad = 0, count = 0;
		for (String line : text.split("\n")) {
			line = line.trim();
			if (!line.startsWith("{\"path\"")) continue;
			if (field(line, "kind").equals("zsync-generation-failure")) continue; // documented baseline defect, no file
			String path = field(line, "path");
			String sha = field(line, "sha256");
			Path p = dir.resolve(path);
			count++;
			if (!Files.isRegularFile(p)) {
				System.out.println("MISSING " + path);
				bad++;
				continue;
			}
			String actual = sha256Hex(Files.readAllBytes(p));
			if (!actual.equals(sha)) {
				System.out.println("HASH MISMATCH " + path);
				bad++;
			}
		}
		System.out.println(count + " files checked, " + bad + " problems");
		return bad == 0 && count > 0;
	}

	static String field(String line, String key) {
		String k = "\"" + key + "\": \"";
		int i = line.indexOf(k);
		if (i < 0) return "";
		int s = i + k.length();
		return line.substring(s, line.indexOf('"', s));
	}

	// ------------------------------------------------------------------ helpers

	static String hex(byte[] b) {
		StringBuilder sb = new StringBuilder();
		for (byte x : b) sb.append(String.format("%02x", x));
		return sb.toString();
	}

	static String sha256Hex(byte[] b) throws Exception {
		return hex(MessageDigest.getInstance("SHA-256").digest(b));
	}

	static String sha1Hex(byte[] b) throws Exception {
		return hex(MessageDigest.getInstance("SHA-1").digest(b));
	}

	// ------------------------------------------------------------------ neutral dump

	/**
	 * Reflective "field dump" of what a parser must extract. Rules (shared with verify.py):
	 * keys sorted; sets as {"_set": [...]} (unordered); enums as constant name; dates as ISO-8601 UTC;
	 * only fields that Java serialization writes (non-static, non-transient); fields named
	 * "parent" omitted (back references); an object met again on the current path is {"_cycle": true};
	 * every object carries "_type" with the simple class name.
	 */
	static final class Dump {
		static String toJson(Object o) {
			return toJsonValue(dump(o, new IdentityHashMap<>()), true);
		}

		static Object dump(Object o, IdentityHashMap<Object, Boolean> path) {
			if (o == null) return null;
			if (o instanceof String || o instanceof Boolean || o instanceof Number) return o;
			if (o instanceof Character) return String.valueOf(o);
			if (o instanceof Enum) return ((Enum<?>) o).name();
			if (o instanceof Date) {
				SimpleDateFormat f = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'");
				f.setTimeZone(TimeZone.getTimeZone("UTC"));
				return f.format((Date) o);
			}
			if (path.containsKey(o)) {
				Map<String, Object> m = new TreeMap<>();
				m.put("_cycle", true);
				return m;
			}
			path.put(o, true);
			try {
				if (o instanceof Map) {
					Map<String, Object> m = new TreeMap<>();
					for (Map.Entry<?, ?> e : ((Map<?, ?>) o).entrySet()) {
						m.put(String.valueOf(e.getKey()), dump(e.getValue(), path));
					}
					return m;
				}
				if (o instanceof Set) {
					List<Object> l = new ArrayList<>();
					for (Object x : (Set<?>) o) l.add(dump(x, path));
					l.sort(Comparator.comparing(x -> toJsonValue(x, false)));
					Map<String, Object> wrapped = new TreeMap<>();
					wrapped.put("_set", l); // unordered: consumers compare as a multiset
					return wrapped;
				}
				if (o instanceof Collection) {
					List<Object> l = new ArrayList<>();
					for (Object x : (Collection<?>) o) l.add(dump(x, path));
					return l;
				}
				Map<String, Object> m = new TreeMap<>();
				m.put("_type", o.getClass().getSimpleName());
				List<Class<?>> chain = new ArrayList<>();
				for (Class<?> c = o.getClass(); c != null && Serializable.class.isAssignableFrom(c); c = c.getSuperclass()) chain.add(c);
				for (Class<?> c : chain) {
					for (java.io.ObjectStreamField sf : java.io.ObjectStreamClass.lookup(c).getFields()) {
						if (sf.getName().equals("parent")) continue;
						try {
							Field f = c.getDeclaredField(sf.getName());
							f.setAccessible(true);
							m.put(sf.getName(), dump(f.get(o), path));
						} catch (ReflectiveOperationException ex) {
							throw new IllegalStateException(ex);
						}
					}
				}
				return m;
			} finally {
				path.remove(o);
			}
		}

		static String str(String s) {
			StringBuilder sb = new StringBuilder("\"");
			for (int i = 0; i < s.length(); i++) {
				char c = s.charAt(i);
				switch (c) {
				case '"': sb.append("\\\""); break;
				case '\\': sb.append("\\\\"); break;
				case '\n': sb.append("\\n"); break;
				case '\r': sb.append("\\r"); break;
				case '\t': sb.append("\\t"); break;
				default:
					if (c < 0x20 || c > 0x7E) sb.append(String.format("\\u%04x", (int) c));
					else sb.append(c);
				}
			}
			return sb.append('"').toString();
		}

		@SuppressWarnings("unchecked")
		static String toJsonValue(Object v, boolean pretty) {
			if (v == null) return "null";
			if (v instanceof String) return str((String) v);
			if (v instanceof Boolean || v instanceof Integer || v instanceof Long || v instanceof Short || v instanceof Byte) return v.toString();
			if (v instanceof Double || v instanceof Float) return v.toString();
			if (v instanceof Number) return v.toString();
			if (v instanceof Map) {
				StringBuilder sb = new StringBuilder("{");
				boolean first = true;
				for (Map.Entry<String, Object> e : ((Map<String, Object>) v).entrySet()) {
					if (!first) sb.append(", ");
					first = false;
					sb.append(str(e.getKey())).append(": ").append(toJsonValue(e.getValue(), false));
				}
				return sb.append("}").toString();
			}
			if (v instanceof List) {
				return ((List<Object>) v).stream().map(x -> toJsonValue(x, false)).collect(Collectors.joining(", ", "[", "]"));
			}
			return str(v.toString());
		}
	}
}
