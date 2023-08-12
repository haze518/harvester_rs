use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{de, Deserialize, Deserializer};
use std::{fs, str::FromStr, fmt::Display};
use anyhow::{Result, Context};

use crate::constants;

#[derive(Debug, Deserialize)]
pub struct Labels {
    pub app: Vec<String>,
    pub unit: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct App {
    pub items: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Unit {
    pub items: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Artifacts {
    pub cores: bool,
    pub backend: bool,
    pub core_labels: Labels,
    pub infra_labels: Labels,
    pub backend_labels: Labels,
}

#[derive(Debug, Deserialize)]
pub struct Param {
    pub ssh: SshConfig,
    pub loki: LokiConfig,
}

#[derive(Debug, Deserialize)]
pub struct SshConfig {
    pub addr: String,
    pub login: String,
    pub password: Option<String>,
}

impl SshConfig {
    pub fn key_path(&self) -> String {
        "/home/pt/.ssh/id_rsa.ptaf".to_string()
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct LokiConfig {
    pub login: String,
    pub password: String,
    #[serde(deserialize_with = "deserialize_datetime_from_str")]
    pub log_from: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "deserialize_datetime_from_str")]
    pub log_to: Option<DateTime<Utc>>,
    pub since: String,
    pub time_zone: String,
    pub tenant_id: String,
}

fn deserialize_datetime_from_str<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_str: Option<String> = Deserialize::deserialize(deserializer)?;
    match opt_str {
        Some(s) => {
            let dt = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M")
                .map_err(|_| de::Error::custom("Invalid date format"))
                .and_then(|dt| Ok(DateTime::<Utc>::from_utc(dt, Utc)))?;
            Ok(Some(dt))
        },
        None => Ok(None)
    }
}

impl LokiConfig {
    pub fn full_address(&self) -> String {
        "http://loki.ptaf-infra.svc.cluster.local:3100".to_string()
    }
    pub fn org_id(&self) -> String {
        "3jqM2DLOMbbQzdodO3cO".to_string()
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub artifacts: Artifacts,
    pub param: Param,
}

impl Config {
    pub fn from_string(config_str: &str) -> Result<Self> {
        // let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn get_envs(&self) -> String {
        "".to_string()
        // let login = &self.ssh_creds.login;
        // let path = match login.as_str() {
        //     "root" => format!("{}/", login),
        //     _ => format!("home/{}", login),
        // };
        // let envs = HashMap::from([
        //     ("USER", login.to_owned()),
        //     ("PWD", path.to_owned()),
        //     ("HOME", path.to_owned()),
        //     ("MAIL", format!("/var/mail/{}", login)),
        //     ("SHELL", "/bin/bash".to_string()),
        //     ("SHLVL", "1".to_string()),
        //     ("LOGNAME", login.to_owned()),
        //     ("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()),
        // ]);
        // let mut env =String::with_capacity(envs.len());
        // for (i, (key, val)) in envs.iter().enumerate() {
        //     env.push_str(format!("{}={}", key, &val).as_str());
        //     if i != envs.len() - 1 {
        //         env.push_str(", ");
        //     }
        // }
        // env
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_config_from_file() {
        let config = Config::from_string(constants::DEFAULT_CONFIG);
        assert!(config.is_ok());
    }
}
