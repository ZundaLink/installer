use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerConfig {
    pub latest_version: String,
    pub version_list: Vec<VersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub name: String,
    pub summary: String,
    pub install_list: Vec<InstallFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallFile {
    pub filename: String,
    pub size: u64,
    pub sha256: String,
    pub url_list: Vec<String>,
}

impl InstallerConfig {
    pub async fn fetch() -> anyhow::Result<Self> {
        let url = "https://api.xn--igrr70arr3c.vip/zundalink/api/v0/installer/config/get";
        let response = reqwest::get(url).await?;
        let config = response.json::<InstallerConfig>().await?;
        Ok(config)
    }
    
    pub fn get_latest_version(&self) -> Option<&VersionInfo> {
        self.version_list.iter()
            .find(|v| v.name == self.latest_version)
            .or_else(|| self.version_list.first())
    }
    
    pub fn get_version_by_name(&self, name: &str) -> Option<&VersionInfo> {
        self.version_list.iter().find(|v| v.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_config() {
        let json_data = r#"{
            "latest_version": "TEST_1.00.00",
            "version_list": [
                {
                    "name": "TEST_1.00.00",
                    "summary": "TEST-VERSION",
                    "install_list": [
                        {
                            "filename": "test.pack",
                            "size": 100,
                            "sha256": "abc123",
                            "url_list": ["https://example.com/test.pack"]
                        }
                    ]
                }
            ]
        }"#;
        
        let config: InstallerConfig = serde_json::from_str(json_data).unwrap();
        assert_eq!(config.latest_version, "TEST_1.00.00");
        assert_eq!(config.version_list.len(), 1);
        assert_eq!(config.version_list[0].name, "TEST_1.00.00");
    }
}
