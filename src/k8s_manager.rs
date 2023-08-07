use anyhow::Result;
use serde::Deserialize;
use serde_yaml::Deserializer;
use std::fs;
use base64::{Engine as _, alphabet, engine::{self, general_purpose}};
use curl::easy::Easy;


#[derive(Debug, Deserialize)]
struct ClusterData {
    cluster: Cluster,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Cluster {
    #[serde(rename = "certificate-authority-data")]
    ca_cert: String,
    server: String,
}

#[derive(Debug, Deserialize)]
struct ContextData {
    context: Context,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Context {
    cluster: String,
    user: String,
}

#[derive(Debug, Deserialize)]
struct UserData {
    name: String,
    user: User,
}

#[derive(Debug, Deserialize)]
struct User {
    #[serde(rename = "client-certificate-data")]
    certificate: String,
    #[serde(rename = "client-key-data")]
    key: String,
}

#[derive(Debug, Deserialize)]
struct KubeConfig {
    clusters: Vec<ClusterData>,
    contexts: Vec<ContextData>,
    users: Vec<UserData>,
}

impl KubeConfig {
    fn new(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let result: KubeConfig = serde_yaml::from_str(content.as_str())?;
        Ok(result)
    }
}

pub struct K8SManager {
    kube_config: KubeConfig,
}

impl K8SManager {
    pub fn new(kubeconfig_path: &str) -> Result<Self> {
        let cfg = KubeConfig::new(kubeconfig_path)?;
        Ok(K8SManager{ kube_config: cfg })
    }

    // корректный запрос: root@m0-34:~# curl https://127.0.0.1:6443/apis/apps/v1/namespaces/default/deployments --cacert ca.cert --cert cert --key key
    // где certs - base64 decode
    // завернуть ответ в serde структуру
    pub fn get_pods(&self) -> Result<()> {
        let ca_cert = general_purpose::STANDARD.decode(&self.kube_config.clusters[0].cluster.ca_cert)?;
        let cert = general_purpose::STANDARD.decode(&self.kube_config.users[0].user.certificate)?;
        let key = general_purpose::STANDARD.decode(&self.kube_config.users[0].user.key)?;

        let kube_api = &self.kube_config.clusters[0].cluster.server;
        let url = format!("{}/api/v1/pods", kube_api);
        
        let mut handle = Easy::new();
        handle.url(&url)?;
        handle.ssl_cainfo_blob(&ca_cert)?;
        handle.ssl_cert_blob(&cert)?;
        handle.ssl_key_blob(&key)?;

        let mut buf = Vec::new();
        let mut transfer = handle.transfer();
        transfer.write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
        }).unwrap();
        transfer.perform()?;
        drop(transfer);

        let result = std::str::from_utf8(&buf)?;

        println!("Response: {:?}", result);
        Ok(())
    }
}


mod tests {
    use super::*;

    #[test]
    fn test_kubeconfig() {
        let data = r#"
        apiVersion: v1
        clusters:
        - cluster:
            certificate-authority-data: hidden
            server: https://127.0.0.1:6443
          name: cluster.local
        contexts:
        - context:
            cluster: cluster.local
            user: kubernetes-admin
          name: kubernetes-admin@cluster.local
        current-context: kubernetes-admin@cluster.local
        kind: Config
        preferences: {}
        users:
        - name: kubernetes-admin
          user:
            client-certificate-data: hidden
            client-key-data: hidden
        "#;
        let result: Result<KubeConfig, serde_yaml::Error> = serde_yaml::from_str(data);
        assert!(result.is_ok());
    }
}