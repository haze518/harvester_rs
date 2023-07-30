use std::{net::TcpStream, io::Read, path::PathBuf};
use r2d2::ManageConnection;
use ssh2::Session;
use thiserror::Error;
// use std::fs;
use std::path;

#[derive(Debug, Error)]
pub enum SessionManagerError {
    #[error("TCP connection error: {0}")]
    TcpError(#[from] std::io::Error),

    #[error("SSH session error: {0}")]
    SshError(#[from] ssh2::Error),

    #[error("SSH connection is no longer valid")]
    InvalidSshConnection,
}

pub struct SessionManager {
    pub host: String,
    pub port: String,
    pub login: String,
    pub password: Option<String>,
    pub key_file: Option<String>,
}

impl ManageConnection for SessionManager {
    type Connection = Session;
    type Error = SessionManagerError;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let tcp_stream = TcpStream::connect(
            format!("{}:{}", self.host, self.port)
        )
            .map_err(|err| SessionManagerError::from(err))?;
        
        let mut session = Session::new()
            .map_err(|err| SessionManagerError::from(err))?;
        
        session.set_tcp_stream(tcp_stream);
        session.handshake()?;

        if let Some(passw) = &self.password {
            session.userauth_password(&self.login, &passw)?;
        } else if let Some(key_file) = &self.key_file {
            let path = PathBuf::from(key_file);
            println!("get userauth_pubkey_file");
            session.userauth_pubkey_file(&self.login, None, &path, None)?;
            println!("end userauth_pubkey_file");
        }

        Ok(session)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> std::result::Result<(), Self::Error> {
        let mut channel = conn.channel_session()
            .map_err(|err| SessionManagerError::from(err))?;
        channel.exec("echo TEST")?;
        
        let mut buf = String::new();
        channel.read_to_string(&mut buf)
            .map_err(|err| SessionManagerError::from(err))?;
        
        if buf.is_empty() {
            Err(SessionManagerError::InvalidSshConnection)
        } else {
            Ok(())
        }
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        self.is_valid(conn).is_err()
    }

}


mod tests {
    use super::*;

    #[test]
    fn test_connect() {
        let session_manager = SessionManager {
            host: "localhost".to_string(),
            port: "2222".to_string(),
            login: "admin".to_string(),
            password: Some("admin".to_string()),
            key_file: None,
        };
        let result = session_manager.connect();
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_valid() {
        let session_manager = SessionManager {
            host: "localhost".to_string(),
            port: "2222".to_string(),
            login: "admin".to_string(),
            password: Some("admin".to_string()),
            key_file: None,
        };
        let mut connection = session_manager.connect().unwrap();
        let result = session_manager.is_valid(&mut connection);
        assert!(result.is_ok());
    }
}