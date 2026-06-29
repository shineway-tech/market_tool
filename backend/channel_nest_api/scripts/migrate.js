const fs = require('fs');
const path = require('path');
const mysql = require('mysql2/promise');
const { DataTypes } = require('sequelize');
const config = require('../config');
const sequelize = require('../src/libs/sequelizor');

const migrationsDir = path.resolve(__dirname, '..', 'migrations');
const command = process.argv[2] || 'up';

async function ensureDatabase() {
  if (config.mysql.db.conn.dialect !== 'mysql') {
    return;
  }

  const connection = await mysql.createConnection({
    host: config.mysql.db.conn.host,
    port: config.mysql.db.conn.port,
    user: config.mysql.db.userName,
    password: config.mysql.db.password,
    multipleStatements: false,
  });

  await connection.query(
    `CREATE DATABASE IF NOT EXISTS \`${config.mysql.db.database}\` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci`,
  ).catch((error) => {
    if (!['ER_DBACCESS_DENIED_ERROR', 'ER_ACCESS_DENIED_ERROR'].includes(error.code)) {
      throw error;
    }
  });
  await connection.end();
}

async function ensureMigrationTable(queryInterface) {
  await queryInterface.createTable('mm_migrations', {
    name: {
      type: DataTypes.STRING(191),
      allowNull: false,
      primaryKey: true,
    },
    batch: {
      type: DataTypes.INTEGER,
      allowNull: false,
      defaultValue: 1,
    },
    migrated_at: {
      type: DataTypes.DATE,
      allowNull: false,
      defaultValue: sequelize.literal('CURRENT_TIMESTAMP'),
    },
  }).catch((error) => {
    if (!String(error.message).includes('already exists')) {
      throw error;
    }
  });
}

async function appliedMigrations() {
  const [rows] = await sequelize.query('SELECT name FROM mm_migrations ORDER BY name ASC');

  return new Set(rows.map(row => row.name));
}

function migrationFiles() {
  return fs.readdirSync(migrationsDir)
    .filter(file => file.endsWith('.js') && !file.startsWith('._'))
    .sort();
}

async function runUp() {
  await ensureDatabase();
  await sequelize.authenticate();
  const queryInterface = sequelize.getQueryInterface();

  await ensureMigrationTable(queryInterface);

  const applied = await appliedMigrations();
  const pending = migrationFiles().filter(file => !applied.has(file));
  let batch = 1;

  if (pending.length === 0) {
    console.log('No pending migrations.');
    return;
  }

  const [latest] = await sequelize.query('SELECT MAX(batch) AS batch FROM mm_migrations');
  batch = Number(latest[0].batch || 0) + 1;

  for (const file of pending) {
    const migration = require(path.join(migrationsDir, file));

    await sequelize.transaction(async (transaction) => {
      await migration.up(queryInterface, DataTypes, { transaction, sequelize });
      await queryInterface.bulkInsert('mm_migrations', [{
        name: file,
        batch,
        migrated_at: new Date(),
      }], { transaction });
    });
    console.log(`Migrated ${file}`);
  }
}

async function printStatus() {
  await ensureDatabase();
  await sequelize.authenticate();
  const queryInterface = sequelize.getQueryInterface();

  await ensureMigrationTable(queryInterface);

  const applied = await appliedMigrations();
  migrationFiles().forEach((file) => {
    console.log(`${applied.has(file) ? 'up' : 'down'} ${file}`);
  });
}

async function main() {
  if (command === 'up') {
    await runUp();
  } else if (command === 'status') {
    await printStatus();
  } else {
    throw new Error(`Unknown migration command: ${command}`);
  }
}

main()
  .catch((error) => {
    console.error(error);
    process.exitCode = 1;
  })
  .finally(async () => {
    await sequelize.close();
  });
