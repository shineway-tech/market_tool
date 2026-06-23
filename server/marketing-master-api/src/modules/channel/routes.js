const Router = require('koa-router');
const controller = require('./controller');
const {
  checkAccountQuery,
  checkCompleteAuth,
  checkStartAuth,
  checkUpsertAccount,
} = require('./filter');
const checkAuth = require('../../platform/middlewares/check_auth');

const router = new Router();

router.use(checkAuth());

router.get('/bootstrap', controller.bootstrap);
router.get('/platforms', controller.platforms);
router.get('/accounts', checkAccountQuery, controller.accounts);
router.post('/accounts', checkUpsertAccount, controller.upsertAccount);
router.post('/accounts/:account_id/refresh', controller.refreshAccount);
router.delete('/accounts/:account_id', controller.deleteAccount);
router.get('/accounts/:account_id/homepage', controller.homepage);

router.post('/auth/start', checkStartAuth, controller.startAuth);
router.get('/auth/:task_id/status', controller.authStatus);
router.post('/auth/:task_id/complete', checkCompleteAuth, controller.completeAuth);

router.post('/relay/sync', controller.syncRelay);

module.exports = router;
