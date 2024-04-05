extern crate core;

use core::convert::TryInto;
use core::time;
use std::{mem, slice};
use std::time::Duration;
use embedded_svc::mqtt::client::QoS;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};

use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use esp_idf_svc::tls::X509;
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};


use log::info;
use rumqttc::{Transport, TlsConfiguration};
use rumqttc::v5::{Client, MqttOptions, };

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let timer_service = EspTaskTimerService::new()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = AsyncWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
        timer_service,
    )?;

    block_on(connect_wifi(&mut wifi))?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    let client_cert_bytes: Vec<u8> = include_bytes!("/home/kefran/Documents/certificats/client1-authnID.pem").to_vec();
    let private_key_bytes: Vec<u8> = include_bytes!("/home/kefran/Documents/certificats/client1-authnID.key").to_vec();
    let client_cert: X509 = convert_certificate(client_cert_bytes);
    let private_key: X509 = convert_certificate(private_key_bytes);
    let transport = Transport::Tls(TlsConfiguration::Simple {
        ca,
        alpn: None,
        client_auth: Some((client_cert, private_key)),
    });

    // Create Client Instance and Define Behaviour on Event
    let mut mqttoptions = MqttOptions::new("client1-session-1", "test-frank.canadacentral-1.ts.eventgrid.azure.net", 8883);
    mqttoptions.set_keep_alive(Duration::from_secs(120));
    mqttoptions.set_credentials("client1-authnID", "");

    let (mut client, mut connection) = Client::new(mqttoptions, 10);

    info!("mqtt client is up");

    info!("Shutting down in 60s...");

    std::thread::sleep(core::time::Duration::from_secs(60));

    Ok(())
}

fn convert_certificate(mut certificate_bytes: Vec<u8>) -> X509<'static> {
    // append NUL
    certificate_bytes.push(0);

    // convert the certificate
    let certificate_slice: &[u8] = unsafe {
        let ptr: *const u8 = certificate_bytes.as_ptr();
        let len: usize = certificate_bytes.len();
        mem::forget(certificate_bytes);

        slice::from_raw_parts(ptr, len)
    };

    // return the certificate file in the correct format
    X509::pem_until_nul(certificate_slice)
}

async fn connect_wifi(wifi: &mut AsyncWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start().await?;
    info!("Wifi started");

    wifi.connect().await?;
    info!("Wifi connected");

    wifi.wait_netif_up().await?;
    info!("Wifi netif up");

    Ok(())
}