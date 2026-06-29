async function tableExists(queryInterface, tableName) {
  return queryInterface.describeTable(tableName)
    .then(() => true)
    .catch(() => false);
}

module.exports = {
  async up(queryInterface) {
    const tables = [
      'mm_channel_auth_tasks',
      'mm_channel_sync_logs',
      'mm_channel_accounts',
      'mm_channel_platforms',
    ];

    for (const tableName of tables) {
      if (await tableExists(queryInterface, tableName)) {
        await queryInterface.dropTable(tableName);
      }
    }
  },

  async down() {
    // Platform accounts are client-local now. Old channel tables are intentionally not restored.
  },
};
