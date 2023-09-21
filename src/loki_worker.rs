use std::sync::Arc;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::fs;
use std::thread;


use crate::k8s_manager;
use crate::config;
use crate::ptaf_node;

const LIMIT: u32 = 2000000000;

struct LokiQueryBuilder<'a> {
    query: Vec<String>,
    time_format: String,
    loki_creds: &'a config::LokiConfig,
}

impl<'a> LokiQueryBuilder<'a> {

    fn new(time_format: &str, loki_creds: &'a config::LokiConfig) -> Self {
        LokiQueryBuilder {
            query: vec![],
            time_format: time_format.to_string(),
            loki_creds,
        }
    }
    
    fn add_query(&mut self, label: &str, val: &str, instance: Option<&str>, instance_val: Option<&str>) -> &mut Self {
        let mut q = format!("query \'{{{}=\"{}\"}}\'", label, val);
        if instance.is_some() && instance_val.is_some() {
            q = format!(
                "query \'{{{}=\"{}\", {}=\"{}\"}}\'", label, val, instance.unwrap(), instance_val.unwrap(),
            );
        }
        self.query.push(q);
        self
    }

    fn add_instance(&mut self, pod_name: &str) -> &mut Self {
        let q = format!("instance=\"{}\"", pod_name);
        self.query.push(q);
        self
    }

    fn add_from(&mut self) -> &mut Self {
        let q = format!("--from=\'{}\'", self.loki_creds.log_from.unwrap().format(self.time_format.as_str()));
        self.query.push(q);
        self
    }

    fn add_to(&mut self) -> &mut Self {
        let q = format!("--to=\'{}\'", self.loki_creds.log_to.unwrap().format(self.time_format.as_str()));
        self.query.push(q);
        self
    }

    fn add_batch(&mut self, number: u16) -> &mut Self {
        let q = format!("--batch={}", number.to_string());
        self.query.push(q);
        self
    }

    fn add_limit(&mut self, number: u32) -> &mut Self {
        let q = format!("--limit {}", number.to_string());
        self.query.push(q);
        self
    }

    fn add_forward(&mut self) -> &mut Self {
        self.query.push(String::from("--forward"));
        self
    }

    fn add_raw(&mut self) -> &mut Self {
        self.query.push(String::from("-o raw"));
        self
    }

    fn script_head(&self) -> String {
        let cmd = "/opt/logcli";
        let user = self.loki_creds.login.clone();
        let passwd = self.loki_creds.password.clone();
        let loki_addr = self.loki_creds.full_address();
        let org_id = self.loki_creds.org_id();

        let mut loki_cmd = format!("{} --username=\"{}\" --password=\"{}\"", cmd, user, passwd);
        loki_cmd = format!("{} --addr=\"{}\" --org-id=\"{}\" -q", loki_cmd, loki_addr, org_id);
        loki_cmd
    }

    fn get_query(&mut self) -> String {
        self.query.insert(0, self.script_head());
        self.query.join(" ")
    }

}

pub struct LokiWorker {
    pub node: Arc<ptaf_node::PTAFNode>,
    pub config: config::SharedConfig,
    pub k8s_manager: Arc<k8s_manager::K8SManager>
}

impl LokiWorker {

    pub fn collect_without_pods(
        &self,
        svc_name: &str,
        label_name: &str,
        path: &str,
    ) -> Result<()> {
        let loki_cmd = LokiQueryBuilder::new(
            "%Y-%m-%dT%H:%M:%SZ",
            &self.config.param.loki
        )
            .add_query(label_name, svc_name, None, None)
            .add_batch(5000)
            .add_from()
            .add_to()
            .add_forward()
            .add_limit(LIMIT)
            .add_raw()
            .get_query();

        let local_file = format!(
            "{}-{}__{}.log",
            svc_name,
            self.config.param.loki.log_from.unwrap().format("%Y-%m-%d_%H-%M-%S"),
            self.config.param.loki.log_to.unwrap().format("%Y-%m-%d_%H-%M-%S"),
        );

        println!("Loki logs for: {}", svc_name);
        self.collect(loki_cmd.as_str(), local_file.as_str(), path)?;
        Ok(())
    }

    pub fn collect_with_pods(
        &self,
        svc_name: &str,
        label_name: &str,
        path: &str,
    ) -> Result<()> {
        let loki_pods = self.collect_labels("instance")?
            .into_iter()
            .filter(|pod| pod.starts_with(svc_name))
            .collect::<Vec<String>>();
        // TODO добавить tenant в конфиг
        // let labels = &self.config.artifacts.get_svc_names();

        let alive_pods = self.k8s_manager.get_pods()?   
            .items
            .into_iter()
            .filter(|x| x.metadata.name.starts_with(svc_name))
            .map(|x| x.metadata.name)
            .collect::<Vec<_>>();

        let dead_pods = loki_pods
            .iter()
            .filter(|x| !alive_pods.contains(x))
            .collect::<Vec<_>>();

        println!("harvest from alive pods");
        println!("alive pods: {:?}, label_name: {}, svc_name: {}", alive_pods, label_name, svc_name);
        for pod in alive_pods {
            let loki_cmd = LokiQueryBuilder::new(
                "%Y-%m-%dT%H:%M:%SZ",
                &self.config.param.loki
            )
                .add_query(label_name, svc_name, Some("instance"), Some(&pod))
                .add_batch(5000)
                .add_from()
                .add_to()
                .add_forward()
                .add_limit(LIMIT)
                .add_raw()
                .get_query();

            let local_file = format!(
                "{}-{}__{}.log",
                pod,
                self.config.param.loki.log_from.unwrap().format("%Y-%m-%d_%H-%M-%S"),
                self.config.param.loki.log_to.unwrap().format("%Y-%m-%d_%H-%M-%S"),
            );
            self.collect(loki_cmd.as_str(), local_file.as_str(), path)?;
        }
        
        Ok(())
    }

    fn collect(
        &self,
        loki_cmd: &str,
        file: &str,
        path: &str,
    ) -> Result<()> {
        let dest_file = format!("{}/{}", path, file);
        let conn = self.node.get_ssh_conn()?;
        let envs = self.config.get_envs();
        println!("collect query: {}", loki_cmd);
        let res = conn.execute(loki_cmd, envs, None)?;
        fs::write(dest_file, res.join("\n"))?;
        Ok(())
    }

    pub fn collect_labels(&self, label: &str) -> Result<Vec<String>> {
        let loki_cmd = format!("{} labels {}", self.script_head(), label);
        let conn = self.node.get_ssh_conn()?;
        let envs = self.config.get_envs();
        conn.execute(loki_cmd.as_str(), envs, None)
    }

    // TODO удалить
    fn script_head(&self) -> String {
        let cmd = "/opt/logcli";
        let user = self.config.param.loki.login.clone();
        let passwd = self.config.param.loki.password.clone();
        let loki_addr = self.config.param.loki.full_address();
        let org_id = self.config.param.loki.org_id();

        let mut loki_cmd = format!("{} --username=\"{}\" --password=\"{}\" ", cmd, user, passwd);
        loki_cmd = format!("{} --addr=\"{}\" --org-id=\"{}\" -q ", loki_cmd, loki_addr, org_id);
        loki_cmd
    }
}


mod tests {
    use chrono::NaiveDateTime;

    use super::*;

    #[test]
    fn test_query_builder() {

    }

    // #[test]
    // fn test_collect_without_pods_query() {
    //     let mut config = config::LokiConfig::default();
    //     config.login = "admin".to_string();
    //     config.password = "123".to_string();
    //     config.log_from = Some(NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
    //         .unwrap().and_utc());
    //     config.log_to = Some(NaiveDateTime::parse_from_str("2023-01-02 00:00:00", "%Y-%m-%d %H:%M:%S")
    //         .unwrap().and_utc());
    //     let result = LokiQueryBuilder::new("%Y-%m-%d_%H-%M-%S", &config)
    //         .add_query("app", "ptaf-conf-mgr")
    //         .add_batch(5000)
    //         .add_from()
    //         .add_to()
    //         .add_forward()
    //         .add_limit(LIMIT)
    //         .add_raw()
    //         .get_query();
    //     let should_eq = "/opt/logcli --username=\"admin\" --password=\"123\" \
    //     --addr=\"http://loki.ptaf-infra.svc.cluster.local:3100\" --org-id=\"3jqM2DLOMbbQzdodO3cO\" -q query \
    //     \'{app=\"ptaf-conf-mgr\"}\' --batch=5000 --from=\'2023-01-01_00-00-00\' --to=\'2023-01-02_00-00-00\' \
    //     --forward --limit 2000000000 -o raw".to_string();
    //     assert_eq!(result, should_eq);
    // }
}
