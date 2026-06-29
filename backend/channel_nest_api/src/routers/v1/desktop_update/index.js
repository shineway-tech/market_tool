const Router = require('koa-router');
const config = require('../../../../config');
const controller = require('./desktop_update');
const { checkPublishRelease } = require('./filter');

const router = new Router();

function checkReleaseToken(ctx, next) {
  const expectedToken = (config.desktop_update || {}).release_token;
  const authHeader = ctx.get('authorization');
  const bearerToken = authHeader.startsWith('Bearer ') ? authHeader.slice(7) : '';
  const token = ctx.get('x-desktop-update-token') || bearerToken;

  if (!expectedToken || token !== expectedToken) {
    ctx.throw(401, 'Invalid desktop update release token');
  }

  return next();
}

router.get('/downloads/:fileName', controller.download);
router.post('/release', checkReleaseToken, checkPublishRelease, controller.publish);
router.get('/:target/:arch/:currentVersion', controller.check);

module.exports = router;
