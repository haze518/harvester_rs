use std::sync::Arc;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::fs;


use crate::ptaf_node;
use crate::config;

const LIMIT: &str = "2000000000";

pub struct LokiWorker {
    pub node: Arc<ptaf_node::PTAFNode>,
    pub config: Arc<config::Config>,
}

impl LokiWorker {

    pub fn collect_without_pods(
        &self,
        svc_name: &str,
        label_name: &str,
        path: &str,
    ) -> Result<()> {
        let query = format!("{{{}=\"{}\"}}", label_name, svc_name);
        let start = format!("{}", self.config.loki.log_from.unwrap().format("%Y-%m-%d_%H-%M-%S"));
        let end = format!("{}", self.config.loki.log_to.unwrap().format("%Y-%m-%d_%H-%M-%S"));
        let local_file = format!("{}-{}__{}.log", svc_name, start, end);

        println!("Loki logs for: {}", svc_name);
        self.collect(query.as_str(), local_file.as_str(), path)?;
        Ok(())
    }

    fn collect(
        &self,
        query: &str,
        file: &str,
        path: &str,
    ) -> Result<()> {
        let out_file = format!("/tmp/{}", file);
        let dest_file = format!("{}/{}", path, file);
        let mut loki_cmd = format!("{} query \'{}\' ", self.script_head(), query);
        let from_str = format!("{}", self.config.loki.log_from.unwrap().format("%Y-%m-%dT%H:%M:%SZ"));
        let to_str = format!("{}", self.config.loki.log_to.unwrap().format("%Y-%m-%dT%H:%M:%SZ"));
        // loki_cmd += format!("--batch=5000 --from=\'{}\' --to=\'{}\' --forward ", from_str, to_str).as_str();
        loki_cmd += format!("--batch=5000 --from=\'{}\' --to=\'{}\' --forward ", from_str, to_str).as_str();
        loki_cmd += format!("--limit {} -o raw", LIMIT).as_str();

        let conn = self.node.get_ssh_conn()?;
        let envs = self.config.get_envs();
        println!("collect query: {}", loki_cmd);
        let res = conn.execute(loki_cmd.as_str(), envs, None)?;
        println!("{:?}", res);
        fs::write(dest_file, res.join("\n"))?;

        // println!("start copy to local");
        // conn.copy_to_local(out_file.as_str(), dest_file.as_str())?;
        // println!("from_file: {}, to_file: {}", dest_file, out_file);
        Ok(())
    }

    pub fn collect_labels(&self, label: &str) -> Result<Vec<String>> {
        let loki_cmd = format!("{} labels {}", self.script_head(), label);
        let conn = self.node.get_ssh_conn()?;
        let envs = self.config.get_envs();
        conn.execute(loki_cmd.as_str(), envs, None)
    }

    fn script_head(&self) -> String {
        let cmd = "/opt/logcli";
        let user = self.config.loki.login.clone();
        let passwd = self.config.loki.password.clone();
        let loki_addr = self.config.loki.full_address();
        let org_id = self.config.loki.org_id.clone();

        let mut loki_cmd = format!("{} --username=\"{}\" --password=\"{}\" ", cmd, user, passwd);
        loki_cmd = format!("{} --addr=\"{}\" --org-id=\"{}\" -q ", loki_cmd, loki_addr, org_id);
        loki_cmd
    }
}

// TODO написать тесты на генерацию команды
