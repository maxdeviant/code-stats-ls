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

        Ok(())
    }

    pub fn remove(&self, pulse: &Pulse) -> Result<()> {
        let mut tx = self.env.write_txn()?;
        self.database.delete(&mut tx, &pulse.coded_at)?;

        Ok(())
    }
}
