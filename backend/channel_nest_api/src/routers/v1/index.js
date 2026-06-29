const Router = require('koa-router');
const auth = require('./auth');
const channel = require('./channel');
const feedback = require('./feedback');
const desktopUpdate = require('./desktop_update');

const router = new Router();

router.use('/auth', auth.routes(), auth.allowedMethods());
router.use('/channel', channel.routes(), channel.allowedMethods());
router.use('/feedback', feedback.routes(), feedback.allowedMethods());
router.use('/desktop-updates', desktopUpdate.routes(), desktopUpdate.allowedMethods());

module.exports = router;
