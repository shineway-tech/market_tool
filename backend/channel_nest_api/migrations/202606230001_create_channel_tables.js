module.exports = {
  async up() {
    // Platform accounts, cookies, and creator sessions are client-local.
    // This historical migration is kept as a no-op so old migration order remains stable.
  },

  async down() {
    // No server-side channel tables are created by this migration.
  },
};
