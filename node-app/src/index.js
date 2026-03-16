require('dotenv').config();
const express = require('express');
const path = require('path');
const { createProxyMiddleware } = require('http-proxy-middleware');
const helmet = require('helmet');
const cors = require('cors');
const rateLimit = require('express-rate-limit');

const app = express();

// Faire confiance au proxy Docker/reverse-proxy pour obtenir la vraie IP client
app.set('trust proxy', 1);

const PORT = parseInt(process.env.PORT) || 3000;
const RUST_API_URL = process.env.RUST_API_URL || 'http://rust-app:3000';

// ── Sécurité ───────────────────────────────────────────────────────────────────
app.use(helmet({
  contentSecurityPolicy: {
    directives: {
      defaultSrc: ["'self'"],
      scriptSrc: ["'self'", "'unsafe-inline'", "https://cdn.tailwindcss.com", "https://unpkg.com"],
      styleSrc: ["'self'", "'unsafe-inline'", "https://fonts.googleapis.com"],
      fontSrc: ["'self'", "https://fonts.gstatic.com"],
      imgSrc: ["'self'", "https:", "data:"],
      connectSrc: ["'self'"],
      frameSrc: ["'none'"],
      objectSrc: ["'none'"],
      baseUri: ["'self'"],
      formAction: ["'self'"],
      upgradeInsecureRequests: process.env.NODE_ENV === 'development' ? null : [],
    },
  },
  crossOriginEmbedderPolicy: false,
  hsts: process.env.NODE_ENV === 'development' ? false : { maxAge: 31536000, includeSubDomains: true },
}));

app.use(cors({
  origin: process.env.CORS_ORIGIN || false,
  methods: ['GET', 'POST', 'PUT', 'DELETE'],
  credentials: true,
  allowedHeaders: ['Content-Type', 'Authorization'],
}));

// Rate limiting global
app.use(rateLimit({
  windowMs: 15 * 60 * 1000,
  max: 500,
  standardHeaders: true,
  legacyHeaders: false,
  message: { error: 'Trop de requêtes. Réessayez dans 15 minutes.' },
}));

// Rate limiting strict pour les endpoints d'authentification
const authLimiter = rateLimit({
  windowMs: 15 * 60 * 1000,
  max: 10,
  standardHeaders: true,
  legacyHeaders: false,
  message: { error: 'Trop de tentatives. Réessayez dans 15 minutes.' },
});
app.use('/api/auth/login', authLimiter);
app.use('/api/auth/register', authLimiter);

// ── Proxy /api/* → Rust backend (réseau Docker interne) ───────────────────────
// IMPORTANT : Le proxy doit être déclaré AVANT express.json() pour que le body
// brut soit transmis tel quel au backend Rust, sinon Express consomme le stream
// et le backend reçoit un body vide.
app.use(createProxyMiddleware({
  pathFilter: '/api',
  target: RUST_API_URL,
  changeOrigin: true,
  timeout: 30000,
  proxyTimeout: 30000,
  on: {
    proxyReq: (proxyReq, req, _res) => {
      console.log(`[PROXY] ${req.method} ${req.url}`);
    },
    error: (_err, _req, res) => {
      if (!res.headersSent) {
        res.writeHead(502, { 'Content-Type': 'application/json' });
      }
      res.end(JSON.stringify({ status: 'error', message: 'Backend indisponible' }));
    },
  },
}));

// JSON body parser — uniquement pour les routes non-proxifiées
app.use(express.json({ limit: '10kb' }));

// ── Santé ──────────────────────────────────────────────────────────────────────
app.get('/health', (_req, res) => {
  res.json({ status: 'ok', service: 'node-frontend' });
});

// ── Fichiers statiques ────────────────────────────────────────────────────────
app.use(express.static(path.join(__dirname, '../public')));

// SPA fallback
app.get('*', (_req, res) => {
  res.sendFile(path.join(__dirname, '../public', 'index.html'));
});

// ── Serveur HTTP ───────────────────────────────────────────────────────────────
app.listen(PORT, '0.0.0.0', () => {
  console.log(`[HTTP] Frontend sur http://localhost:${PORT}`);
  console.log(`[PROXY] /api/* → ${RUST_API_URL}`);
});

process.on('uncaughtException', (err) => {
  console.error('[Fatal]', err.message);
  process.exit(1);
});
