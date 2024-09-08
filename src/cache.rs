use std::path::Path;

use anyhow::{Context, Result};
use heed::types::SerdeBincode;
use heed::{Database, EnvOpenOptions};

use crate::Pulse;

pub struct PulseCache {
    env: heed::Env,
    database: Database<SerdeBincode<String>, SerdeBincode<Pulse>>,
}

impl PulseCache {
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir().context("no data directory found")?;
        let database_path = data_dir.join("code-stats-ls/cache");

        Self::from_path(database_path)
    }

    fn from_path(database_path: impl AsRef<Path>) -> Result<Self> {
        std::fs::create_dir_all(&database_path)
            .context("failed to create cache database directory")?;

        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(100 * 1024 * 1024)
                .max_dbs(1)
                .open(database_path)
                .context("failed to open cache database")?
        };

        let mut tx = env.write_txn()?;
        let database = env.create_database(&mut tx, Some("cache"))?;
        tx.commit()?;

        Ok(Self { database, env })
    }

    pub fn list(&self) -> Result<Vec<Pulse>> {
        let tx = self.env.read_txn()?;
        let pulses = self
            .database
            .iter(&tx)?
            .filter_map(|entry| entry.ok().map(|(_, pulse)| pulse))
            .collect::<Vec<_>>();

        Ok(pulses)
    }

    pub fn save(&self, pulse: &Pulse) -> Result<()> {
        let mut tx = self.env.write_txn()?;
        self.database.put(&mut tx, &pulse.coded_at, pulse)?;
        tx.commit()?;

        Ok(())
    }

    pub fn remove(&self, pulse: &Pulse) -> Result<()> {
        let mut tx = self.env.write_txn()?;
        self.database.delete(&mut tx, &pulse.coded_at)?;
        tx.commit()?;

        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        let mut tx = self.env.write_txn()?;
        self.database.clear(&mut tx)?;
        tx.commit()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Local;

    use crate::PulseXp;

    use super::*;

    #[test]
    fn test_pulse_cache() {
        let pulse_cache = PulseCache::from_path("./test_database").unwrap();

        pulse_cache.clear().unwrap();

        let initial_pulses = pulse_cache.list().unwrap();
        assert_eq!(initial_pulses.len(), 0);

        let first_pulse = Pulse {
            coded_at: Local::now().to_rfc3339(),
            xps: vec![PulseXp {
                language: "Rust".into(),
                xp: 10,
            }],
        };

        pulse_cache.save(&first_pulse).unwrap();

        let pulses = pulse_cache.list().unwrap();
        assert_eq!(pulses.len(), 1);

        let second_pulse = Pulse {
            coded_at: Local::now().to_rfc3339(),
            xps: vec![PulseXp {
                language: "Gleam".into(),
                xp: 20,
            }],
        };

        pulse_cache.save(&second_pulse).unwrap();

        let pulses = pulse_cache.list().unwrap();
        assert_eq!(pulses.len(), 2);

        pulse_cache.remove(&second_pulse).unwrap();

        let pulses = pulse_cache.list().unwrap();
        assert_eq!(pulses.len(), 1);

        pulse_cache.clear().unwrap();

        let pulses = pulse_cache.list().unwrap();
        assert_eq!(pulses.len(), 0);
    }
}
