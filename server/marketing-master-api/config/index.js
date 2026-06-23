const fs = require('fs');
const path = require('path');
const lodash = require('lodash');

const configPath = path.resolve(__dirname, 'config.json');

const defaultConfig = {
  env: 'dev',
  host: '127.0.0.1',
  port: 3100,
  sign_token: 'marketing-master-local-sign',
  jwt_secret: 'marketing-master-local-secret',
  auth: {
    allow_anonymous_desktop: false,
  },
  mysql: {
    db: {
      database: 'marketing_master',
      userName: 'root',
      password: '',
      conn: {
        host: '127.0.0.1',
        port: 3306,
        dialect: 'mysql',
        logging: false,
        timezone: '+08:00',
        define: {
          charset: 'utf8mb4',
          collate: 'utf8mb4_unicode_ci',
        },
      },
    },
  },
  relay: {
    enabled: true,
    server_url: 'https://aitoearn.cn/api',
    api_key: '',
    timeout: 18000,
  },
  desktop_update: {
    enabled: true,
    latest_version: '1.0.0',
    pub_date: '2026-06-23T00:00:00Z',
    notes: '账号密码登录与注册、验证码校验、个人信息和密码修改；支持小红书、视频号、抖音、哔哩哔哩、快手渠道授权和多账号管理；支持账号刷新、删除、打开创作者主页；支持中英文、深浅色主题、本地 JSON 配置和意见反馈。',
    download_dir: 'public/desktop-updates',
    platforms: {},
  },
  dingtalk: {
    token: '',
  },
};

function readConfigFile() {
  if (!fs.existsSync(configPath)) {
    return {};
  }

  return JSON.parse(fs.readFileSync(configPath, 'utf8'));
}

function normalizeConfig(config) {
  const normalized = config;
  const mysqlLogging = lodash.get(normalized, 'mysql.db.conn.logging', false);

  lodash.set(normalized, 'mysql.db.conn.logging', mysqlLogging ? console.log : false);

  return normalized;
}

module.exports = normalizeConfig(lodash.defaultsDeep({}, readConfigFile(), defaultConfig));
