-- ─── Données de développement : 50 comptes fictifs ───────────────────────────
-- Comptes de test pour démonstrations UI et développement.
-- ATTENTION : le password_hash est un placeholder non fonctionnel.
-- Ces comptes ne peuvent pas se connecter. Pour tester l'auth, utilisez /api/auth/register.
-- ─────────────────────────────────────────────────────────────────────────────

-- Hash Argon2id placeholder (syntaxe valide, ne vérifie aucun mot de passe)
DO $$
DECLARE
    v_hash TEXT := '$argon2id$v=19$m=19456,t=2,p=1$c2VlZHNhbHRzZWVkc2E$YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE';
BEGIN

-- ── Insertion des 50 utilisateurs fictifs ────────────────────────────────────
-- league doit être défini explicitement (le trigger sync ne s'exécute que sur UPDATE)

INSERT INTO users (email, username, display_name, password_hash, total_points, league, active_title, last_login_at) VALUES

-- ── Quasar (600 000 – 999 999 pts) ──────────────────────────────────────────
('crimsonapex@glorious.dev',   'CrimsonApex',    'Crimson Apex',      v_hash, 980000, 'Quasar',     'Singularité',        NOW() - INTERVAL '2 hours'),

-- ── Apex (300 000 – 599 999 pts) ────────────────────────────────────────────
('voidslayer@glorious.dev',    'VoidSlayer',     'Void Slayer',       v_hash, 450000, 'Apex',       'Dieu des Succès',    NOW() - INTERVAL '6 hours'),

-- ── Supernova (150 000 – 299 999 pts) ───────────────────────────────────────
('nightphoenix@glorious.dev',  'NightPhoenix',   'Night Phoenix',     v_hash, 220000, 'Supernova',  'Supernova',          NOW() - INTERVAL '1 day'),

-- ── Eclipse (75 000 – 149 999 pts) ──────────────────────────────────────────
('starlightx@glorious.dev',    'StarlightX',     'Starlight X',       v_hash, 125000, 'Eclipse',    'Fantôme Numérique',  NOW() - INTERVAL '3 hours'),
('darkmetterz@glorious.dev',   'DarkMatterZ',    'Dark Matter Z',     v_hash,  95000, 'Eclipse',    'Fantôme Numérique',  NOW() - INTERVAL '2 days'),

-- ── Zenith (35 000 – 74 999 pts) ────────────────────────────────────────────
('thunderking@glorious.dev',   'ThunderKing',    'Thunder King',      v_hash,  68000, 'Zenith',     'Légende',            NOW() - INTERVAL '5 hours'),
('silvernova@glorious.dev',    'SilverNova',     'Silver Nova',       v_hash,  55000, 'Zenith',     'Légende',            NOW() - INTERVAL '1 day'),
('shadowforge@glorious.dev',   'ShadowForge',    'Shadow Forge',      v_hash,  42000, 'Zenith',     'Légende',            NOW() - INTERVAL '3 days'),
('frostknight@glorious.dev',   'FrostKnight',    'Frost Knight',      v_hash,  38000, 'Zenith',     'Légende',            NOW() - INTERVAL '4 hours'),

-- ── Pulsar (15 000 – 34 999 pts) ────────────────────────────────────────────
('ironwarden@glorious.dev',    'IronWarden',     'Iron Warden',       v_hash,  33000, 'Pulsar',     'Chasseur d''Élite',  NOW() - INTERVAL '2 days'),
('quantumpulse@glorious.dev',  'QuantumPulse',   'Quantum Pulse',     v_hash,  28000, 'Pulsar',     'Chasseur d''Élite',  NOW() - INTERVAL '6 hours'),
('titancrest@glorious.dev',    'TitanCrest',     'Titan Crest',       v_hash,  24000, 'Pulsar',     'Chasseur d''Élite',  NOW() - INTERVAL '1 day'),
('omegarune@glorious.dev',     'OmegaRune',      'Omega Rune',        v_hash,  20000, 'Pulsar',     'Chasseur d''Élite',  NOW() - INTERVAL '8 hours'),
('nebuladrift@glorious.dev',   'NebulaDrift',    'Nebula Drift',      v_hash,  17000, 'Pulsar',     'Chasseur d''Élite',  NOW() - INTERVAL '3 days'),
('vortexblade@glorious.dev',   'VortexBlade',    'Vortex Blade',      v_hash,  15500, 'Pulsar',     'Chasseur d''Élite',  NOW() - INTERVAL '12 hours'),

-- ── Nebula (5 000 – 14 999 pts) ─────────────────────────────────────────────
('astralagm@glorious.dev',     'AstralVoid',     'Astral Void',       v_hash,  14000, 'Nebula',     'Collectionneur',     NOW() - INTERVAL '2 days'),
('mysticstorm@glorious.dev',   'MysticStorm',    'Mystic Storm',      v_hash,  12500, 'Nebula',     'Collectionneur',     NOW() - INTERVAL '5 hours'),
('cyberhunter@glorious.dev',   'CyberHunter',    'Cyber Hunter',      v_hash,  10800, 'Nebula',     'Collectionneur',     NOW() - INTERVAL '1 day'),
('ghostproto@glorious.dev',    'GhostProtocol',  'Ghost Protocol',    v_hash,   9200, 'Nebula',     'Collectionneur',     NOW() - INTERVAL '7 hours'),
('blazingcomet@glorious.dev',  'BlazingComet',   'Blazing Comet',     v_hash,   7800, 'Nebula',     'Collectionneur',     NOW() - INTERVAL '4 days'),
('crystalshard@glorious.dev',  'CrystalShard',   'Crystal Shard',     v_hash,   6400, 'Nebula',     'Collectionneur',     NOW() - INTERVAL '2 days'),
('pixelcrush@glorious.dev',    'PixelCrush',     'Pixel Crush',       v_hash,   5500, 'Nebula',     'Explorateur',        NOW() - INTERVAL '3 days'),
('neonrogue@glorious.dev',     'NeonRogue',      'Neon Rogue',        v_hash,   5100, 'Nebula',     'Explorateur',        NOW() - INTERVAL '6 hours'),

-- ── Horizon (1 000 – 4 999 pts) ─────────────────────────────────────────────
('emberflare@glorious.dev',    'EmberFlare',     'Ember Flare',       v_hash,   4700, 'Horizon',    'Explorateur',        NOW() - INTERVAL '1 day'),
('steeledge@glorious.dev',     'SteelEdge',      'Steel Edge',        v_hash,   4200, 'Horizon',    'Explorateur',        NOW() - INTERVAL '5 hours'),
('runewalker@glorious.dev',    'RuneWalker',     'Rune Walker',       v_hash,   3800, 'Horizon',    'Explorateur',        NOW() - INTERVAL '3 days'),
('lunarbyte@glorious.dev',     'LunarByte',      'Lunar Byte',        v_hash,   3300, 'Horizon',    'Explorateur',        NOW() - INTERVAL '2 days'),
('icebreaker@glorious.dev',    'IceBreaker',     'Ice Breaker',       v_hash,   2900, 'Horizon',    'Explorateur',        NOW() - INTERVAL '8 hours'),
('starforgex@glorious.dev',    'StarForgeX',     'Star Forge X',      v_hash,   2500, 'Horizon',    'Explorateur',        NOW() - INTERVAL '1 day'),
('cosmicdust@glorious.dev',    'CosmicDust',     'Cosmic Dust',       v_hash,   2100, 'Horizon',    'Explorateur',        NOW() - INTERVAL '4 days'),
('turbofalcon@glorious.dev',   'TurboFalcon',    'Turbo Falcon',      v_hash,   1800, 'Horizon',    'Chasseur de Trophées', NOW() - INTERVAL '2 days'),
('ultrashift@glorious.dev',    'UltraShift',     'Ultra Shift',       v_hash,   1500, 'Horizon',    'Chasseur de Trophées', NOW() - INTERVAL '6 hours'),
('hexcodedev@glorious.dev',    'HexCode',        'Hex Code',          v_hash,   1250, 'Horizon',    'Chasseur de Trophées', NOW() - INTERVAL '3 days'),
('zerogravity@glorious.dev',   'ZeroGravity',    'Zero Gravity',      v_hash,   1100, 'Horizon',    'Chasseur de Trophées', NOW() - INTERVAL '5 days'),
('redvector@glorious.dev',     'RedVector',      'Red Vector',        v_hash,   1050, 'Horizon',    'Chasseur de Trophées', NOW() - INTERVAL '10 hours'),

-- ── Orbit (0 – 999 pts) ─────────────────────────────────────────────────────
('voltstrike@glorious.dev',    'VoltStrike',     'Volt Strike',       v_hash,    990, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '1 day'),
('nightowl99@glorious.dev',    'NightOwl99',     'Night Owl',         v_hash,    850, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '2 days'),
('solarflux@glorious.dev',     'SolarFlux',      'Solar Flux',        v_hash,    720, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '3 days'),
('darkpulse@glorious.dev',     'DarkPulse',      'Dark Pulse',        v_hash,    600, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '5 hours'),
('glitchmstr@glorious.dev',    'GlitchMstr',     'Glitch Master',     v_hash,    500, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '4 days'),
('coldfusion@glorious.dev',    'ColdFusion',     'Cold Fusion',       v_hash,    420, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '2 days'),
('byterunner@glorious.dev',    'ByteRunner',     'Byte Runner',       v_hash,    350, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '6 days'),
('plasmacut@glorious.dev',     'PlasmaCut',      'Plasma Cut',        v_hash,    280, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '3 days'),
('irondrift@glorious.dev',     'IronDrift',      'Iron Drift',        v_hash,    200, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '7 days'),
('shadowstep@glorious.dev',    'ShadowStep',     'Shadow Step',       v_hash,    150, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '5 days'),
('nightcore@glorious.dev',     'NightCore',      'Night Core',        v_hash,    100, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '4 days'),
('digitalnomad@glorious.dev',  'DigitalNomad',   'Digital Nomad',     v_hash,     75, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '8 days'),
('starterpack@glorious.dev',   'StarterPack',    'Starter Pack',      v_hash,     50, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '10 days'),
('newhorizon1@glorious.dev',   'NewHorizon1',    'New Horizon',       v_hash,     25, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '12 days'),
('gloriousnew@glorious.dev',   'GloriousNew',    'Glorious New',      v_hash,      0, 'Orbit',      'Chasseur de Trophées', NOW() - INTERVAL '15 days')

ON CONFLICT (email) DO NOTHING;

-- ── Peuplement du leaderboard_cache ─────────────────────────────────────────
-- Estimation du nombre de succès : total_points / 100
-- Complétion estimée par tranche de ligue

INSERT INTO leaderboard_cache (user_id, rank_global, total_points, total_achievements, completion_avg, league, refreshed_at)
SELECT
    u.id,
    ROW_NUMBER() OVER (ORDER BY u.total_points DESC)::INT,
    u.total_points,
    (u.total_points / 100)::INT AS total_achievements,
    CASE
        WHEN u.total_points >= 600000 THEN 92.50
        WHEN u.total_points >= 300000 THEN 87.30
        WHEN u.total_points >= 150000 THEN 81.70
        WHEN u.total_points >= 75000  THEN 71.55
        WHEN u.total_points >= 35000  THEN 58.15
        WHEN u.total_points >= 15000  THEN 40.35
        WHEN u.total_points >= 5000   THEN 24.68
        WHEN u.total_points >= 1000   THEN 10.40
        ELSE 2.15
    END::NUMERIC(5,2),
    u.league,
    NOW()
FROM users u
WHERE u.email LIKE '%@glorious.dev'
ON CONFLICT (user_id) DO UPDATE SET
    rank_global        = EXCLUDED.rank_global,
    total_points       = EXCLUDED.total_points,
    total_achievements = EXCLUDED.total_achievements,
    completion_avg     = EXCLUDED.completion_avg,
    league             = EXCLUDED.league,
    refreshed_at       = NOW();

END $$;
