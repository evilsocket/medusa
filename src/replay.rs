use std::fs;

use glob::glob;
use log::info;

use crate::record::Record;

pub(crate) fn start(path: &str) -> Result<(), String> {
    info!("starting replay mode from {} ...", path);

    let mut records = vec![];

    for entry in glob(&format!("{}/**/*.json", path)).unwrap() {
        let entry = entry.unwrap();
        let raw = fs::read_to_string(&entry)
            .map_err(|e| format!("could not open {} for reading: {:?}", entry.display(), e))?;

        let record: Record = serde_json::from_str(&raw)
            .map_err(|e| format!("could not deserialize {}: {:?}", entry.display(), e))?;

        records.push(record);
    }

    records.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    for record in records {
        println!("{}", record);
    }

    Ok(())
}
