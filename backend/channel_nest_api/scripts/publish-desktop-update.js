#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const item = argv[index];
    if (!item.startsWith('--')) continue;
    const key = item.slice(2);
    const value = argv[index + 1] && !argv[index + 1].startsWith('--')
      ? argv[index += 1]
      : 'true';
    args[key] = value;
  }
  return args;
}

function required(value, label) {
  if (!value) throw new Error(`Missing ${label}`);
  return value;
}

function readManifestFiles(manifestDir) {
  return fs.readdirSync(manifestDir)
    .filter((fileName) => fileName.endsWith('.json'))
    .map((fileName) => path.join(manifestDir, fileName))
    .map((filePath) => JSON.parse(fs.readFileSync(filePath, 'utf8')));
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const manifestDir = path.resolve(required(args['manifest-dir'], '--manifest-dir'));
  const version = required(args.version, '--version').replace(/^v/i, '');
  const apiUrl = process.env.DESKTOP_UPDATE_API_URL
    || 'https://market-api.honeykid.cn/v1/desktop-updates/release';
  const token = required(process.env.DESKTOP_UPDATE_RELEASE_TOKEN, 'DESKTOP_UPDATE_RELEASE_TOKEN');
  const notes = process.env.DESKTOP_UPDATE_NOTES || 'Desktop release for Channel Nest.';
  const manifests = readManifestFiles(manifestDir);

  if (!manifests.length) throw new Error(`No manifest files found in ${manifestDir}`);

  const platforms = {};
  for (const manifest of manifests) {
    Object.assign(platforms, manifest.platforms || {});
  }

  const payload = {
    latest_version: version,
    pub_date: new Date().toISOString(),
    notes,
    platforms,
  };

  const response = await fetch(apiUrl, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-desktop-update-token': token,
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`Failed to publish desktop update: ${response.status} ${body}`);
  }

  console.log(`Published desktop update ${version} for ${Object.keys(platforms).join(', ')}`);
}

if (require.main === module) {
  main().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
