const { DataTypes } = require('sequelize');
const sequelize = require('../../platform/sequelizor');

const Feedback = sequelize.define('Feedback', {
  id: {
    type: DataTypes.CHAR(36),
    primaryKey: true,
  },
  user_id: {
    type: DataTypes.CHAR(36),
    allowNull: false,
  },
  content: {
    type: DataTypes.TEXT,
    allowNull: false,
  },
  contact: {
    type: DataTypes.STRING(191),
    allowNull: false,
    defaultValue: '',
  },
  status: {
    type: DataTypes.STRING(32),
    allowNull: false,
    defaultValue: 'new',
  },
}, {
  tableName: 'mm_feedbacks',
  underscored: true,
});

module.exports = {
  Feedback,
};
