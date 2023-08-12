use std::sync::Arc;

use chrono::{Utc, Duration};
use std::thread;
use std::time::Instant;

mod session_manager;
mod ssh_utils;
mod ptaf_node;
mod k8s_manager;
mod loki_worker;
mod config;
mod constants;


fn main() {
    println!("start");
    println!("{:?}", std::env::current_dir().unwrap());
    let mut config = config::Config::from_string(constants::DEFAULT_CONFIG).unwrap();
    config.param.loki.log_from = Some(Utc::now() - Duration::hours(1));
    config.param.loki.log_to = Some(Utc::now());
    config.param.loki.password = "admin".to_string();
    config.param.loki.login = "admin".to_string();

    config.param.ssh.login = "ptdeploy".to_string();

    // let mut loki = config::Loki::default();
    // loki.login = "admin".to_string();
    // loki.password = "admin".to_string();
    // loki.log_from = Some(Utc::now() - Duration::hours(1));
    // loki.log_to = Some(Utc::now());

    let config = Arc::new(config
        // config::Config {
        //     ssh_creds: config::SSHCreds {
        //         login: "ptdeploy".to_string(),
        //         password: None,
        //         key_path: Some("/home/pt/.ssh/id_rsa.ptaf".to_string()),
        //     },
        //     loki,
        // }
    );
    let host_name = "m0-165.af.rd.ptsecurity.ru".to_string();
    // let host_name = hostname::get()
    //     .unwrap()
    //     .into_string()
    //     .unwrap();
    println!("hostname: {}", host_name);

    // let k8s_manager = k8s_manager::K8SManager::new("/home/pt/.kube/config").unwrap();
    // k8s_manager.get_pods().unwrap();

    let node = Arc::new(ptaf_node::PTAFNode::new(host_name, "22013".to_string(), config.clone()));

    let lw = Arc::new(loki_worker::LokiWorker{
        node,
        config: config.clone(),
    });
    let now = Instant::now();
    let mut threads = vec![];
    for svc in ["ptaf-conf-mgr-rest", "ptaf-border", "ptaf-resource-mgr"] {
        // lw.collect_without_pods(svc, "app", "/home").unwrap();
        let l = lw.clone();
        let t = thread::spawn(move || {
            l.collect_without_pods(svc, "app", "/home").unwrap();
        });
        threads.push(t);
    }
    for t in threads {
        t.join().unwrap();
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
    // lw.collect_without_pods("ptaf-conf-mgr-rest", "app", "/home").unwrap();
    // let result = lw.collect_labels("instance").unwrap();
    // println!("result: {:?}", result)
}
