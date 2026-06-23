const Sequelize = require('sequelize');
const config = require('../../config');

const sequelize = new Sequelize(
  config.mysql.db.database,
  config.mysql.db.userName,
  config.mysql.db.password,
  config.mysql.db.conn,
);

module.exports = sequelize;
