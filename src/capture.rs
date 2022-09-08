use std::path::PathBuf;

use log::{error, info};

#[cfg(feature = "packet_capture")]
pub(crate) fn start(device: &str, ports: Vec<u16>, path: PathBuf) -> Result<(), String> {
    let filter = ports
        .iter()
        .map(|p| format!("port {}", p))
        .collect::<Vec<String>>()
        .join(" or ");

    let mut cap = pcap::Capture::from_device(device)
        .map_err(|e| format!("can't create capture object for device {}: {:?}", device, e))?
        .immediate_mode(true)
        .open()
        .map_err(|e| format!("can't create capture object: {:?}", e))?;

    cap.filter(&filter, true)
        .map_err(|e| format!("can't set capture filter '{}': {:?}", &filter, e))?;

    let mut savefile = cap
        .savefile(&path)
        .map_err(|e| format!("can't set capture file: {:?}", e))?;

    tokio::spawn(async move {
        info!("packet capture started, writing to {} ...", path.display());
        while let Ok(packet) = cap.next_packet() {
            savefile.write(&packet);
            if let Err(e) = savefile.flush() {
                error!("error flushing packet capture file: {:?}", e);
            }
        }
    });

    Ok(())
}
