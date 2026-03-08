-- ─── Jaquettes Sega 32X — source : thumbnails.libretro.com ───────────────────
-- Seules les versions finales (hors Beta/Proto) sont conservées, une par région.
-- Les jeux CD-32X (Corpse Killer, Night Trap…) n'ont pas de jaquette disponible
-- sur cette source et sont donc ignorés.

INSERT INTO retro_game_covers (game_id, region, url)
SELECT rg.id, c.region, c.url
FROM retro_consoles rc
JOIN retro_games rg ON rg.console_id = rc.id
JOIN (VALUES
    -- ── After Burner Complete ────────────────────────────────────────────────
    ('After Burner Complete',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/After%20Burner%20Complete%20(Europe).png'),
    ('After Burner Complete',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/After%20Burner%20Complete%20(Japan%2C%20USA)%20(En).png'),
    -- ── BC Racers ────────────────────────────────────────────────────────────
    ('BC Racers',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/BC%20Racers%20(USA).png'),
    -- ── Blackthorne ──────────────────────────────────────────────────────────
    ('Blackthorne',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Blackthorne%20(USA).png'),
    -- ── Brutal: Paws of Fury Special Edition (titre 32X : Above the Claw) ───
    ('Brutal: Paws of Fury Special Edition',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Brutal%20-%20Above%20the%20Claw%20(USA).png'),
    -- ── Cosmic Carnage ───────────────────────────────────────────────────────
    ('Cosmic Carnage',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Cosmic%20Carnage%20(Europe).png'),
    ('Cosmic Carnage',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Cosmic%20Carnage%20(Japan%2C%20USA)%20(En).png'),
    -- ── Darxide ──────────────────────────────────────────────────────────────
    ('Darxide',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Darxide%20(Europe)%20(En%2CFr%2CDe%2CEs).png'),
    -- ── Doom ─────────────────────────────────────────────────────────────────
    ('Doom',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Doom%20(Europe).png'),
    ('Doom',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Doom%20(Japan%2C%20USA)%20(En).png'),
    -- ── FIFA Soccer 96 ───────────────────────────────────────────────────────
    ('FIFA Soccer 96',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/FIFA%20Soccer%2096%20(Europe)%20(En%2CFr%2CDe%2CEs%2CIt%2CSv).png'),
    -- ── Golf Magazine: 36 Holes Starring Fred Couples ────────────────────────
    ('Golf Magazine: 36 Holes Starring Fred Couples',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Golf%20Magazine%20Presents%20-%2036%20Great%20Holes%20Starring%20Fred%20Couples%20(Europe).png'),
    ('Golf Magazine: 36 Holes Starring Fred Couples',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Golf%20Magazine%20Presents%20-%2036%20Great%20Holes%20Starring%20Fred%20Couples%20(Japan%2C%20USA)%20(En).png'),
    -- ── Knuckles' Chaotix ────────────────────────────────────────────────────
    ('Knuckles'' Chaotix',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Knuckles%27%20Chaotix%20(Europe).png'),
    ('Knuckles'' Chaotix',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Knuckles%27%20Chaotix%20(Japan%2C%20USA)%20(En).png'),
    -- ── Kolibri ──────────────────────────────────────────────────────────────
    ('Kolibri',
     'USA / Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Kolibri%20(USA%2C%20Europe).png'),
    -- ── Metal Head ───────────────────────────────────────────────────────────
    ('Metal Head',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Metal%20Head%20(Europe)%20(En%2CJa).png'),
    ('Metal Head',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Metal%20Head%20(Japan%2C%20USA)%20(En%2CJa).png'),
    -- ── Mortal Kombat II ─────────────────────────────────────────────────────
    ('Mortal Kombat II',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Mortal%20Kombat%20II%20(Europe).png'),
    ('Mortal Kombat II',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Mortal%20Kombat%20II%20(Japan%2C%20USA)%20(En).png'),
    -- ── Motocross Championship ───────────────────────────────────────────────
    ('Motocross Championship',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Motocross%20Championship%20(Europe).png'),
    ('Motocross Championship',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Motocross%20Championship%20(USA).png'),
    -- ── NBA Jam Tournament Edition ───────────────────────────────────────────
    ('NBA Jam Tournament Edition',
     'World',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/NBA%20Jam%20-%20Tournament%20Edition%20(World).png'),
    -- ── NFL Quarterback Club ─────────────────────────────────────────────────
    ('NFL Quarterback Club',
     'World',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/NFL%20Quarterback%20Club%20(World).png'),
    -- ── Pitfall: The Mayan Adventure ─────────────────────────────────────────
    ('Pitfall: The Mayan Adventure',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Pitfall%20-%20The%20Mayan%20Adventure%20(USA).png'),
    -- ── Primal Rage ──────────────────────────────────────────────────────────
    ('Primal Rage',
     'USA / Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Primal%20Rage%20(USA%2C%20Europe).png'),
    -- ── RBI Baseball '95 ─────────────────────────────────────────────────────
    ('RBI Baseball ''95',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/RBI%20Baseball%20%2795%20(USA).png'),
    -- ── Sangokushi IV ────────────────────────────────────────────────────────
    ('Sangokushi IV',
     'Japan',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Sangokushi%20IV%20(Japan).png'),
    -- ── Shadow Squadron (= Stellar Assault hors Japon) ───────────────────────
    ('Shadow Squadron',
     'Japan',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Stellar%20Assault%20(Japan).png'),
    ('Shadow Squadron',
     'USA / Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Stellar%20Assault%20(USA%2C%20Europe).png'),
    -- ── Space Harrier ────────────────────────────────────────────────────────
    ('Space Harrier',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Space%20Harrier%20(Europe).png'),
    ('Space Harrier',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Space%20Harrier%20(Japan%2C%20USA)%20(En).png'),
    -- ── Spider-Man: Web of Fire ───────────────────────────────────────────────
    ('Spider-Man: Web of Fire',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Amazing%20Spider-Man%2C%20The%20-%20Web%20of%20Fire%20(USA).png'),
    -- ── Star Trek: Starfleet Academy ─────────────────────────────────────────
    ('Star Trek: Starfleet Academy',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Star%20Trek%20-%20Starfleet%20Academy%20-%20Starship%20Bridge%20Simulator%20(USA).png'),
    -- ── Star Wars Arcade ─────────────────────────────────────────────────────
    ('Star Wars Arcade',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Star%20Wars%20Arcade%20(Europe).png'),
    ('Star Wars Arcade',
     'Japan',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Star%20Wars%20Arcade%20(Japan).png'),
    ('Star Wars Arcade',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Star%20Wars%20Arcade%20(USA).png'),
    -- ── T-Mek ────────────────────────────────────────────────────────────────
    ('T-Mek',
     'USA / Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/T-MEK%20(USA%2C%20Europe).png'),
    -- ── Tempo ────────────────────────────────────────────────────────────────
    ('Tempo',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Tempo%20(Japan%2C%20USA)%20(En).png'),
    -- ── Toughman Contest ─────────────────────────────────────────────────────
    ('Toughman Contest',
     'USA / Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Toughman%20Contest%20(USA%2C%20Europe).png'),
    -- ── Virtua Fighter ───────────────────────────────────────────────────────
    ('Virtua Fighter',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Fighter%20(Europe).png'),
    ('Virtua Fighter',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Fighter%20(Japan%2C%20USA)%20(En).png'),
    -- ── Virtua Racing Deluxe ─────────────────────────────────────────────────
    ('Virtua Racing Deluxe',
     'Europe',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Racing%20Deluxe%20(Europe).png'),
    ('Virtua Racing Deluxe',
     'Japan',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Racing%20Deluxe%20(Japan).png'),
    ('Virtua Racing Deluxe',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Virtua%20Racing%20Deluxe%20(USA).png'),
    -- ── World Series Baseball '95 ─────────────────────────────────────────────
    ('World Series Baseball ''95',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/World%20Series%20Baseball%20Starring%20Deion%20Sanders%20(USA).png'),
    -- ── Wrestlemania: The Arcade Game ────────────────────────────────────────
    ('Wrestlemania: The Arcade Game',
     'USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/WWF%20WrestleMania%20-%20The%20Arcade%20Game%20(USA).png'),
    -- ── Zaxxon's Motherbase 2000 ──────────────────────────────────────────────
    ('Zaxxon''s Motherbase 2000',
     'Japan / USA',
     'https://thumbnails.libretro.com/Sega%20-%2032X/Named_Boxarts/Zaxxon%27s%20Motherbase%202000%20(Japan%2C%20USA)%20(En).png')
) AS c(game_name, region, url) ON rg.name = c.game_name
WHERE rc.name = '32X'
ON CONFLICT (game_id, region) DO UPDATE SET url = EXCLUDED.url;