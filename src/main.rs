use std::sync::Arc;

use chrono::{Utc, Duration};
use loki_worker::LokiWorker;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::config::SharedConfig;

mod session_manager;
mod ssh_utils;
mod ptaf_node;
mod k8s_manager;
mod loki_worker;
mod config;
mod constants;


fn colllect_with_pods(services: Option<Vec<String>>, lw: Arc<LokiWorker>, label_name: &str, n_tries: u8) -> Vec<JoinHandle<()>> {
    let mut threads = vec![];
    if let Some(units) = services {
        for unit in units {
            let l = lw.clone();
            let label = label_name.to_string();
            let t = thread::spawn(move || {
                let mut tries_left = n_tries;
                loop {
                    if tries_left == 0 {
                        println!("failed unit: {} label: {}", &unit, &label);
                        panic!("asd");
                    }            
                    if let Err(err) = l.collect_with_pods(&unit, &label, "/home") {
                        println!(">>>>>>>>>> {:?} unit: {} label: {} <<<<<<<<<<<<<<", err, &unit, &label);
                        tries_left -= 1;
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue; // Повторяем попытку
                    } else {
                        break; // Успешно выполнено
                    }
                }
            });
            threads.push(t);
        }
    }
    threads
}

fn colllect_without_pods(services: Option<Vec<String>>, lw: Arc<LokiWorker>, label_name: &str, n_tries: u8) -> Vec<JoinHandle<()>> {
    let mut threads = vec![];
    if let Some(units) = services {
        for unit in units {
            let l = lw.clone();
            let label = label_name.to_string();
            let t = thread::spawn(move || {
                let mut tries_left = n_tries;
                loop {
                    if tries_left == 0 {
                        println!("failed unit: {} label: {}", &unit, &label);
                        panic!("asd");
                    }            
                    if let Err(err) = l.collect_without_pods(&unit, &label, "/home") {
                        println!(">>>>>>>>>> {:?} unit: {} label: {} <<<<<<<<<<<<<<", err, &unit, &label);
                        tries_left -= 1;
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue; // Повторяем попытку
                    } else {
                        break; // Успешно выполнено
                    }
                }
            });
            threads.push(t);
        }
    }
    threads
}


fn main() {
    println!("start");
    println!("{:?}", std::env::current_dir().unwrap());
    let mut config = config::Config::from_string(constants::DEFAULT_CONFIG).unwrap();
    config.param.loki.log_from = Some(Utc::now() - Duration::hours(4));
    config.param.loki.log_to = Some(Utc::now());
    config.param.loki.password = "admin".to_string();
    config.param.loki.login = "admin".to_string();

    config.param.ssh.login = "ptdeploy".to_string();

    let shared_config = SharedConfig::new(config);
    // TODO добавить получение имени хоста
    let host_name = "m0-98.af.rd.ptsecurity.ru".to_string();
    println!("hostname: {}", host_name);

    let k8s_manager = Arc::new(k8s_manager::K8SManager::new("/home/pt/.kube/config").unwrap());
    let node = Arc::new(ptaf_node::PTAFNode::new(host_name, "22013".to_string(), shared_config.clone()));

    let lw = Arc::new(loki_worker::LokiWorker{ node, k8s_manager, config: shared_config.clone() });
    let now = Instant::now();
    let mut threads = vec![];
    let n_tries = 3;
    for label in shared_config.artifacts.get_labels() {
        match label {
            config::LabelType::CoreLabel(l) |
            config::LabelType::BackendLabel(l) => {
                let app_threads = colllect_with_pods(
                    Some(l.app), lw.clone(), "app", n_tries
                );
                threads.extend(app_threads);

                let unit_threads = colllect_with_pods(
                    l.unit, lw.clone(), "unit", n_tries
                );
                threads.extend(unit_threads);
            }

            config::LabelType::InfraLabel(l) => {

                let app_threads = colllect_without_pods(
                    Some(l.app), lw.clone(), "app", n_tries
                );
                threads.extend(app_threads);

                let unit_threads = colllect_without_pods(
                    l.unit, lw.clone(), "unit", n_tries
                );
                threads.extend(unit_threads);

            }

        }
    }

    for t in threads {
        t.join().unwrap();
    }

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
