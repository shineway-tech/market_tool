const fs = require('fs');
const path = require('path');
const config = require('../../../config');

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
  const updateConfig = config.desktop_update || {};
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
  const downloadDir = path.isAbsolute(configuredDir)
    ? configuredDir
    : path.resolve(__dirname, '../../..', configuredDir);
  const filePath = path.resolve(downloadDir, fileName);

  if (!filePath.startsWith(`${downloadDir}${path.sep}`) || !fs.existsSync(filePath)) {
    return null;
  }

  return {
    filePath,
    fileName,
  };
}

module.exports = {
  getUpdate,
  getDownloadFile,
};
