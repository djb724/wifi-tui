use zbus::proxy::Proxy;
use crate::network_manager::SavedConnection;


#[derive(Clone, Debug)]
pub struct AccessPoint {
    pub object_path: zbus::zvariant::OwnedObjectPath,
    pub ssid: String,
    pub hw_address: String,
    pub strength: u8,
    pub mode: u32,
    pub flags: u32,
    pub connection: Option<SavedConnection>
}

impl AccessPoint {
    pub async fn from_proxy(proxy: &Proxy<'_>) -> Result<Self, zbus::Error> {
        let ssid = match proxy.get_property("Ssid").await {
            Ok(ssid) => match String::from_utf8(ssid) {
                Ok(s) => s,
                Err(_) => String::from("")
            },
            Err(e) => return Err(e)
        };

        let hw_address = match proxy.get_property("HwAddress").await {
            Ok(hw_address) => hw_address,
            Err(e) => return Err(e)
        };

        let strength: u8 = match proxy.get_property("Strength").await {
            Ok(strength) => strength,
            Err(e) => return Err(e)
        };

        // let bandwidth: u32 = match proxy.get_property("Bandwidth").await {
        //     Ok(bandwidth) => bandwidth,
        //     Err(e) => return Err(e)
        // };

        let mode: u32 = match proxy.get_property("Mode").await {
            Ok(mode) => mode,
            Err(e) => return Err(e)
        };

        let flags: u32 = match proxy.get_property("Flags").await {
            Ok(flags) => flags,
            Err(e) => return Err(e)
        };

        // let wpa_flags: u32 = match proxy.get_property("WpaFlags").await {
        //     Ok(wpa_flags) => wpa_flags,
        //     Err(e) => return Err(e)
        // };
        //
        // let rsn_flags: u32 = match proxy.get_property("RsnFlags").await {
        //     Ok(rsa_flags) => rsa_flags,
        //     Err(e) => return Err(e)
        // };

        Ok(AccessPoint{
            object_path: proxy.path().clone().into(),
            ssid: ssid,
            hw_address: hw_address,
            strength: strength,
            mode: mode,
            flags: flags,
            connection: None
        })
    }
}
