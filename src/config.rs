use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub enum NetworkType {
    Cluster,
    App,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct NodeInfo {
    pub cluster_addr: String,
    pub app_addr: String,
    pub public_addr: String,
}

#[derive(Deserialize, Debug, Clone)]
pub enum JoinStrategy {
    Static,
    Dynamic,
}

pub type NodeList = Vec<NodeInfo>;

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigSchema {
    pub discovery_host: String,
    pub join_strategy: JoinStrategy,
    pub nodes: NodeList,
}
