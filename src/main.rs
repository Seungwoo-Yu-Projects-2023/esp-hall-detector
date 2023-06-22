use std::io::Write;
use std::net::TcpStream;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{PinDriver};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_sys as _;
use esp_idf_sys::EspError;
use log::info;

const WIFI_SSID: &'static str = env!("WIFI_SSID");
const WIFI_PW: &'static str = env!("WIFI_PW");
const SERVER_HOST: &'static str = env!("SERVER_HOST");

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().expect("Cannot get peripherals");
    let sys_loop = EspSystemEventLoop::take().expect("Cannot get system loop");
    let nvs = EspDefaultNvsPartition::take().expect("Cannot get nvs");

    let _wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))
        .expect("Cannot instantiate wifi");
    let mut wifi = BlockingWifi::wrap(
        _wifi,
        sys_loop,
    ).expect("Cannot instantiate blocking wifi");
    let wifi_config = Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.into(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: WIFI_PW.into(),
        channel: None,
    });

    connect_wifi(&mut wifi, &wifi_config).expect("Cannot connect to wifi");

    let stream = TcpStream::connect(SERVER_HOST).expect("Cannot connect to server");
    
    let hall = peripherals.pins.gpio6;
    let hall_driver = PinDriver::output(hall).expect("Cannot get pin driver");
    let mut notified = false;

    loop {
        if hall_driver.is_set_high() {
            if !notified {
                notified = true;
                match notify(&stream, "door_opened") {
                    Ok(_) => {}
                    Err(_) => {
                        return;
                    }
                };
            }
        } else {
            if notified {
                notified = false;
                match notify(&stream, "door_closed") {
                    Ok(_) => {}
                    Err(_) => {
                        return;
                    }
                };
            }
        }

        Ets::delay_ms(50)
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>, config: &Configuration) -> Result<(), EspError> {
    wifi.set_configuration(config)?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(())
}

fn notify(stream: &TcpStream, context: &str) -> std::io::Result<()> {
    let mut writer = stream;
    writer.write_all(context.as_bytes())
}
