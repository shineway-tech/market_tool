const { DataTypes } = require('sequelize');
const sequelize = require('../../platform/sequelizor');

const AuthUser = sequelize.define('AuthUser', {
  id: {
    type: DataTypes.CHAR(36),
    primaryKey: true,
  },
  account: {
    type: DataTypes.STRING(64),
    allowNull: false,
    unique: true,
  },
  nickname: {
    type: DataTypes.STRING(64),
    allowNull: false,
  },
  password_hash: {
    type: DataTypes.STRING(255),
    allowNull: false,
  },
  status: {
    type: DataTypes.STRING(32),
    allowNull: false,
    defaultValue: 'active',
  },
  last_login_at: DataTypes.DATE,
}, {
  tableName: 'mm_users',
  underscored: true,
});

const AuthCaptcha = sequelize.define('AuthCaptcha', {
  id: {
    type: DataTypes.CHAR(36),
    primaryKey: true,
  },
  code: {
    type: DataTypes.STRING(16),
    allowNull: false,
  },
  scene: {
    type: DataTypes.STRING(32),
    allowNull: false,
    defaultValue: 'auth',
  },
  expires_at: {
    type: DataTypes.DATE,
    allowNull: false,
  },
  used_at: DataTypes.DATE,
}, {
  tableName: 'mm_captcha_codes',
  underscored: true,
});

module.exports = {
  AuthCaptcha,
  AuthUser,
};
