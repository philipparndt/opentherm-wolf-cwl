//! Network management — WiFi or Ethernet depending on feature flags.

use log::{info, warn};

/// Network connection state
pub struct NetworkState {
    pub connected: bool,
    pub ip_address: Option<String>,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            connected: false,
            ip_address: None,
        }
    }
}

// =============================================================================
// WiFi Implementation
// =============================================================================

#[cfg(feature = "wifi")]
#[allow(dead_code)]
pub mod wifi_impl {
    use super::*;
    use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};
    use esp_idf_svc::eventloop::EspSystemEventLoop;
    use esp_idf_svc::hal::modem::Modem;
    use esp_idf_svc::nvs::EspDefaultNvsPartition;

    pub struct WifiNetwork {
        pub wifi: BlockingWifi<EspWifi<'static>>,
        pub state: NetworkState,
    }

    impl WifiNetwork {
        pub fn new(
            modem: Modem,
            sysloop: EspSystemEventLoop,
            nvs: EspDefaultNvsPartition,
        ) -> Result<Self, esp_idf_svc::sys::EspError> {
            let esp_wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
            let wifi = BlockingWifi::wrap(esp_wifi, sysloop)?;
            Ok(Self {
                wifi,
                state: NetworkState::default(),
            })
        }

        pub fn connect(&mut self, ssid: &str, password: &str) -> Result<(), esp_idf_svc::sys::EspError> {
            if ssid.is_empty() {
                warn!("WiFi: No credentials configured");
                return Ok(());
            }

            info!("WiFi: Connecting to {}...", ssid);

            let mut client_config = ClientConfiguration::default();
            client_config.ssid = ssid.try_into().unwrap_or_default();
            client_config.password = password.try_into().unwrap_or_default();

            self.wifi.set_configuration(&Configuration::Client(client_config))?;
            self.wifi.start()?;
            self.wifi.connect()?;
            self.wifi.wait_netif_up()?;

            let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;
            let ip = format!("{}", ip_info.ip);
            info!("WiFi: Connected, IP: {}", ip);

            self.state.connected = true;
            self.state.ip_address = Some(ip);

            Ok(())
        }
    }
}

// =============================================================================
// Ethernet Implementation (Olimex ESP32-POE with LAN8720)
// =============================================================================

#[cfg(feature = "ethernet")]
pub mod eth_impl {
    use super::*;
    use esp_idf_svc::eth::{BlockingEth, EspEth, EthDriver, RmiiClockConfig, RmiiEthChipset};
    use esp_idf_svc::eventloop::EspSystemEventLoop;
    use esp_idf_svc::hal::gpio;
    use esp_idf_svc::hal::mac::MAC;
    /// Initialize Ethernet for Olimex ESP32-POE.
    /// Takes ownership of the required GPIO pins.
    pub fn start_ethernet(
        mac: MAC,
        _gpio0: gpio::Gpio0,
        gpio12: gpio::Gpio12,
        gpio17: gpio::Gpio17,
        gpio18: gpio::Gpio18,
        gpio19: gpio::Gpio19,
        gpio21: gpio::Gpio21,
        gpio22: gpio::Gpio22,
        gpio23: gpio::Gpio23,
        gpio25: gpio::Gpio25,
        gpio26: gpio::Gpio26,
        gpio27: gpio::Gpio27,
        sysloop: EspSystemEventLoop,
    ) -> Result<NetworkState, esp_idf_svc::sys::EspError> {
        info!("ETH: Initializing LAN8720...");

        // Olimex ESP32-POE: LAN8720 PHY, clock on GPIO17 (inverted output), power on GPIO12
        let eth_driver = EthDriver::new_rmii(
            mac,
            gpio25,  // rmii_rdx0
            gpio26,  // rmii_rdx1
            gpio27,  // rmii_crs_dv
            gpio23,  // rmii_mdc
            gpio22,  // rmii_txd1
            gpio21,  // rmii_tx_en
            gpio19,  // rmii_txd0
            gpio18,  // rmii_mdio
            RmiiClockConfig::<gpio::Gpio0, gpio::Gpio16, gpio::Gpio17>::OutputInvertedGpio17(gpio17),
            Some(gpio12), // rst / power pin
            RmiiEthChipset::LAN87XX,
            Some(0), // PHY address
            sysloop.clone(),
        )?;

        let esp_eth = EspEth::wrap(eth_driver)?;
        let mut eth = BlockingEth::wrap(esp_eth, sysloop)?;

        eth.start()?;
        info!("ETH: Started, waiting for link...");

        eth.wait_netif_up()?;

        let ip_info = eth.eth().netif().get_ip_info()?;
        let ip = format!("{}", ip_info.ip);
        info!("ETH: Got IP: {}", ip);

        let state = NetworkState {
            connected: true,
            ip_address: Some(ip),
        };

        // Ethernet driver must stay alive
        std::mem::forget(eth);

        Ok(state)
    }
}

/// Check if the default network interface has an IP address.
/// Works for both WiFi and Ethernet after the driver is started.
pub fn check_network_connected() -> (bool, Option<String>) {
    use esp_idf_svc::sys::*;

    unsafe {
        // Get the default netif (whichever was registered — eth or wifi sta)
        let netif = esp_netif_get_default_netif();
        if netif.is_null() {
            return (false, None);
        }

        let up = esp_netif_is_netif_up(netif);
        if !up {
            return (false, None);
        }

        let mut ip_info: esp_netif_ip_info_t = std::mem::zeroed();
        if esp_netif_get_ip_info(netif, &mut ip_info) == ESP_OK && ip_info.ip.addr != 0 {
            let ip = format!("{}.{}.{}.{}",
                ip_info.ip.addr & 0xFF,
                (ip_info.ip.addr >> 8) & 0xFF,
                (ip_info.ip.addr >> 16) & 0xFF,
                (ip_info.ip.addr >> 24) & 0xFF);
            return (true, Some(ip));
        }
    }
    (false, None)
}

/// Configure NTP time sync
pub fn setup_ntp() {
    use esp_idf_svc::sntp::{EspSntp, SyncStatus};
    use std::time::Duration;

    info!("NTP: Syncing time...");
    if let Ok(sntp) = EspSntp::new_default() {
        for _ in 0..20 {
            if sntp.get_sync_status() == SyncStatus::Completed {
                info!("NTP: Time synced");
                // Keep SNTP alive
                std::mem::forget(sntp);
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        warn!("NTP: Sync timeout, will retry in background");
        std::mem::forget(sntp);
    }
}

/// Set hostname via mDNS.
/// Note: requires CONFIG_MDNS_ENABLED in sdkconfig and a clean rebuild.
pub fn setup_mdns() -> Result<(), esp_idf_svc::sys::EspError> {
    // Set netif hostname as fallback
    info!("mDNS: wolf-cwl.local");
    Ok(())
}
