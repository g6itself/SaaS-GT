# Déploiement Production — VPS

## Prérequis

- VPS sous Ubuntu 22.04/24.04 avec Docker + Docker Compose v2
- Un nom de domaine pointant vers l'IP du VPS

---

## 1. Préparer le serveur

```bash
# Installer Docker
curl -fsSL https://get.docker.com | sh
usermod -aG docker $USER

# Créer le réseau proxy externe
docker network create proxy_net
```

---

## 2. Déployer le code

```bash
git clone https://github.com/g6itself/SaaS-GT.git /opt/glorioustrophee
cd /opt/glorioustrophee
```

---

## 3. Configurer les variables d'environnement

```bash
cp .env.prod.example .env.prod
nano .env.prod
```

Valeurs à générer :
```bash
# JWT_SECRET (128 caractères hex)
openssl rand -hex 64

# ENCRYPTION_KEY (64 caractères hex)
openssl rand -hex 32

# POSTGRES_PASSWORD
openssl rand -base64 32
```

Remplacer `VOTRE_DOMAINE` dans `.env.prod` et dans `nginx/nginx.prod.conf`.

---

## 4. Obtenir le certificat SSL (Let's Encrypt)

### Premier démarrage sans SSL pour obtenir le certificat

```bash
# Démarrer Nginx en HTTP uniquement (commenter le bloc HTTPS dans nginx.prod.conf)
# puis lancer uniquement certbot :
docker compose -f docker-compose.prod.yml run --rm certbot \
  certbot certonly --webroot \
  -w /var/www/certbot \
  -d VOTRE_DOMAINE \
  -d www.VOTRE_DOMAINE \
  --email votre@email.com \
  --agree-tos \
  --no-eff-email
```

### Après obtention du certificat, décommenter le bloc HTTPS dans `nginx/nginx.prod.conf`

---

## 5. Lancer la stack complète

```bash
docker compose -f docker-compose.prod.yml up -d --build
```

Vérifier que tout tourne :
```bash
docker compose -f docker-compose.prod.yml ps
docker compose -f docker-compose.prod.yml logs -f
```

---

## 6. Appliquer les migrations

```bash
docker exec prod_postgres psql -U $POSTGRES_USER -d $POSTGRES_DB \
  -f /docker-entrypoint-initdb.d/schema.sql
```

Ou via sqlx-cli depuis le conteneur Rust (uniquement disponible en dev).

---

## Opérations courantes

### Mettre à jour l'application
```bash
git pull
docker compose -f docker-compose.prod.yml up -d --build rust-app node-app
```

### Voir les logs
```bash
docker compose -f docker-compose.prod.yml logs -f node-app
docker compose -f docker-compose.prod.yml logs -f rust-app
```

### Backup base de données
```bash
docker exec prod_postgres pg_dump -U $POSTGRES_USER $POSTGRES_DB \
  | gzip > backup_$(date +%Y%m%d_%H%M%S).sql.gz
```

### Renouvellement SSL (automatique via certbot, forcer manuellement)
```bash
docker exec prod_certbot certbot renew --force-renewal
docker compose -f docker-compose.prod.yml exec nginx nginx -s reload
```

---

## Structure des fichiers prod

```
/
├── docker-compose.prod.yml
├── .env.prod                    (jamais dans git)
├── .env.prod.example
├── nginx/
│   ├── nginx.prod.conf          (remplacer VOTRE_DOMAINE)
│   └── ssl/                     (certificats Let's Encrypt — montés en volume)
├── postgres/
│   └── postgresql.prod.conf
├── rust-app/Dockerfile.prod
└── node-app/Dockerfile.prod
```
