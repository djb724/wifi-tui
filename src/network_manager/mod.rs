use tokio::time::timeout;
use std::fmt;
use zbus::{
    connection::Connection,
    zvariant::{ObjectPath, OwnedObjectPath, Value},
    proxy::{Proxy, SignalStream}
};
use futures_util::stream::TryStreamExt;
pub mod access_point;
pub mod connection;
use access_point::AccessPoint;
use connection::SavedConnection;
use std::{collections::HashMap, time::Duration};

const DEST: &str = "org.freedesktop.NetworkManager";

#[derive(Debug, Clone)]
pub enum NmError {
    NoWirelessDevice,
    NoSuchConnection,
    ConnectionFailed,
    ConnectionTimeout,
    UnknownError,
    InvalidCredentials,
    Dbus(zbus::Error)
}

impl fmt::Display for NmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NmError::NoWirelessDevice => write!(f, "NoWirelessDevice"),
            NmError::NoSuchConnection => write!(f, "NoSuchConnection"),
            NmError::ConnectionFailed => write!(f, "ConnectionFailed"),
            NmError::ConnectionTimeout => write!(f, "ConnectionTimeout"),
            NmError::UnknownError => write!(f, "UnknownError"),
            NmError::InvalidCredentials => write!(f, "InvalidCredentials"),
            NmError::Dbus(ze) => write!(f, "{}", ze)
        }
    }
}

impl std::error::Error for NmError {}

impl From<zbus::Error> for NmError {
    fn from(e: zbus::Error) -> Self {
        NmError::Dbus(e)
    }
}

enum NmActiveConnectionState {
    Unknown = 0,
    Activating = 1,
    Activated = 2,
    Deactivating = 3,
    Deactivated = 4
}

const UNKNOWN: u32 = 0;
const NONE: u32 = 1;
const USER_DISCONNECTED: u32 = 2;
const DEVICE_DISCONNECTED: u32 = 3;
const SERVICE_STOPPED: u32 = 4;
const IP_CONFIG_INVALID: u32 = 5;
const CONNECT_TIMEOUT: u32 = 6;
const SERVICE_START_TIMEOUT: u32 = 7;
const SERVICE_START_FAILED: u32 = 8;
const NO_SECRETS: u32 = 9;
const LOGIN_FAILED: u32 = 10;
const CONNECTION_REMOVED: u32 = 11;
const DEPENDENCY_FAILED: u32 = 12;
const DEVICE_REALIZE_FAILED: u32 = 13;
const DEVICE_REMOVED: u32 = 14;

fn rescan_match_rule<'m>() -> Result<zbus::MatchRule<'m>, zbus::Error> {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.freedesktop.NetworkManager")?
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .build();
    Ok(rule)
}

pub struct NetworkManager {
    connection: Connection,
    pub wireless_interface: String,
    wireless_object_path: zbus::zvariant::OwnedObjectPath
}

impl NetworkManager {
    pub async fn new() -> Result<Self, NmError> {
        let connection = match Connection::system().await {
            Ok(c) => c,
            Err(e) => return Err(e.into())
        };

        let network_manager_proxy = match Proxy::new(
            &connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        ).await {
            Ok(proxy) => proxy,
            Err(e) => return Err(e.into())
        };

        let device_object_paths: Vec<OwnedObjectPath> = match network_manager_proxy.call("GetAllDevices", &()).await {
            Ok(device_object_paths) => device_object_paths,
            Err(e) => return Err(e.into())
        };
        // println!("Device object paths: {:?}", device_object_paths);

        let mut wireless_interface = String::from("");
        let mut wireless_device_path: Option<OwnedObjectPath> = None;

        for dop in device_object_paths {
            let dop_str = dop.to_string();
            let device_proxy = match Proxy::new(
                &connection, 
                DEST, 
                dop_str,
                "org.freedesktop.NetworkManager.Device"
            ).await {
                Ok(p) => p,
                Err(e) => return Err(e.into())
            };

            let device_type: u32 = match device_proxy.get_property("DeviceType").await {
                Ok(dt) => dt,
                Err(e) => return Err(e.into())
            };

            if device_type == 2 {
                wireless_device_path = Some(dop);
                wireless_interface = match device_proxy.get_property("Interface").await {
                    Ok(iname) => iname,
                    Err(e) => return Err(e.into())
                };
                break;
            }
        };

        let wireless_device_path = if let Some(path) = wireless_device_path {
            path
        } else {
            panic!("No wireless device found");
        };

        Ok(Self{
            connection: connection,
            wireless_interface: wireless_interface,
            wireless_object_path: wireless_device_path
        })
    }

    pub async fn rescan(&self) -> Result<(), NmError> {
        let mut options: HashMap<&str, Value> = HashMap::new();
        options.insert("ssids", Value::from(Vec::<Vec<u8>>::new()));

        let proxy = Proxy::new(
            &self.connection, 
            DEST, 
            &self.wireless_object_path, 
            "org.freedesktop.NetworkManager.Device.Wireless"
        ).await?;

        match proxy.call("RequestScan", &options).await {
            Err(e) => Err(e.into()),
            Ok(()) => {
                let rule = rescan_match_rule()?;
                let mut stream = zbus::MessageStream::for_match_rule(rule, &self.connection, Some(1)).await?;
                stream.try_next().await?;
                Ok(())
            }
        }
    }

    pub async fn get_access_points(&self) -> Result<HashMap<String, AccessPoint>, NmError> {
        let connection = &self.connection;

        self.rescan().await?;

        let wireless_device_proxy = match Proxy::new(
            &connection, 
            DEST, 
            &self.wireless_object_path, 
            "org.freedesktop.NetworkManager.Device.Wireless"
        ).await {
            Ok(p) => p,
            Err(e) => return Err(e.into())
        };

        let access_point_paths: Vec<OwnedObjectPath> = match wireless_device_proxy.call("GetAllAccessPoints", &()).await {
            Ok(aps) => aps,
            Err(e) => return Err(e.into())
        };
        // println!("Access point object paths: {:?}", access_point_paths);

        let mut access_point_map = HashMap::<String, AccessPoint>::new();
        for ap_path in &access_point_paths {
            let access_point_proxy = Proxy::new(
                &connection, 
                DEST, 
                ap_path, 
                "org.freedesktop.NetworkManager.AccessPoint"
            ).await?;

            let access_point = AccessPoint::from_proxy(&access_point_proxy).await?;

            if access_point.ssid.is_empty() {
                continue;
            }

            if access_point_map.contains_key(&access_point.ssid) {
                let existing_ap = access_point_map.get(&access_point.ssid).unwrap();
                if access_point.strength > existing_ap.strength {
                    access_point_map.insert(access_point.ssid.clone(), access_point);
                }
            } else {
                access_point_map.insert(access_point.ssid.clone(), access_point);
            }
        };

        let settings_proxy = Proxy::new(
            &connection,
            DEST,
            "/org/freedesktop/NetworkManager/Settings",
            "org.freedesktop.NetworkManager.Settings"
        ).await?;

        let connection_paths: Vec<OwnedObjectPath> = settings_proxy.call("ListConnections", &()).await?;
        for conn_path in &connection_paths {
            let connection_proxy = Proxy::new(
                &connection,
                DEST,
                conn_path,
                "org.freedesktop.NetworkManager.Settings.Connection"
            ).await?;

            let connection = SavedConnection::from_proxy(&connection_proxy).await?;

            if let Some(ap) = access_point_map.get_mut(&connection.ssid) {
                ap.connection = Some(connection);
            }
        }

        Ok(access_point_map)
    }

    async fn wait_for_connection_state(&self, path: &OwnedObjectPath) -> Result<(), NmError> {
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender("org.freedesktop.NetworkManager").unwrap()
            .path(path).unwrap()
            .interface("org.freedesktop.NetworkManager.Connection.Active").unwrap()
            .member("StateChanged").unwrap()
            .build();
        let mut stream = zbus::MessageStream::for_match_rule(rule, &self.connection, None).await.map_err(NmError::Dbus)?;

        let result = timeout(Duration::from_secs(10), async {
            loop {
                if let Ok(Some(msg)) = stream.try_next().await {
                    let (state, reason) = match msg.body().deserialize::<(u32, u32)>() {
                        Ok(t) => t,
                        Err(_) => return Err(NmError::UnknownError)
                    };

                    if state == NmActiveConnectionState::Activated as u32 {
                        break Ok(());
                    } else if state == NmActiveConnectionState::Deactivated as u32 {
                        match reason {
                            LOGIN_FAILED => break Err(NmError::InvalidCredentials),
                            CONNECT_TIMEOUT => break Err(NmError::ConnectionTimeout),
                            IP_CONFIG_INVALID => break Err(NmError::ConnectionFailed),
                            _ => break Err(NmError::ConnectionFailed)
                        }
                    }
                } else {
                    break Err(NmError::UnknownError);
                }
            }
        }).await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(NmError::ConnectionTimeout)
        }
    }

    async fn clean_connection(&self, connection_path: &OwnedObjectPath) -> Result<(), NmError> {
        let proxy = Proxy::new(
            &self.connection, 
            DEST, 
            connection_path, 
            "org.freedesktop.NetworkManager.Settings.Connection"
        ).await?;

        let _: () = proxy.call("Delete", &()).await?;

        Ok(())
    }

    pub async fn restore_connection(&self, connection: &SavedConnection) -> Result<SavedConnection, NmError> {
        let proxy = Proxy::new(
            &self.connection,
            DEST,
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        ).await?;

        let active_connection_path: OwnedObjectPath = proxy.call("ActivateConnection", &(
                ObjectPath::try_from(connection.object_path.clone()).unwrap(),
                self.wireless_object_path.clone(),
                ObjectPath::from_static_str("/").unwrap()
        )).await?;

        self.wait_for_connection_state(&active_connection_path).await?;

        Ok(connection.clone())
    }

    pub async fn connect_secure(&self, access_point: &AccessPoint, password: &str) -> Result<SavedConnection, NmError> {
        
        let mut options = HashMap::<String, HashMap::<String, Value>>::new();
        let connection_settings = HashMap::<String, Value>::from([
            ("type".to_string(), Value::from("802-11-wireless")),
            ("id".to_string(), Value::from(format!("{} connection", access_point.ssid)))
        ]);
        let wifi_settings = HashMap::<String, Value>::from([
            ("ssid".to_string(), Value::from(access_point.ssid.clone())),
            ("mode".to_string(), Value::from("infrastructure"))
        ]);
        let wifi_security_settings = HashMap::<String, Value>::from([
            ("key-mgmt".to_string(), Value::from("wpa-psk")),
            ("psk".to_string(), Value::from(password.to_string()))
        ]);
        options.insert("connection".to_string(), connection_settings);
        options.insert("802-11-wireless".to_string(), wifi_settings);
        options.insert("802-11-wireless-security".to_string(), wifi_security_settings);

        let network_manager_proxy = Proxy::new(
            &self.connection,
            DEST,
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        ).await?;

        let (connection_path, active_connection_path): (OwnedObjectPath, OwnedObjectPath)
            = network_manager_proxy.call("AddAndActivateConnection", &(
                &options,
                &self.wireless_object_path,
                &access_point.object_path
            )).await?;

        match self.wait_for_connection_state(&active_connection_path).await {
            Ok(_) => {
                let proxy = Proxy::new(&self.connection, DEST, &connection_path, "org.freedesktop.NetworkManager.Settings.Connection").await?;
                let sc = SavedConnection::from_proxy(&proxy).await?;
                Ok(sc)
            },
            Err(NmError::ConnectionTimeout) => Err(NmError::ConnectionTimeout),
            Err(e) => {
                self.clean_connection(&connection_path).await?;
                Err(e)
            }
        }
    }

    pub async fn connect_open(&self, access_point: &AccessPoint) -> Result<SavedConnection, NmError> {
        let connection = &self.connection;

        let mut options = HashMap::<String, HashMap::<String, Value>>::new();
        let connection_settings = HashMap::<String, Value>::from([
            ("type".to_string(), Value::from("802-11-wireless")),
            ("id".to_string(), Value::from(format!("{} connection", access_point.ssid)))
        ]);
        let wifi_settings = HashMap::<String, Value>::from([
            ("ssid".to_string(), Value::from(access_point.ssid.clone())),
            ("mode".to_string(), Value::from("infrastructure"))
        ]);
        options.insert("connection".to_string(), connection_settings);
        options.insert("802-11-wireless".to_string(), wifi_settings);

        let network_manager_proxy = Proxy::new(
            &connection,
            DEST,
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager"
        ).await?;

        let (connection_path, active_connection_path): (OwnedObjectPath, OwnedObjectPath)
            = network_manager_proxy.call("AddAndActivateConnection", &(
                &options,
                &self.wireless_object_path,
                &access_point.object_path
            )).await?;

        match self.wait_for_connection_state(&active_connection_path).await {
            Ok(_) => {
                let proxy = Proxy::new(&self.connection, DEST, &connection_path, "org.freedesktop.NetworkManager.Settings.Connection").await?;
                let sc = SavedConnection::from_proxy(&proxy).await?;
                Ok(sc)
            },
            Err(NmError::ConnectionTimeout) => Err(NmError::ConnectionTimeout),
            Err(e) => {
                self.clean_connection(&connection_path).await?;
                Err(e)
            }
        }
    }
}
