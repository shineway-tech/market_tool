const Router = require('koa-router');
const controller = require('./feedback');
const { checkCreate } = require('./filter');
const checkAuth = require('../../../middlewares/check_auth');

const router = new Router();

router.post('/', checkAuth(), checkCreate, controller.create);

module.exports = router;
