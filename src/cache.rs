use std::path::Path;

use anyhow::{Context, Result};
use heed::types::SerdeBincode;
use heed::{Database, EnvOpenOptions};

use crate::pulse::Pulse;

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

    #[cfg(test)]
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

    /// Removes up to `limit` of the oldest pulses from the cache and returns them.
    ///
    /// This happens in a single write transaction, so concurrent processes will
    /// never take the same pulse.
    pub fn take(&self, limit: usize) -> Result<Vec<Pulse>> {
        let mut tx = self.env.write_txn()?;
        let entries = self
            .database
            .iter(&tx)?
            .take(limit)
            .collect::<heed::Result<Vec<_>>>()?;

        for (key, _) in &entries {
            self.database.delete(&mut tx, key)?;
        }
        tx.commit()?;

        Ok(entries.into_iter().map(|(_, pulse)| pulse).collect())
    }

    #[cfg(test)]
    pub fn remove(&self, pulse: &Pulse) -> Result<()> {
        let mut tx = self.env.write_txn()?;
        self.database.delete(&mut tx, &pulse.coded_at)?;
        tx.commit()?;

        Ok(())
    }

    #[cfg(test)]
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

    use crate::pulse::PulseXp;

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

        pulse_cache.save(&second_pulse).unwrap();

        let taken = pulse_cache.take(1).unwrap();
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].coded_at, first_pulse.coded_at);

        let pulses = pulse_cache.list().unwrap();
        assert_eq!(pulses.len(), 1);
        assert_eq!(pulses[0].coded_at, second_pulse.coded_at);

        pulse_cache.clear().unwrap();

        let pulses = pulse_cache.list().unwrap();
        assert_eq!(pulses.len(), 0);
    }
}
