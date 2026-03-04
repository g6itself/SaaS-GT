require('dotenv').config();
const express = require('express');
const { Pool } = require('pg');

const app = express();
const PORT = process.env.PORT || 3000;

// Pool de connexions PostgreSQL
const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
});

app.use(express.json());

app.get('/', (req, res) => {
  res.json({ status: 'ok', message: 'Node.js API en cours d\'exécution' });
});

app.get('/health', async (req, res) => {
  try {
    await pool.query('SELECT 1');
    res.json({ status: 'ok', database: 'connectée' });
  } catch (err) {
    res.status(503).json({ status: 'error', database: err.message });
  }
});

app.listen(PORT, '0.0.0.0', () => {
  console.log(`Serveur Node.js démarré sur le port ${PORT}`);
});
