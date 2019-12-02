use serde::Deserialize;

#[derive(Clone, Debug, PartialEq)]
pub enum NetworkType {
    Cluster,
    App,
}

#[derive(Deserialize, Debug, Clone)]
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
    pub join_strategy: JoinStrategy,
    pub nodes: NodeList,
}
