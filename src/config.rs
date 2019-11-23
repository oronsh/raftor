use serde::Deserialize;

#[derive(Clone, Debug)]
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

pub type NodeList = Vec<NodeInfo>;

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigSchema {
    pub nodes: NodeList,
}
