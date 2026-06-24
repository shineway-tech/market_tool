const Router = require('koa-router');
const checkSign = require('@honeykid/ml/middlewares/check_sign');
const config = require('../config');
const setAuthUser = require('./platform/middlewares/set_auth_user');
const auth = require('./modules/auth');
const channel = require('./modules/channel');
const feedback = require('./modules/feedback');
const desktopUpdate = require('./modules/desktop_update');

const router = new Router();
const signWhiteList = [
  /^\/health$/,
  /^\/v1\/auth/,
  /^\/v1\/channel/,
  /^\/v1\/feedback/,
  /^\/v1\/desktop-updates/,
];

router.use(checkSign(config.sign_token, signWhiteList));
router.use(setAuthUser());
router.get('/health', async (ctx) => {
  ctx.setData({
    service: 'marketing-master-api',
    env: config.env,
  });
});
router.use('/v1/auth', auth.routes(), auth.allowedMethods());
router.use('/v1/channel', channel.routes(), channel.allowedMethods());
router.use('/v1/feedback', feedback.routes(), feedback.allowedMethods());
router.use('/v1/desktop-updates', desktopUpdate.routes(), desktopUpdate.allowedMethods());

module.exports = router;
