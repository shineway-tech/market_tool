module.exports = {
  async up(queryInterface, DataTypes, { transaction }) {
    const table = await queryInterface.describeTable('mm_channel_accounts');

    if (!table.likes) {
      await queryInterface.addColumn('mm_channel_accounts', 'likes', {
        type: DataTypes.BIGINT.UNSIGNED,
        allowNull: true,
      }, { transaction });
    }
  },

  async down(queryInterface, DataTypes, { transaction }) {
    const table = await queryInterface.describeTable('mm_channel_accounts');

    if (table.likes) {
      await queryInterface.removeColumn('mm_channel_accounts', 'likes', { transaction });
    }
  },
};
