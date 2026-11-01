use std::fs::File;
use std::io::{BufReader, BufWriter};

use crate::world::World;

pub const SAVE_PATH: &str = "cogworks_save.json";

pub fn save(world: &mut World, path: &str) -> Result<(), String> {
    world.sync_pipes();
    let file = File::create(path).map_err(|e| format!("Save failed: {e}"))?;
    serde_json::to_writer(BufWriter::new(file), world).map_err(|e| format!("Save failed: {e}"))
}

pub fn load(path: &str) -> Result<World, String> {
    let file = File::open(path).map_err(|e| format!("Load failed: {e}"))?;
    let mut world: World =
        serde_json::from_reader(BufReader::new(file)).map_err(|e| format!("Load failed: {e}"))?;
    world.after_load();
    Ok(world)
}
