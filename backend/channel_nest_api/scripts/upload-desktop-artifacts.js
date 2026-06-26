#!/usr/bin/env node

const fs = require('fs');
const crypto = require('crypto');
const http = require('http');
const https = require('https');
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
  if (!value) {
    throw new Error(`Missing ${label}`);
  }
  return value;
}

function normalizePrefix(prefix) {
  return String(prefix || 'public/market_tool')
    .replace(/^\/+|\/+$/g, '')
    .replace(/\/+/g, '/');
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function stableObjectFileName(fileName, version) {
  const escapedVersion = escapeRegExp(version);

  return fileName
    .replace(new RegExp(`[_-]v?${escapedVersion}(?=[_.-])`, 'g'), '')
    .replace(/__+/g, '_')
    .replace(/--+/g, '-');
}

function stableObjectKeyFor(fileName, version, prefix) {
  return `${prefix}/${stableObjectFileName(fileName, version)}`;
}

function versionedObjectKeyFor(fileName, version, prefix) {
  return `${prefix}/${version}/${stableObjectFileName(fileName, version)}`;
}

function publicUrlFor(objectKey) {
  const baseUrl = process.env.ALIYUN_OSS_PUBLIC_BASE_URL || 'https://cdn.honeykid.cn';
  if (baseUrl) return `${baseUrl.replace(/\/+$/g, '')}/${objectKey}`;

  const bucket = required(process.env.ALIYUN_OSS_BUCKET, 'ALIYUN_OSS_BUCKET');
  const region = required(process.env.ALIYUN_OSS_REGION, 'ALIYUN_OSS_REGION');
  return `https://${bucket}.${region}.aliyuncs.com/${objectKey}`;
}

function objectUrlFor(objectKey) {
  const bucket = required(process.env.ALIYUN_OSS_BUCKET, 'ALIYUN_OSS_BUCKET');
  const region = required(process.env.ALIYUN_OSS_REGION, 'ALIYUN_OSS_REGION');
  const endpoint = String(process.env.ALIYUN_OSS_ENDPOINT || `${region}.aliyuncs.com`)
    .replace(/^https?:\/\//i, '')
    .replace(/\/+$/g, '');

  return new URL(`https://${bucket}.${endpoint}/${objectKey}`);
}

function signOssRequest({ method, contentType, date, objectKey, ossHeaders }) {
  const bucket = required(process.env.ALIYUN_OSS_BUCKET, 'ALIYUN_OSS_BUCKET');
  const accessKeyId = required(process.env.ALIYUN_OSS_ACCESS_KEY_ID, 'ALIYUN_OSS_ACCESS_KEY_ID');
  const accessKeySecret = required(process.env.ALIYUN_OSS_ACCESS_KEY_SECRET, 'ALIYUN_OSS_ACCESS_KEY_SECRET');
  const canonicalizedOssHeaders = Object.entries(ossHeaders || {})
    .map(([key, value]) => [key.toLowerCase(), String(value).trim()])
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, value]) => `${key}:${value}\n`)
    .join('');
  const canonicalizedResource = `/${bucket}/${objectKey}`;
  const stringToSign = [
    method,
    '',
    contentType,
    date,
    `${canonicalizedOssHeaders}${canonicalizedResource}`,
  ].join('\n');
  const signature = crypto
    .createHmac('sha1', accessKeySecret)
    .update(stringToSign)
    .digest('base64');

  return `OSS ${accessKeyId}:${signature}`;
}

function putObject(objectKey, filePath, options = {}) {
  const fileStat = fs.statSync(filePath);
  const method = 'PUT';
  const contentType = 'application/octet-stream';
  const date = new Date().toUTCString();
  const ossHeaders = {};
  if (options.objectAcl) ossHeaders['x-oss-object-acl'] = options.objectAcl;

  const url = objectUrlFor(objectKey);
  const client = url.protocol === 'http:' ? http : https;
  const headers = {
    Date: date,
    'Content-Type': contentType,
    'Content-Length': fileStat.size,
    Authorization: signOssRequest({
      method,
      contentType,
      date,
      objectKey,
      ossHeaders,
    }),
    ...ossHeaders,
  };

  return new Promise((resolve, reject) => {
    const request = client.request(url, {
      method,
      headers,
    }, (response) => {
      const chunks = [];
      response.on('data', (chunk) => chunks.push(chunk));
      response.on('end', () => {
        if (response.statusCode >= 200 && response.statusCode < 300) {
          resolve();
          return;
        }

        reject(new Error(`OSS upload failed: ${response.statusCode} ${Buffer.concat(chunks).toString('utf8')}`));
      });
    });

    request.on('error', reject);
    fs.createReadStream(filePath).pipe(request);
  });
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const artifactDir = path.resolve(required(args.dir, '--dir'));
  const version = required(args.version, '--version').replace(/^v/i, '');
  const platform = required(args.platform, '--platform');
  const updateFileName = required(args['update-file'], '--update-file');
  const signatureFileName = required(args['signature-file'], '--signature-file');
  const manifestFile = path.resolve(required(args['manifest-file'], '--manifest-file'));
  const prefix = normalizePrefix(process.env.ALIYUN_OSS_PREFIX);

  const updateFilePath = path.resolve(artifactDir, updateFileName);
  const signatureFilePath = path.resolve(artifactDir, signatureFileName);
  if (!fs.existsSync(updateFilePath)) throw new Error(`Missing update file: ${updateFilePath}`);
  if (!fs.existsSync(signatureFilePath)) throw new Error(`Missing signature file: ${signatureFilePath}`);

  required(process.env.ALIYUN_OSS_REGION, 'ALIYUN_OSS_REGION');
  required(process.env.ALIYUN_OSS_BUCKET, 'ALIYUN_OSS_BUCKET');
  required(process.env.ALIYUN_OSS_ACCESS_KEY_ID, 'ALIYUN_OSS_ACCESS_KEY_ID');
  required(process.env.ALIYUN_OSS_ACCESS_KEY_SECRET, 'ALIYUN_OSS_ACCESS_KEY_SECRET');

  const uploaded = [];
  const uploadedKeys = new Set();
  const fileNames = fs.readdirSync(artifactDir)
    .filter((fileName) => fs.statSync(path.join(artifactDir, fileName)).isFile());
  const objectAcl = process.env.ALIYUN_OSS_OBJECT_ACL;

  for (const fileName of fileNames) {
    const filePath = path.join(artifactDir, fileName);
    const objectKey = versionedObjectKeyFor(fileName, version, prefix);
    const stableObjectKey = stableObjectKeyFor(fileName, version, prefix);

    for (const key of [objectKey, stableObjectKey]) {
      if (uploadedKeys.has(key)) continue;
      await putObject(key, filePath, { objectAcl });
      uploadedKeys.add(key);
    }

    uploaded.push({
      file: fileName,
      oss_file: path.basename(objectKey),
      object_key: objectKey,
      url: publicUrlFor(objectKey),
      stable_object_key: stableObjectKey,
      stable_url: publicUrlFor(stableObjectKey),
    });
  }

  const updateObjectKey = versionedObjectKeyFor(updateFileName, version, prefix);
  const manifest = {
    version,
    pub_date: new Date().toISOString(),
    platform,
    platforms: {
      [platform]: {
        url: publicUrlFor(updateObjectKey),
        signature: fs.readFileSync(signatureFilePath, 'utf8').trim(),
      },
    },
    uploaded,
  };

  fs.mkdirSync(path.dirname(manifestFile), { recursive: true });
  fs.writeFileSync(manifestFile, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`Uploaded ${uploaded.length} files for ${platform}`);
  console.log(`Wrote ${manifestFile}`);
}

if (require.main === module) {
  main().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
