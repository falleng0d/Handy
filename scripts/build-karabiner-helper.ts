import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

if (process.platform !== "darwin") process.exit(0);

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const target =
  process.argv[2] ||
  process.env.TAURI_ENV_TARGET_TRIPLE ||
  execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(
    /^host: (.+)$/m,
  )?.[1];

if (!target) throw new Error("Could not determine the Rust target triple");

const outputDir = path.join(root, "src-tauri", "binaries");
const output = path.join(outputDir, `karabiner-input-helper-${target}`);
const localManifest = path.resolve(root, "..", "karabiner-input", "Cargo.toml");
const buildRoot = path.join(root, "target", "karabiner-input-helper");
const revision = "5e4f238e58b2a8d84104afd3a1148baeae733284";
const localCertificate = process.env.KARABINER_INPUT_LOCAL_CERT_SHA1;
const localCertificateFeature = localCertificate
  ? ["--features", "local-development-certificate"]
  : [];

if (process.env.CI && existsSync(output)) process.exit(0);

mkdirSync(outputDir, { recursive: true });

let binary: string;
if (existsSync(localManifest)) {
  const localRoot = path.dirname(localManifest);
  const head = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: localRoot,
    encoding: "utf8",
  }).trim();
  const status = execFileSync("git", ["status", "--porcelain"], {
    cwd: localRoot,
    encoding: "utf8",
  }).trim();
  if (head !== revision || status) {
    throw new Error(
      `Local karabiner-input must be clean at ${revision}; found ${head}${status ? " with uncommitted changes" : ""}`,
    );
  }
  execFileSync(
    "cargo",
    [
      "build",
      "--manifest-path",
      localManifest,
      "--bin",
      "karabiner-input-helper",
      "--release",
      "--target",
      target,
      "--target-dir",
      buildRoot,
      ...localCertificateFeature,
    ],
    { stdio: "inherit" },
  );
  binary = path.join(buildRoot, target, "release", "karabiner-input-helper");
} else {
  const installRoot = path.join(buildRoot, "install");
  execFileSync(
    "cargo",
    [
      "install",
      "--git",
      "https://github.com/falleng0d/karabiner-input",
      "--rev",
      revision,
      "--bin",
      "karabiner-input-helper",
      "--root",
      installRoot,
      "--target",
      target,
      "--locked",
      "--force",
      ...localCertificateFeature,
    ],
    { stdio: "inherit" },
  );
  binary = path.join(installRoot, "bin", "karabiner-input-helper");
}

copyFileSync(binary, output);
chmodSync(output, 0o755);
