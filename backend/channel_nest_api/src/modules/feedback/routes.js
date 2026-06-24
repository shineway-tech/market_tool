const Router = require('koa-router');
const controller = require('./controller');
const { checkCreate } = require('./filter');
const checkAuth = require('../../platform/middlewares/check_auth');

const router = new Router();

router.post('/', checkAuth(), checkCreate, controller.create);

module.exports = router;
