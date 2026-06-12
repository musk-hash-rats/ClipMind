import { createHash } from "node:crypto";
import { readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const bundleRoot = path.join(repoRoot, "src-tauri", "target", "release", "bundle");
const packageJson = JSON.parse(
  await import("node:fs/promises").then(({ readFile }) =>
    readFile(path.join(repoRoot, "package.json"), "utf8"),
  ),
);

const artifactExtensions = new Set([
  ".appimage",
  ".deb",
  ".dmg",
  ".exe",
  ".msi",
  ".rpm",
]);

async function collectArtifacts(dir) {
  const entries = await readdir(dir, { withFileTypes: true }).catch((error) => {
    if (error.code === "ENOENT") {
      return [];
    }

    throw error;
  });
  const artifacts = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);

    if (entry.isDirectory()) {
      artifacts.push(...(await collectArtifacts(fullPath)));
      continue;
    }

    if (entry.isFile() && artifactExtensions.has(path.extname(entry.name).toLowerCase())) {
      artifacts.push(fullPath);
    }
  }

  return artifacts.sort();
}

async function sha256(filePath) {
  const { readFile } = await import("node:fs/promises");
  const bytes = await readFile(filePath);
  return createHash("sha256").update(bytes).digest("hex");
}

function gitValue(args) {
  try {
    return execFileSync("git", args, {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return null;
  }
}

const artifacts = await collectArtifacts(bundleRoot);

if (artifacts.length === 0) {
  throw new Error(`No release artifacts found under ${path.relative(repoRoot, bundleRoot)}`);
}

const generatedAt = new Date().toISOString();
const commit = process.env.GITHUB_SHA || gitValue(["rev-parse", "HEAD"]);
const gitRef = process.env.GITHUB_REF_NAME || gitValue(["rev-parse", "--abbrev-ref", "HEAD"]);

const records = [];

for (const filePath of artifacts) {
  const fileStat = await stat(filePath);
  const digest = await sha256(filePath);
  const relativePath = path.relative(repoRoot, filePath).replaceAll(path.sep, "/");

  records.push({
    path: relativePath,
    fileName: path.basename(filePath),
    sha256: digest,
    sizeBytes: fileStat.size,
  });
}

const checksums = records.map((record) => `${record.sha256}  ${record.path}`).join("\n") + "\n";
const provenance = {
  schema: "clipmind.release-provenance.v1",
  package: packageJson.name,
  version: packageJson.version,
  generatedAt,
  commit,
  gitRef,
  ci: {
    provider: process.env.GITHUB_ACTIONS === "true" ? "github-actions" : "local",
    runId: process.env.GITHUB_RUN_ID || null,
    runAttempt: process.env.GITHUB_RUN_ATTEMPT || null,
  },
  artifacts: records,
};

await writeFile(path.join(bundleRoot, "SHA256SUMS"), checksums, "utf8");
await writeFile(path.join(bundleRoot, "provenance.json"), JSON.stringify(provenance, null, 2) + "\n", "utf8");

console.log(`Wrote checksums for ${records.length} artifact(s).`);
