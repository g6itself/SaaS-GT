# Achievement Tracker

SaaS de centralisation des achievements/trophees des plateformes gaming **Steam**, **GOG** et **Epic Games**.

## Stack technique

| Composant | Technologie |
|-----------|-------------|
| Backend | Rust - Actix-web 4 |
| Frontend | Rust - Leptos 0.8 (SSR + hydration WASM) |
| Base de donnees | PostgreSQL 16 |
| ORM | SQLx 0.7 (compile-time checked queries) |
| Auth | JWT + Argon2 |
| HTTP Client | reqwest (APIs gaming) |
| Conteneurisation | Docker + Docker Compose |

## Prerequis

- **Docker Desktop** installe et demarre
- **Cle API Steam** (obtenir sur https://steamcommunity.com/dev/apikey)

## Demarrage rapide

```bash
# 1. Cloner le projet
git clone <url-du-repo>
cd dev-env

# 2. Configurer l'environnement
cp .env.example .env
# Editer .env : changer les mots de passe, ajouter la cle Steam

# 3. Demarrer tous les services
docker compose up -d

# 4. Verifier le statut
docker compose ps

# 5. Appliquer les migrations (premiere fois)
docker exec -it dev_rust sqlx migrate run
```

L'application est accessible sur **http://localhost:3000**

## Services et ports

| Service | URL / Port | Description |
|---------|-----------|-------------|
| Achievement Tracker | http://localhost:3000 | App Rust (API + Frontend Leptos) |
| Node.js (legacy) | http://localhost:3100 | Frontend Node.js (transition) |
| PostgreSQL | localhost:5432 | Base de donnees |
| pgAdmin | http://localhost:5050 | Interface web PostgreSQL |

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  Navigateur                      │
│              (WASM + Hydration)                  │
└─────────────────┬───────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────┐
│           Actix-web (port 3000)                  │
│  ┌────────────────┐  ┌────────────────────────┐  │
│  │  API REST       │  │  Leptos SSR            │  │
│  │  /api/auth/*    │  │  Pages HTML + WASM     │  │
│  │  /api/games/*   │  │  Hydration client      │  │
│  │  /api/platforms/*│  │                        │  │
│  │  /api/achieve/* │  │                        │  │
│  └───────┬────────┘  └────────────────────────┘  │
│          │                                        │
│  ┌───────▼────────────────────────────────────┐  │
│  │  Platform Clients                           │  │
│  │  ├── Steam (API REST)                       │  │
│  │  ├── GOG (experimental)                     │  │
│  │  └── Epic (stub)                            │  │
│  └───────┬────────────────────────────────────┘  │
└──────────┼───────────────────────────────────────┘
           │
┌──────────▼───────────────────────────────────────┐
│              PostgreSQL 16                        │
│  users | platform_connections | games             │
│  game_platform_ids | achievements                 │
│  user_achievements                                │
└──────────────────────────────────────────────────┘
```

## API REST

### Authentification
| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/auth/register` | Inscription |
| POST | `/api/auth/login` | Connexion (retourne JWT) |
| GET | `/api/auth/me` | Utilisateur courant |

### Plateformes
| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/platforms` | Lister les connexions |
| POST | `/api/platforms/{platform}` | Lier un compte |
| DELETE | `/api/platforms/{platform}` | Delier un compte |
| POST | `/api/platforms/{platform}/sync` | Synchroniser |

### Jeux et Achievements
| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/games` | Lister les jeux (pagine) |
| GET | `/api/games/{id}` | Detail d'un jeu |
| GET | `/api/games/search?q=` | Recherche (fuzzy) |
| GET | `/api/games/{id}/achievements` | Achievements d'un jeu |
| GET | `/api/achievements/stats` | Statistiques globales |
| GET | `/api/achievements/recent` | Recemment debloques |

## Schema base de donnees

```
users
├── id (UUID, PK)
├── email (unique)
├── username
├── password_hash
├── display_name
├── avatar_url
├── is_active
└── created_at, updated_at

platform_connections
├── id (UUID, PK)
├── user_id → users.id
├── platform (steam | gog | epic)
├── platform_user_id
├── platform_username
├── access_token, refresh_token
├── last_synced_at
└── created_at, updated_at

games
├── id (UUID, PK)
├── title
├── normalized_title (index trigram)
├── cover_image_url
└── created_at

game_platform_ids
├── id (UUID, PK)
├── game_id → games.id
├── platform (steam | gog | epic)
├── platform_game_id (unique par plateforme)
├── platform_name
└── total_achievements

achievements
├── id (UUID, PK)
├── game_platform_id → game_platform_ids.id
├── platform_achievement_id
├── name, description
├── icon_url
├── is_hidden
└── global_unlock_pct

user_achievements
├── id (UUID, PK)
├── user_id → users.id
├── achievement_id → achievements.id
├── unlocked_at
├── is_unlocked
└── synced_at
```

## Structure du projet

```
dev-env/
├── docker-compose.yml          # Orchestration des services
├── .env                        # Variables d'environnement (secret)
├── .env.example                # Template a copier
├── README.md
├── rust-app/
│   ├── Cargo.toml              # Dependances Rust + config Leptos
│   ├── Dockerfile.dev          # Image Docker dev (Leptos + WASM)
│   ├── rust-toolchain.toml     # Version Rust + target WASM
│   ├── migrations/             # Migrations SQLx
│   ├── assets/                 # Fichiers statiques
│   ├── style/main.scss         # Styles SCSS
│   └── src/
│       ├── main.rs             # Point d'entree serveur
│       ├── lib.rs              # Library (hydration WASM)
│       ├── app.rs              # Composant racine + router
│       ├── models/             # Structures de donnees
│       ├── server/             # Code serveur uniquement
│       │   ├── auth.rs         # JWT + Argon2
│       │   ├── db.rs           # Pool PostgreSQL
│       │   ├── api/            # Handlers REST
│       │   └── platforms/      # Clients APIs gaming
│       └── pages/              # Composants Leptos
│           ├── landing.rs
│           ├── dashboard.rs
│           ├── login.rs
│           └── components/     # Composants reutilisables
├── node-app/                   # Frontend Node.js (transition)
└── postgres/
    └── init/01_init.sql        # Schema initial
```

## Developpement

```bash
# Voir les logs en temps reel
docker compose logs -f rust-app

# Ouvrir un shell dans le conteneur Rust
docker exec -it dev_rust bash

# Lancer les migrations manuellement
docker exec -it dev_rust sqlx migrate run

# Reconstruire l'image apres changement du Dockerfile
docker compose up -d --build rust-app

# Arreter tous les services
docker compose down

# Tout reinitialiser (supprime les donnees)
docker compose down -v
```

## Limitations connues

- **GOG** : API non officielle (reverse-engineered), peut changer sans preavis. Marque comme "experimental".
- **Epic Games** : Acces aux donnees d'achievements restreint depuis janvier 2025. Necessite le SDK EOS. Implementation stub fournie.
- **Matching cross-plateforme** : Le rapprochement des jeux entre plateformes utilise la similarite de titre (pg_trgm). Peut generer des faux positifs/negatifs.

## Connexion pgAdmin

1. Ouvrir http://localhost:5050
2. Email / Mot de passe : voir `.env`
3. Ajouter un serveur :
   - Host : `postgres`
   - Port : `5432`
   - Database / Username / Password : voir `.env`
