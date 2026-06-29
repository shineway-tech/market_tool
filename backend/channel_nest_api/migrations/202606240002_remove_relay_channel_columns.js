module.exports = {
  async up(queryInterface, DataTypes, { transaction }) {
    const platformTable = await queryInterface.describeTable('mm_channel_platforms').catch(() => null);
    const accountTable = await queryInterface.describeTable('mm_channel_accounts').catch(() => null);
    const authTaskTable = await queryInterface.describeTable('mm_channel_auth_tasks').catch(() => null);

    if (platformTable && platformTable.relay_platform_id) {
      await queryInterface.removeColumn('mm_channel_platforms', 'relay_platform_id', { transaction });
    }
    if (accountTable && accountTable.relay_account_ref) {
      await queryInterface.removeColumn('mm_channel_accounts', 'relay_account_ref', { transaction });
    }
    if (authTaskTable && authTaskTable.relay_session_id) {
      await queryInterface.removeColumn('mm_channel_auth_tasks', 'relay_session_id', { transaction });
    }
    if (platformTable && platformTable.auth_mode) {
      await queryInterface.bulkUpdate('mm_channel_platforms', {
        auth_mode: 'creator',
      }, {}, { transaction });
    }
  },

  async down(queryInterface, DataTypes, { transaction }) {
    const platformTable = await queryInterface.describeTable('mm_channel_platforms').catch(() => null);
    const accountTable = await queryInterface.describeTable('mm_channel_accounts').catch(() => null);
    const authTaskTable = await queryInterface.describeTable('mm_channel_auth_tasks').catch(() => null);

    if (platformTable && !platformTable.relay_platform_id) {
      await queryInterface.addColumn('mm_channel_platforms', 'relay_platform_id', {
        type: DataTypes.STRING(64),
        allowNull: true,
      }, { transaction });
    }
    if (accountTable && !accountTable.relay_account_ref) {
      await queryInterface.addColumn('mm_channel_accounts', 'relay_account_ref', {
        type: DataTypes.STRING(191),
        allowNull: true,
      }, { transaction });
    }
    if (authTaskTable && !authTaskTable.relay_session_id) {
      await queryInterface.addColumn('mm_channel_auth_tasks', 'relay_session_id', {
        type: DataTypes.STRING(191),
        allowNull: true,
      }, { transaction });
    }
  },
};
