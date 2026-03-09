pub mod steam;
pub mod gog;
pub mod epic;

/// Statistiques de synchronisation
#[derive(Debug)]
pub struct SyncStats {
    pub games_synced: u32,
    pub achievements_synced: u32,
    pub total_achievements: u32,
    pub games_completed: u32,
}
