use chrono::{DateTime, Utc};
use std::collections::HashMap;

type UTCDatetTime = DateTime<Utc>;

#[derive(Clone, Debug)]
pub struct Loki {
    pub addr: String,
    pub https: bool,
    pub login: String,
    pub password: String,
    pub org_id: String,
    pub log_from: Option<UTCDatetTime>,
    pub log_to: Option<UTCDatetTime>,
}

impl Default for Loki {
    fn default() -> Self {
        Loki {
            addr: "loki.ptaf-infra.svc.cluster.local:3100".to_string(),
            https: false,
            login: "".to_string(),
            password: "".to_string(),
            org_id: "3jqM2DLOMbbQzdodO3cO".to_string(),
            log_from: None,
            log_to: None,
        }
    }
}

impl Loki {
    pub fn full_address(&self) -> String {
        let protocol = if self.https { "https" } else { "http" }; 
        format!("{}://{}", protocol, self.addr)
    }
}

pub struct SSHCreds {
    pub login: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
}


pub struct Config {
    pub ssh_creds: SSHCreds,
    pub loki: Loki,
}


impl Config {
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