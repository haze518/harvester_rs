use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use anyhow::Result;
use r2d2::Pool;
use std::path::PathBuf;

use crate::session_manager;
use crate::ssh_utils;
use crate::config;
use crate::constants;

pub struct PTAFNode {
    host: String,
    port: String,
    config: config::SharedConfig,
    ssh_manager: ssh_utils::SSHManager,
}

impl PTAFNode {
    
    pub fn new(host: String, port: String, config: config::SharedConfig) -> Self {
        let ssh_manager: ssh_utils::SSHManager = Self::init_ssh_manager(host.clone(), port.clone(), config.clone()).unwrap();
        PTAFNode { host, port, config, ssh_manager }
    }

    pub fn get_ssh_conn(&self) -> Result<ssh_utils::SSHConnection> {
        Ok(self.ssh_manager.get_connection()?)
    }

    fn init_ssh_manager(host: String, port: String, config: config::SharedConfig) -> Result<ssh_utils::SSHManager> {
        let manager = session_manager::SessionManager {
            host: host,
            port: port,
            login: config.param.ssh.login.clone(),
            password: config.param.ssh.password.clone(),
            key_file: Some(config.param.ssh.key_path()),
        };
        println!("init ssh manager");
        let pool = Pool::builder().build(manager)?;
        let ssh_manager = ssh_utils::SSHManager::new(pool);
        Ok(ssh_manager)
    }
    
}


mod tests {

    use super::*;

    #[test]
    fn test_get_ssh_conn() {
        let mut cfg = config::Config::from_string(constants::DEFAULT_CONFIG).unwrap();
        cfg.param.ssh.login = "admin".to_string();
        cfg.param.ssh.password = Some("admin".to_string());

        let shared_config = config::SharedConfig::new(cfg);

        let node = Arc::new(
            PTAFNode::new("localhost".to_string(), "2222".to_string(), shared_config)
        );

        let threads = vec!["echo 1337", "echo 777"]
            .into_iter()
            .map(|x| {
                let n = node.clone();
                thread::spawn(move || {
                    let conn = n.get_ssh_conn().unwrap();

                    let result = conn
                        .execute(
                            x,
                            "".to_string(),
                            None,
                        )
                        .unwrap();
                    result            
                })
            });
        
        let mut f = vec![];
        for handle in threads {
            let res = handle.join().unwrap();
            f.extend(res);
        }

        assert_eq!(f, vec!["1337", "777"]);        
    }

}
