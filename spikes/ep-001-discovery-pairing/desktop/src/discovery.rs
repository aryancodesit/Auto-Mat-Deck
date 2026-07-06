use log::info;
use mdns_sd::{ServiceDaemon, ServiceInfo};

pub trait AdvertisementProvider: Send + Sync {
    fn start(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn stop(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn device_id(&self) -> &str;
    fn provider_name(&self) -> &'static str;
}

pub struct MdnsAnnouncer {
    daemon: Option<ServiceDaemon>,
    device_id: String,
    hostname: String,
    port: u16,
}

impl MdnsAnnouncer {
    pub fn new(device_id: String, hostname: String, port: u16) -> Self {
        Self {
            daemon: None,
            device_id,
            hostname,
            port,
        }
    }
}

impl AdvertisementProvider for MdnsAnnouncer {
    fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mdns = ServiceDaemon::new()?;
        let props: Vec<(String, String)> = vec![
            ("deviceId".into(), self.device_id.clone()),
            ("deviceType".into(), "desktop".into()),
            ("protocolVersions".into(), "1.0".into()),
            ("os".into(), std::env::consts::OS.into()),
            ("provider".into(), "mdns".into()),
        ];

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            SERVICE_NAME,
            &format!("{}.local.", &self.hostname),
            "",
            self.port,
            &props[..],
        )?
        .enable_addr_auto();

        mdns.register(service_info)?;
        self.daemon = Some(mdns);
        info!(
            "[mDNS] Advertising {} as {}.{} on port {}",
            self.device_id, SERVICE_NAME, SERVICE_TYPE, self.port
        );
        Ok(())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref daemon) = self.daemon {
            daemon.shutdown()?;
            info!("[mDNS] Advertisement stopped");
        }
        Ok(())
    }

    fn device_id(&self) -> &str {
        &self.device_id
    }

    fn provider_name(&self) -> &'static str {
        "mDNS"
    }
}

const SERVICE_TYPE: &str = "_amd._tcp.local.";
const SERVICE_NAME: &str = "AutoMatDeckDesktop";
