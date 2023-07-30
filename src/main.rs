use std::sync::Arc;

use chrono::{Utc, Duration};

mod session_manager;
mod ssh_utils;
mod config;
mod ptaf_node;
mod loki_worker;


fn main() {
    println!("start");
    let mut loki = config::Loki::default();
    loki.login = "admin".to_string();
    loki.password = "admin".to_string();
    loki.log_from = Some(Utc::now());
    loki.log_to = Some(Utc::now() + Duration::minutes(5 as i64));

    let config = Arc::new(
        config::Config {
            ssh_creds: config::SSHCreds {
                login: "ptdeploy".to_string(),
                password: None,
                key_path: Some("/home/pt/.ssh/id_rsa.ptaf".to_string()),
            },
            loki,
        }
    );
    let host_name = "m0-98.af.rd.ptsecurity.ru".to_string();
    // let host_name = hostname::get()
    //     .unwrap()
    //     .into_string()
    //     .unwrap();
    println!("hostname: {}", host_name);

    let node = Arc::new(ptaf_node::PTAFNode::new(host_name, "22013".to_string(), config.clone()));

    let lw = loki_worker::LokiWorker{
        node,
        config: config.clone(),
    };
    lw.collect_without_pods("ptaf-conf-mgr-rest", "app", "/home/ash").unwrap();
    // let result = lw.collect_labels("instance").unwrap();
    // println!("result: {:?}", result)
}
