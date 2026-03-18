# Déploiement Production — VPS AlmaLinux

## Prérequis

- VPS sous AlmaLinux 8/9
- Un nom de domaine pointant vers l'IP du VPS

---

## 1. Préparer le serveur

```bash
# Mise à jour système
dnf update -y

# Installer les outils de base
dnf install -y git curl openssl

# Installer Docker (dépôt officiel)
dnf install -y dnf-plugins-core
dnf config-manager --add-repo https://download.docker.com/linux/rhel/docker-ce.repo
dnf install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
systemctl enable --now docker

# Ajouter ton utilisateur au groupe docker (se déconnecter/reconnecter après)
usermod -aG docker $USER

# Ouvrir les ports HTTP et HTTPS dans firewalld
firewall-cmd --permanent --add-service=http
firewall-cmd --permanent --add-service=https
firewall-cmd --reload

# Créer le réseau Docker externe pour Nginx
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

Générer les secrets :
```bash
# JWT_SECRET (128 caractères hex)
openssl rand -hex 64

# ENCRYPTION_KEY (64 caractères hex — DOIT faire exactement 64 chars)
openssl rand -hex 32

# POSTGRES_PASSWORD
openssl rand -base64 32
```

Remplacer aussi `VOTRE_DOMAINE` dans `nginx/nginx.prod.conf`.

---

## 4. Obtenir le certificat SSL (Let's Encrypt)

Le processus se fait en deux temps : d'abord obtenir le certificat en HTTP, puis activer HTTPS.

### Étape 4a — Nginx HTTP uniquement (temporaire)

Dans `nginx/nginx.prod.conf`, commenter le bloc `server` HTTPS (tout le second bloc `server { listen 443... }`) et démarrer uniquement nginx + certbot :

```bash
docker compose -f docker-compose.prod.yml up -d nginx certbot
```

### Étape 4b — Obtenir le certificat

```bash
docker compose -f docker-compose.prod.yml run --rm certbot \
  certbot certonly --webroot \
  -w /var/www/certbot \
  -d VOTRE_DOMAINE \
  -d www.VOTRE_DOMAINE \
  --email votre@email.com \
  --agree-tos \
  --no-eff-email
```

### Étape 4c — Activer HTTPS

Décommenter le bloc HTTPS dans `nginx/nginx.prod.conf`, puis recharger Nginx :

```bash
docker compose -f docker-compose.prod.yml exec nginx nginx -s reload
```

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

## 6. Appliquer les migrations SQL

Les migrations sont jouées automatiquement au premier démarrage via `postgres/init/`.
Si tu dois les rejouer manuellement :

```bash
# Lister les fichiers de migration
ls rust-app/migrations/

# Appliquer un fichier spécifique
docker exec -i prod_postgres psql \
  -U $(grep POSTGRES_USER .env.prod | cut -d= -f2) \
  -d $(grep POSTGRES_DB .env.prod | cut -d= -f2) \
  < rust-app/migrations/NOM_DU_FICHIER.sql
```

---

## Opérations courantes

### Mettre à jour l'application
```bash
cd /opt/glorioustrophee
git pull
docker compose -f docker-compose.prod.yml up -d --build rust-app node-app
```

### Voir les logs
```bash
docker compose -f docker-compose.prod.yml logs -f node-app
docker compose -f docker-compose.prod.yml logs -f rust-app
docker compose -f docker-compose.prod.yml logs -f nginx
```

### Backup base de données
```bash
source .env.prod
docker exec prod_postgres pg_dump -U $POSTGRES_USER $POSTGRES_DB \
  | gzip > /opt/backups/backup_$(date +%Y%m%d_%H%M%S).sql.gz
```

Automatiser avec cron :
```bash
mkdir -p /opt/backups
crontab -e
# Ajouter : backup quotidien à 3h du matin
0 3 * * * source /opt/glorioustrophee/.env.prod && docker exec prod_postgres pg_dump -U $POSTGRES_USER $POSTGRES_DB | gzip > /opt/backups/backup_$(date +\%Y\%m\%d).sql.gz
```

### Renouvellement SSL (automatique, forcer manuellement si besoin)
```bash
docker exec prod_certbot certbot renew --force-renewal
docker compose -f docker-compose.prod.yml exec nginx nginx -s reload
```

### Redémarrer un service
```bash
docker compose -f docker-compose.prod.yml restart rust-app
```

---

## Structure des fichiers prod

```
/opt/glorioustrophee/
├── docker-compose.prod.yml
├── .env.prod                    (jamais dans git)
├── .env.prod.example
├── DEPLOY.md                    (ce fichier)
├── nginx/
│   ├── nginx.prod.conf          (remplacer VOTRE_DOMAINE)
│   └── ssl/                     (certificats Let's Encrypt — volume Docker)
├── postgres/
│   ├── init/                    (scripts SQL exécutés au premier démarrage)
│   └── postgresql.prod.conf
├── rust-app/
│   ├── Dockerfile.prod
│   └── migrations/
└── node-app/
    └── Dockerfile.prod
```

---

## Dépannage AlmaLinux

### SELinux bloque Docker
```bash
# Vérifier si SELinux est en cause
ausearch -m avc -ts recent

# Autoriser Docker à accéder aux volumes montés
setsebool -P container_manage_cgroup on
chcon -Rt svirt_sandbox_file_t /opt/glorioustrophee
```

### Port 80/443 non accessible malgré firewalld
```bash
# Vérifier les règles actives
firewall-cmd --list-all

# Si le problème persiste, vérifier que le panneau VPS ne bloque pas ces ports
# (règles réseau côté hôte, indépendantes de firewalld)
```
