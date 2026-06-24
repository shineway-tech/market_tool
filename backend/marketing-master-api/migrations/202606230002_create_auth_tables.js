function timestamps(DataTypes, sequelize) {
  return {
    created_at: {
      type: DataTypes.DATE,
      allowNull: false,
      defaultValue: sequelize.literal('CURRENT_TIMESTAMP'),
    },
    updated_at: {
      type: DataTypes.DATE,
      allowNull: false,
      defaultValue: sequelize.literal('CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP'),
    },
  };
}

module.exports = {
  async up(queryInterface, DataTypes, { transaction, sequelize }) {
    await queryInterface.createTable('mm_users', {
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
      ...timestamps(DataTypes, sequelize),
    }, { transaction });

    await queryInterface.createTable('mm_captcha_codes', {
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
      ...timestamps(DataTypes, sequelize),
    }, { transaction });

    await queryInterface.addIndex('mm_captcha_codes', ['expires_at', 'used_at'], {
      name: 'idx_mm_captcha_codes_expires_used',
      transaction,
    });
  },
};
