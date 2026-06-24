const Router = require('koa-router');
const controller = require('./controller');

const router = new Router();

router.get('/downloads/:fileName', controller.download);
router.get('/:target/:arch/:currentVersion', controller.check);

module.exports = router;
