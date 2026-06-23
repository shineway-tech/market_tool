const { DataTypes } = require('sequelize');
const sequelize = require('../../platform/sequelizor');

const ChannelPlatform = sequelize.define('ChannelPlatform', {
  id: {
    type: DataTypes.BIGINT.UNSIGNED,
    primaryKey: true,
    autoIncrement: true,
  },
  platform_id: {
    type: DataTypes.STRING(64),
    allowNull: false,
    unique: true,
  },
  relay_platform_id: DataTypes.STRING(64),
  name: {
    type: DataTypes.STRING(64),
    allowNull: false,
  },
  slug: {
    type: DataTypes.STRING(32),
    allowNull: false,
  },
  color: {
    type: DataTypes.STRING(32),
    allowNull: false,
  },
  description: DataTypes.STRING(255),
  supports_builtin_oauth: {
    type: DataTypes.BOOLEAN,
    allowNull: false,
    defaultValue: true,
  },
  auth_mode: {
    type: DataTypes.STRING(32),
    allowNull: false,
    defaultValue: 'relay',
  },
  sort_no: {
    type: DataTypes.INTEGER,
    allowNull: false,
    defaultValue: 0,
  },
  is_enabled: {
    type: DataTypes.BOOLEAN,
    allowNull: false,
    defaultValue: true,
  },
  config_json: DataTypes.JSON,
}, {
  tableName: 'mm_channel_platforms',
  underscored: true,
});

const ChannelAccount = sequelize.define('ChannelAccount', {
  id: {
    type: DataTypes.CHAR(36),
    primaryKey: true,
  },
  user_id: {
    type: DataTypes.STRING(64),
    allowNull: false,
    defaultValue: 'local-desktop',
  },
  platform_id: {
    type: DataTypes.STRING(64),
    allowNull: false,
  },
  platform_uid: {
    type: DataTypes.STRING(191),
    allowNull: false,
  },
  nickname: {
    type: DataTypes.STRING(191),
    allowNull: false,
  },
  avatar: DataTypes.TEXT,
  followers: DataTypes.BIGINT.UNSIGNED,
  status: {
    type: DataTypes.STRING(32),
    allowNull: false,
    defaultValue: 'active',
  },
  relay_account_ref: DataTypes.STRING(191),
  access_token: DataTypes.TEXT,
  refresh_token: DataTypes.TEXT,
  token_expires_at: DataTypes.DATE,
  token_payload: DataTypes.JSON,
  login_cookie: DataTypes.TEXT('medium'),
  webview_session_id: DataTypes.STRING(191),
  homepage_url: DataTypes.TEXT,
  last_sync_at: DataTypes.DATE,
  is_deleted: {
    type: DataTypes.BOOLEAN,
    allowNull: false,
    defaultValue: false,
  },
  deleted_at: DataTypes.DATE,
}, {
  tableName: 'mm_channel_accounts',
  underscored: true,
});

const ChannelAuthTask = sequelize.define('ChannelAuthTask', {
  id: {
    type: DataTypes.CHAR(36),
    primaryKey: true,
  },
  user_id: {
    type: DataTypes.STRING(64),
    allowNull: false,
    defaultValue: 'local-desktop',
  },
  platform_id: {
    type: DataTypes.STRING(64),
    allowNull: false,
  },
  mode: {
    type: DataTypes.STRING(32),
    allowNull: false,
  },
  auth_type: {
    type: DataTypes.STRING(32),
    allowNull: false,
    defaultValue: 'oauth',
  },
  relay_session_id: DataTypes.STRING(191),
  url: DataTypes.TEXT,
  callback_url: DataTypes.TEXT,
  status: {
    type: DataTypes.STRING(32),
    allowNull: false,
    defaultValue: 'pending',
  },
  message: DataTypes.STRING(255),
  account_id: DataTypes.CHAR(36),
  expires_at: DataTypes.DATE,
  payload_json: DataTypes.JSON,
}, {
  tableName: 'mm_channel_auth_tasks',
  underscored: true,
});

const ChannelSyncLog = sequelize.define('ChannelSyncLog', {
  id: {
    type: DataTypes.BIGINT.UNSIGNED,
    primaryKey: true,
    autoIncrement: true,
  },
  user_id: {
    type: DataTypes.STRING(64),
    allowNull: false,
    defaultValue: 'local-desktop',
  },
  account_id: {
    type: DataTypes.CHAR(36),
    allowNull: false,
  },
  action: {
    type: DataTypes.STRING(64),
    allowNull: false,
  },
  status: {
    type: DataTypes.STRING(32),
    allowNull: false,
  },
  message: DataTypes.STRING(255),
  payload_json: DataTypes.JSON,
}, {
  tableName: 'mm_channel_sync_logs',
  underscored: true,
  updatedAt: false,
});

module.exports = {
  ChannelAccount,
  ChannelAuthTask,
  ChannelPlatform,
  ChannelSyncLog,
};
