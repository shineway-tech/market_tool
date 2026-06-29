const { DataTypes } = require('sequelize');
const sequelize = require('../libs/sequelizor');

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

module.exports = AuthCaptcha;
