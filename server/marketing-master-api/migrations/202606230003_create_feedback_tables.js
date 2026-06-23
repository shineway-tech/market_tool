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
    await queryInterface.createTable('mm_feedbacks', {
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
      ...timestamps(DataTypes, sequelize),
    }, { transaction });

    await queryInterface.addIndex('mm_feedbacks', ['user_id', 'created_at'], {
      name: 'idx_mm_feedbacks_user_created',
      transaction,
    });

    await queryInterface.addIndex('mm_feedbacks', ['status', 'created_at'], {
      name: 'idx_mm_feedbacks_status_created',
      transaction,
    });
  },
};
