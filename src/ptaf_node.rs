use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use anyhow::Result;
use r2d2::Pool;
use std::path::PathBuf;

use crate::session_manager;
use crate::ssh_utils;
use crate::config;

pub struct PTAFNode {
    host: String,
    port: String,
    config: Arc<config::Config>,
    ssh_manager: ssh_utils::SSHManager,
}

impl PTAFNode {
    
    pub fn new(host: String, port: String, config: Arc<config::Config>) -> Self {
        let c = config.clone();
        let ssh_manager = Self::init_ssh_manager(host.clone(), port.clone(), c).unwrap();
        PTAFNode { host, port, config, ssh_manager }
    }

    pub fn get_ssh_conn(&self) -> Result<ssh_utils::SSHConnection> {
        Ok(self.ssh_manager.get_connection()?)
    }

    fn init_ssh_manager(host: String, port: String, config: Arc<config::Config>) -> Result<ssh_utils::SSHManager> {
        let cfg = config.clone();
        let manager = session_manager::SessionManager {
            host: host,
            port: port,
            login: cfg.ssh_creds.login.clone(),
            password: cfg.ssh_creds.password.clone(),
            key_file: cfg.ssh_creds.key_path.clone(),
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
        let config = Arc::new(
            config::Config {
                ssh_creds: config::SSHCreds {
                    login: "admin".to_string(),
                    password: Some("admin".to_string()),
                    key_path: Some("".to_string()),
                },
                loki: config::Loki::default(),
            }
        );

        let node = Arc::new(
            PTAFNode::new("localhost".to_string(), "2222".to_string(), config)
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
