const fs = require('fs');
const path = require('path');
const lodash = require('lodash');
const config = require('../../../config');

function resolveProjectPath(configuredPath) {
  return path.isAbsolute(configuredPath)
    ? configuredPath
    : path.resolve(__dirname, '../../..', configuredPath);
}

function getManifestPath() {
  const updateConfig = config.desktop_update || {};
  return resolveProjectPath(updateConfig.manifest_path || 'storage/desktop-update-manifest.json');
}

function readManifest() {
  const manifestPath = getManifestPath();
  if (!fs.existsSync(manifestPath)) return {};

  return JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
}

function getUpdateConfig() {
  return lodash.defaultsDeep({}, readManifest(), config.desktop_update || {});
}

function normalizeVersion(version) {
  return String(version || '')
    .trim()
    .replace(/^v/i, '')
    .split('.')
    .map((part) => Number.parseInt(part, 10))
    .map((part) => (Number.isFinite(part) ? part : 0));
}

function isNewerVersion(latestVersion, currentVersion) {
  const latest = normalizeVersion(latestVersion);
  const current = normalizeVersion(currentVersion);
  const length = Math.max(latest.length, current.length, 3);

  for (let index = 0; index < length; index += 1) {
    const left = latest[index] || 0;
    const right = current[index] || 0;
    if (left > right) return true;
    if (left < right) return false;
  }

  return false;
}

function getUpdate(target, arch, currentVersion) {
  const updateConfig = getUpdateConfig();
  if (!updateConfig.enabled) return null;

  const latestVersion = updateConfig.latest_version;
  if (!latestVersion || !isNewerVersion(latestVersion, currentVersion)) return null;

  const platformKey = `${target}-${arch}`;
  const platform = (updateConfig.platforms || {})[platformKey];
  if (!platform || !platform.url || !platform.signature) return null;

  return {
    version: latestVersion,
    pub_date: updateConfig.pub_date || undefined,
    url: platform.url,
    signature: platform.signature,
    notes: updateConfig.notes || '',
  };
}

function getDownloadFile(fileName) {
  if (!/^[a-zA-Z0-9._-]+$/.test(fileName || '')) {
    return null;
  }

  const updateConfig = config.desktop_update || {};
  const configuredDir = updateConfig.download_dir || 'public/desktop-updates';
  const downloadDir = resolveProjectPath(configuredDir);
  const filePath = path.resolve(downloadDir, fileName);

  if (!filePath.startsWith(`${downloadDir}${path.sep}`) || !fs.existsSync(filePath)) {
    return null;
  }

  return {
    filePath,
    fileName,
  };
}

function publishRelease(input) {
  const manifestPath = getManifestPath();
  const current = readManifest();
  const next = lodash.defaultsDeep({}, {
    enabled: true,
    latest_version: input.latest_version,
    pub_date: input.pub_date,
    notes: input.notes || '',
    platforms: input.platforms,
    updated_at: new Date().toISOString(),
  }, current);

  next.enabled = true;
  next.latest_version = input.latest_version;
  next.pub_date = input.pub_date;
  next.notes = input.notes || '';
  next.platforms = input.platforms;
  next.updated_at = new Date().toISOString();

  fs.mkdirSync(path.dirname(manifestPath), { recursive: true });
  fs.writeFileSync(manifestPath, `${JSON.stringify(next, null, 2)}\n`);

  return next;
}

module.exports = {
  getUpdate,
  getDownloadFile,
  publishRelease,
};
