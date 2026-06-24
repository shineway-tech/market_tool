const { Op } = require('sequelize');
const { AuthCaptcha, AuthUser } = require('./model');

function plain(row) {
  return row && typeof row.toJSON === 'function' ? row.toJSON() : row;
}

class AuthRepository {
  async createUser(entries) {
    const created = await AuthUser.create(entries);

    return plain(created);
  }

  async findUserByAccount(account) {
    return AuthUser.findOne({
      where: { account },
    }).then(plain);
  }

  async findUserById(id) {
    return AuthUser.findOne({
      where: { id },
    }).then(plain);
  }

  async markLogin(userId) {
    await AuthUser.update({
      last_login_at: new Date(),
    }, {
      where: { id: userId },
    });
  }

  async updateUser(userId, entries) {
    await AuthUser.update(entries, {
      where: { id: userId },
    });

    return this.findUserById(userId);
  }

  async createCaptcha(entries) {
    const created = await AuthCaptcha.create(entries);

    return plain(created);
  }

  async findValidCaptcha(id) {
    return AuthCaptcha.findOne({
      where: {
        id,
        used_at: null,
        expires_at: {
          [Op.gt]: new Date(),
        },
      },
    }).then(plain);
  }

  async markCaptchaUsed(id) {
    await AuthCaptcha.update({
      used_at: new Date(),
    }, {
      where: { id },
    });
  }
}

module.exports = new AuthRepository();
