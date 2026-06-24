const { Feedback } = require('./model');

function plain(row) {
  return row && typeof row.toJSON === 'function' ? row.toJSON() : row;
}

class FeedbackRepository {
  async create(entries) {
    const created = await Feedback.create(entries);

    return plain(created);
  }
}

module.exports = new FeedbackRepository();
