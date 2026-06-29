const crypto = require('crypto');
const { Op } = require('sequelize');
const {
  BadArgumentError,
  ForbiddenError,
  NotFoundError,
} = require('@honeykid/ml/errors');
const AuthCaptcha = require('../models/auth_captcha');
const AuthUser = require('../models/auth_user');
const Jwt = require('../utils/jwt');
const { JwtTokenTTL } = require('../utils/constants');

const CaptchaChars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';

function plain(row) {
  return row && typeof row.toJSON === 'function' ? row.toJSON() : row;
}

function toClientUser(user) {
  return {
    id: user.id,
    account: user.account,
    nickname: user.nickname,
    status: user.status,
    lastLoginAt: user.last_login_at,
  };
}

function randomCaptchaCode() {
  let code = '';

  for (let index = 0; index < 4; index += 1) {
    code += CaptchaChars[crypto.randomInt(0, CaptchaChars.length)];
  }

  return code;
}

function escapeSvgText(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function captchaSvg(code) {
  const chars = code.split('');
  const text = chars.map((char, index) => {
    const x = 22 + index * 24;
    const y = 38 + (index % 2 === 0 ? 0 : -4);
    const rotate = [-9, 5, -4, 8][index] || 0;

    return `<text x="${x}" y="${y}" transform="rotate(${rotate} ${x} ${y})">${escapeSvgText(char)}</text>`;
  }).join('');

  return `<svg xmlns="http://www.w3.org/2000/svg" width="132" height="48" viewBox="0 0 132 48">
    <rect width="132" height="48" rx="8" fill="#10272d"/>
    <path d="M8 36 C34 8, 58 52, 124 14" stroke="#16e68a" stroke-opacity=".34" stroke-width="2" fill="none"/>
    <path d="M10 16 H122 M18 28 H116" stroke="#8aa0a6" stroke-opacity=".18" stroke-width="1"/>
    <g fill="#d8e5e7" font-family="Menlo, Consolas, monospace" font-size="25" font-weight="800">${text}</g>
  </svg>`;
}

function hashPassword(password, salt = crypto.randomBytes(16).toString('hex')) {
  const hash = crypto.scryptSync(password, salt, 64).toString('hex');

  return `scrypt$${salt}$${hash}`;
}

function verifyPassword(password, passwordHash) {
  const [algorithm, salt, expected] = String(passwordHash || '').split('$');

  if (algorithm !== 'scrypt' || !salt || !expected) {
    return false;
  }

  const actual = hashPassword(password, salt).split('$')[2];
  const expectedBuffer = Buffer.from(expected, 'hex');
  const actualBuffer = Buffer.from(actual, 'hex');

  return expectedBuffer.length === actualBuffer.length
    && crypto.timingSafeEqual(expectedBuffer, actualBuffer);
}

class AuthLogic {
  static async captcha() {
    const code = randomCaptchaCode();
    const item = plain(await AuthCaptcha.create({
      id: crypto.randomUUID(),
      code,
      scene: 'auth',
      expires_at: new Date(Date.now() + 10 * 60 * 1000),
    }));
    const svg = captchaSvg(code);

    return {
      captchaId: item.id,
      image: `data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`,
      expiresAt: item.expires_at,
    };
  }

  static async register(entries) {
    await AuthLogic.verifyCaptcha(entries.captcha_id, entries.captcha_code);

    const existing = await AuthLogic.findUserByAccount(entries.account);

    if (existing) {
      throw new BadArgumentError('账号已存在');
    }

    const user = plain(await AuthUser.create({
      id: crypto.randomUUID(),
      account: entries.account,
      nickname: entries.nickname || entries.account,
      password_hash: hashPassword(entries.password),
      status: 'active',
    }));

    return AuthLogic.issueToken(user);
  }

  static async login(entries) {
    await AuthLogic.verifyCaptcha(entries.captcha_id, entries.captcha_code);

    const user = await AuthLogic.findUserByAccount(entries.account);

    if (!user || !verifyPassword(entries.password, user.password_hash)) {
      throw new BadArgumentError('账号或密码错误');
    }

    if (user.status !== 'active') {
      throw new ForbiddenError('账号不可用');
    }

    await AuthUser.update({
      last_login_at: new Date(),
    }, {
      where: { id: user.id },
    });

    return AuthLogic.issueToken({
      ...user,
      last_login_at: new Date(),
    });
  }

  static async me(userId) {
    const user = await AuthLogic.findUserById(userId);

    if (!user) {
      throw new NotFoundError('用户不存在');
    }

    return toClientUser(user);
  }

  static async updateProfile(userId, entries) {
    const user = await AuthLogic.findUserById(userId);

    if (!user) {
      throw new NotFoundError('用户不存在');
    }

    await AuthUser.update({
      nickname: entries.nickname,
    }, {
      where: { id: userId },
    });

    return toClientUser(await AuthLogic.findUserById(userId));
  }

  static async updatePassword(userId, entries) {
    const user = await AuthLogic.findUserById(userId);

    if (!user) {
      throw new NotFoundError('用户不存在');
    }

    if (!verifyPassword(entries.current_password, user.password_hash)) {
      throw new BadArgumentError('当前密码错误');
    }

    if (verifyPassword(entries.new_password, user.password_hash)) {
      throw new BadArgumentError('新密码不能与当前密码相同');
    }

    await AuthUser.update({
      password_hash: hashPassword(entries.new_password),
    }, {
      where: { id: userId },
    });

    return toClientUser(await AuthLogic.findUserById(userId));
  }

  static async verifyCaptcha(captchaId, code) {
    const captcha = plain(await AuthCaptcha.findOne({
      where: {
        id: captchaId,
        used_at: null,
        expires_at: {
          [Op.gt]: new Date(),
        },
      },
    }));

    if (!captcha) {
      throw new BadArgumentError('验证码已失效');
    }

    await AuthCaptcha.update({
      used_at: new Date(),
    }, {
      where: { id: captchaId },
    });

    if (String(captcha.code).toLowerCase() !== String(code).trim().toLowerCase()) {
      throw new BadArgumentError('验证码错误');
    }
  }

  static issueToken(user) {
    const token = Jwt.generateToken(user.id, JwtTokenTTL);

    return {
      token,
      tokenName: Jwt.getTokenName(),
      expiresIn: JwtTokenTTL,
      user: toClientUser(user),
    };
  }

  static async findUserByAccount(account) {
    return AuthUser.findOne({
      where: { account },
    }).then(plain);
  }

  static async findUserById(id) {
    return AuthUser.findOne({
      where: { id },
    }).then(plain);
  }
}

module.exports = AuthLogic;
