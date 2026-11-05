#!/usr/bin/env node
/**
 * Set the application version in every place that declares one.
 *
 * The version lives in three files by necessity — npm's manifest, Cargo's
 * manifest, and Tauri's config — and Tauri's bundler names its artefacts from
 * the config value. Editing them by hand is how releases end up shipping a
 * binary whose version disagrees with its own installer, so this script is the
 * only supported way to bump it.
 *
 *   node scripts/set-version.mjs 2.1.0
 *   node scripts/set-version.mjs --check      # fail if they disagree
 */

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const PACKAGE = resolve(root, "package.json");
const CARGO = resolve(root, "src-tauri/Cargo.toml");
const TAURI = resolve(root, "src-tauri/tauri.conf.json");

/** Semantic version, optionally with a prerelease suffix. */
const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

/** Replace only the `version` key inside `[package]`, leaving the rest intact. */
function readCargoVersion(toml) {
  const match = toml.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  return match ? match[1] : null;
}

function writeCargoVersion(toml, version) {
  const section = toml.indexOf("[package]");
  if (section === -1) {
    throw new Error("Cargo.toml has no [package] section");
  }
  const next = toml.replace(
    /^(\[package\][\s\S]*?^version\s*=\s*)"[^"]+"/m,
    `$1"${version}"`,
  );
  if (next === toml) {
    throw new Error("could not find the [package] version in Cargo.toml");
  }
  return next;
}

function currentVersions() {
  const pkg = readJson(PACKAGE);
  const tauri = readJson(TAURI);
  const cargo = readCargoVersion(readFileSync(CARGO, "utf8"));
  return { npm: pkg.version, tauri: tauri.version, cargo };
}

function main() {
  const arg = process.argv[2];
  const versions = currentVersions();

  if (!arg || arg === "--check") {
    const unique = new Set(Object.values(versions));
    if (unique.size !== 1) {
      console.error("Version mismatch:");
      for (const [source, value] of Object.entries(versions)) {
        console.error(`  ${source}: ${value}`);
      }
      process.exit(1);
    }
    console.log(`All manifests agree: ${versions.npm}`);
    return;
  }

  if (!SEMVER.test(arg)) {
    console.error(`Not a semantic version: ${arg}`);
    process.exit(1);
  }

  // package.json
  const pkg = readJson(PACKAGE);
  pkg.version = arg;
  writeFileSync(PACKAGE, `${JSON.stringify(pkg, null, 2)}\n`);

  // tauri.conf.json
  const tauri = readJson(TAURI);
  tauri.version = arg;
  writeFileSync(TAURI, `${JSON.stringify(tauri, null, 2)}\n`);

  // Cargo.toml — text edit so comments and formatting survive.
  const cargo = readFileSync(CARGO, "utf8");
  writeFileSync(CARGO, writeCargoVersion(cargo, arg));

  console.log(`Version set to ${arg}`);
  for (const [source, value] of Object.entries(currentVersions())) {
    console.log(`  ${source}: ${value}`);
  }
}

main();
