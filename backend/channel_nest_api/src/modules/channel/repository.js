const { Op } = require('sequelize');
const {
  ChannelAccount,
  ChannelAuthTask,
  ChannelPlatform,
  ChannelSyncLog,
} = require('./model');

function plain(row) {
  return row && typeof row.toJSON === 'function' ? row.toJSON() : row;
}

class ChannelRepository {
  async listPlatforms() {
    return ChannelPlatform.findAll({
      where: { is_enabled: true },
      order: [['sort_no', 'ASC'], ['id', 'ASC']],
    }).then((rows) => rows.map(plain));
  }

  async getPlatform(platformId) {
    return ChannelPlatform.findOne({
      where: {
        platform_id: platformId,
        is_enabled: true,
      },
    }).then(plain);
  }

  async listAccounts(userId, entries = {}) {
    const where = {
      user_id: userId,
      is_deleted: false,
    };

    if (entries.platform_id) {
      where.platform_id = entries.platform_id;
    }

    if (entries.search_key) {
      where[Op.or] = [
        { nickname: { [Op.like]: `%${entries.search_key}%` } },
        { platform_uid: { [Op.like]: `%${entries.search_key}%` } },
      ];
    }

    const rows = await ChannelAccount.findAll({
      where,
      order: [['platform_id', 'ASC'], ['updated_at', 'DESC']],
    });

    return rows.map(plain);
  }

  async getAccount(userId, accountId) {
    return ChannelAccount.findOne({
      where: {
        id: accountId,
        user_id: userId,
        is_deleted: false,
      },
    }).then(plain);
  }

  async findAccountByIdentity(userId, platformId, platformUid) {
    const or = [];

    if (platformUid) {
      or.push({ platform_uid: platformUid });
    }

    if (or.length === 0) {
      return null;
    }

    return ChannelAccount.findOne({
      where: {
        user_id: userId,
        platform_id: platformId,
        is_deleted: false,
        [Op.or]: or,
      },
    }).then(plain);
  }

  async upsertAccount(identity, entries) {
    const existing = await this.findAccountByIdentity(
      entries.user_id,
      entries.platform_id,
      identity.platform_uid,
    );

    if (existing) {
      await ChannelAccount.update(entries, {
        where: { id: existing.id },
      });

      return this.getAccount(entries.user_id, existing.id);
    }

    const created = await ChannelAccount.create(entries);

    return plain(created);
  }

  async updateAccount(userId, accountId, entries) {
    await ChannelAccount.update(entries, {
      where: {
        id: accountId,
        user_id: userId,
        is_deleted: false,
      },
    });

    return this.getAccount(userId, accountId);
  }

  async deleteAccount(userId, accountId) {
    await ChannelAccount.update({
      is_deleted: true,
      deleted_at: new Date(),
    }, {
      where: {
        id: accountId,
        user_id: userId,
        is_deleted: false,
      },
    });
  }

  async createAuthTask(entries) {
    const created = await ChannelAuthTask.create(entries);

    return plain(created);
  }

  async getAuthTask(userId, taskId) {
    return ChannelAuthTask.findOne({
      where: {
        id: taskId,
        user_id: userId,
      },
    }).then(plain);
  }

  async updateAuthTask(userId, taskId, entries) {
    await ChannelAuthTask.update(entries, {
      where: {
        id: taskId,
        user_id: userId,
      },
    });

    return this.getAuthTask(userId, taskId);
  }

  async createSyncLog(entries) {
    const created = await ChannelSyncLog.create(entries);

    return plain(created);
  }
}

module.exports = new ChannelRepository();
