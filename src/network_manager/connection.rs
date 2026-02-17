use zbus::proxy::Proxy;
use zbus::zvariant::{Value, OwnedValue};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SavedConnection {
    pub object_path: String,
    pub id: String,
    pub uuid: String,
    pub ssid: String
}

impl SavedConnection {
    pub async fn from_proxy(proxy: &Proxy<'_>) -> Result<Self, zbus::Error> {
        let settings: HashMap<String, HashMap<String, OwnedValue>> = proxy.call("GetSettings", &()).await?;

        let id = match &settings.get("connection") {
            Some(connection_settings) => match connection_settings.get("id") {
                Some(id) => match id.downcast_ref::<String>() {
                    Ok(s) => s.clone(),
                    Err(_) => String::from("")
                },
                None => String::from("")
            },
            None => String::from("")
        };

        let uuid = match &settings.get("connection") {
            Some(connection_settings) => match connection_settings.get("uuid") {
                Some(uuid) => match uuid.downcast_ref::<String>() {
                    Ok(s) => s.clone(),
                    Err(_) => String::from("")
                },
                None => String::from("")
            },
            None => String::from("")
        };

        let ssid = match &settings.get("802-11-wireless") {
            Some(wireless_settings) => match wireless_settings.get("ssid") {
                Some(ssid) => match ssid.downcast_ref() {
                    Ok(Value::Array(s)) => {
                        let bytes: Vec<u8> = s.iter().filter_map(|v| {
                            if let Value::U8(b) = v {
                                Some(*b)
                            } else {
                                None
                            }
                        }).collect();
                        match String::from_utf8(bytes) {
                            Ok(s) => s,
                            Err(_) => String::from("")
                        }
                    }
                    Ok(_) => String::from(""),
                    Err(_) => String::from("")
                },
                None => String::from("")
            },
            None => String::from("")
        };

        Ok(Self {
            object_path: proxy.path().to_string(),
            id,
            uuid,
            ssid
        })
    }
}
