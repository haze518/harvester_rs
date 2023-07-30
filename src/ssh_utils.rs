use crate::session_manager::SessionManager;
use r2d2::{Pool, PooledConnection};
use std::collections::HashMap;
use std::io::{Read, ErrorKind, Write};
use std::path::Path;
use std::fs;
use std::sync::Mutex;
use anyhow::Result;

pub struct SSHConnection {
    connection: PooledConnection<SessionManager>
}

impl SSHConnection {

    pub fn execute(
        &self,
        command: &str,
        envs: String,
        working_directory: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut channel = self.connection.channel_session()?;
        let mut command = match working_directory {
            Some(dir) => format!("cd {}; {}", dir, command),
            None => command.to_string(),
        };
        if envs.len() > 0 {
            command = format!("{}; {}", envs, command);
        }

        channel.request_pty_size(1024, 24, Some(0), Some(0))?;
        println!("command execute: {}", command);
        channel.exec(&command)?;

        let mut buf = vec![0; 1024];
        let mut chunks = Vec::new();
        
        while let Ok(bytes_read) = channel.read(&mut buf) {
            println!("bytes: {}", bytes_read);
            if bytes_read == 0 {
                break;
            }
            let chunk = &buf[..bytes_read];
            chunks.extend(chunk.to_owned());
        }

        let res = String::from_utf8_lossy(&chunks);
        let splited: Vec<String> = res
            .split("\n")
            .filter(|x| !x.is_empty())
            .map(|x| x.to_string())
            .collect();
        channel.close()?;
        Ok(splited)
    }

    pub fn copy_to_local(
        &self,
        source: &str,
        destination_file: &str,
    ) -> Result<()> {
        println!("dest file: {}", destination_file);
        let dirname = Path::new(destination_file)
            .parent()
            .ok_or(std::io::Error::from(ErrorKind::InvalidInput))?
            .to_str()
            .ok_or(std::io::Error::from(ErrorKind::InvalidData))?;
        ensure_dir_exists(dirname)?;
        println!("dirname: {}", dirname);

        let sftp = self.connection.sftp()?;

        let mut file = sftp.open(Path::new(source))?;

        let mut buf = vec![0; 20 * 1024 * 1024];
        let mut destination_file = fs::File::create(destination_file)?;
        println!("start loop: {}", dirname);
        loop {
            let bytes_read = file.read(&mut buf)?;
            if bytes_read == 0 {
                break;
            }
            destination_file.write_all(&buf[..bytes_read])?;
        }
        println!("end loop: {}", dirname);
        Ok(())
    }
}

pub struct SSHManager{
    pool: Mutex<Pool<SessionManager>>,
}

impl SSHManager {
    pub fn new(pool: Pool<SessionManager>) -> SSHManager {
        SSHManager { pool: Mutex::new(pool) }
    }

    pub fn get_connection(&self) -> Result<SSHConnection> {
        let pool = self.pool.lock().unwrap();
        let conn = pool.get()?;
        Ok(SSHConnection{connection: conn})
    }
}

fn ensure_dir_exists(dir_path: &str) -> Result<()> {
    let path = Path::new(dir_path);
    if !path.exists() {
        fs::create_dir(path)?;
    }
    Ok(())
}


mod tests {
    use super::*;

    #[test]
    fn test_execute() {
        let login = "admin".to_string();
        let session_manager = SessionManager {
            host: "localhost".to_string(),
            port: "2222".to_string(),
            login: login.clone(),
            password: Some("admin".to_string()),
            key_file: None,
        };

        let pool = Pool::builder()
            .max_size(1)
            .build(session_manager)
            .unwrap();

        let ssh = SSHManager::new(pool);
        let conn = ssh.get_connection().unwrap();

        let msg = "Hello World";
        let envs = "TEST=1";
        let result = conn
            .execute(
                format!("echo {}; echo {}", msg, "$TEST").as_str(),
                envs.to_string(),
                None,
            )
            .unwrap();
        assert_eq!(result, vec![msg, "1"]);
    }
}
