//! Device fingerprinting and anomaly detection

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFingerprint {
    pub user_agent: String,
    pub screen_resolution: Option<String>,
    pub timezone: Option<String>,
    pub language: Option<String>,
    pub platform: Option<String>,
    pub canvas_fingerprint: Option<String>,
    pub webgl_vendor: Option<String>,
    pub webgl_renderer: Option<String>,
    pub fonts: Option<Vec<String>>,
    pub ip_subnet: String, // /24 for IPv4, /48 for IPv6
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnomalyLevel {
    None,
    Minor,    // Different browser, same location
    Major,    // Different device or location
    Critical, // Multiple major changes
}

impl DeviceFingerprint {
    /// Generate a hash of the fingerprint
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        
        // Include stable components in hash
        hasher.update(&self.user_agent);
        if let Some(ref res) = self.screen_resolution {
            hasher.update(res);
        }
        if let Some(ref platform) = self.platform {
            hasher.update(platform);
        }
        if let Some(ref canvas) = self.canvas_fingerprint {
            hasher.update(canvas);
        }
        if let Some(ref vendor) = self.webgl_vendor {
            hasher.update(vendor);
        }
        if let Some(ref renderer) = self.webgl_renderer {
            hasher.update(renderer);
        }
        
        format!("{:x}", hasher.finalize())
    }
    
    /// Extract subnet from IP address
    pub fn subnet_from_ip(ip: &str) -> String {
        match ip.parse::<IpAddr>() {
            Ok(IpAddr::V4(addr)) => {
                // Get /24 subnet
                let octets = addr.octets();
                format!("{}.{}.{}.0/24", octets[0], octets[1], octets[2])
            }
            Ok(IpAddr::V6(addr)) => {
                // Get /48 subnet (first 3 segments)
                let segments = addr.segments();
                format!("{:x}:{:x}:{:x}::/48", segments[0], segments[1], segments[2])
            }
            Err(_) => ip.to_string(),
        }
    }
    
    /// Detect anomalies between two fingerprints
    pub fn detect_anomaly(&self, other: &DeviceFingerprint) -> AnomalyLevel {
        let mut changes = 0;
        let mut major_changes = 0;
        
        // User agent change (minor unless OS change)
        if self.user_agent != other.user_agent {
            changes += 1;
            // Check if OS changed
            if extract_os(&self.user_agent) != extract_os(&other.user_agent) {
                major_changes += 1;
            }
        }
        
        // Screen resolution (minor)
        if self.screen_resolution != other.screen_resolution {
            changes += 1;
        }
        
        // Timezone change (major if > 3 hours)
        if self.timezone != other.timezone {
            if let (Some(tz1), Some(tz2)) = (&self.timezone, &other.timezone) {
                if timezone_diff_hours(tz1, tz2) > 3 {
                    major_changes += 1;
                } else {
                    changes += 1;
                }
            }
        }
        
        // Platform change (major)
        if self.platform != other.platform {
            major_changes += 1;
        }
        
        // Canvas fingerprint change (major)
        if self.canvas_fingerprint != other.canvas_fingerprint {
            major_changes += 1;
        }
        
        // WebGL change (major)
        if self.webgl_vendor != other.webgl_vendor || 
           self.webgl_renderer != other.webgl_renderer {
            major_changes += 1;
        }
        
        // IP subnet change (major if different subnet)
        if self.ip_subnet != other.ip_subnet {
            major_changes += 1;
        }
        
        // Determine anomaly level
        if major_changes >= 3 {
            AnomalyLevel::Critical
        } else if major_changes >= 1 {
            AnomalyLevel::Major
        } else if changes >= 2 {
            AnomalyLevel::Minor
        } else {
            AnomalyLevel::None
        }
    }
}

/// Extract OS from user agent
fn extract_os(user_agent: &str) -> &str {
    if user_agent.contains("Windows") {
        "Windows"
    } else if user_agent.contains("Mac OS") || user_agent.contains("macOS") {
        "macOS"
    } else if user_agent.contains("Linux") {
        "Linux"
    } else if user_agent.contains("Android") {
        "Android"
    } else if user_agent.contains("iOS") || user_agent.contains("iPhone") {
        "iOS"
    } else {
        "Unknown"
    }
}

/// Calculate timezone difference in hours
fn timezone_diff_hours(tz1: &str, tz2: &str) -> i32 {
    // Simple implementation - in production use proper timezone library
    let offset1 = parse_timezone_offset(tz1);
    let offset2 = parse_timezone_offset(tz2);
    (offset1 - offset2).abs()
}

fn parse_timezone_offset(tz: &str) -> i32 {
    // Parse GMT+X or GMT-X format
    if let Some(offset_str) = tz.strip_prefix("GMT") {
        offset_str.parse::<i32>().unwrap_or(0)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_subnet_extraction() {
        assert_eq!(
            DeviceFingerprint::subnet_from_ip("192.168.1.100"),
            "192.168.1.0/24"
        );
    }
    
    #[test]
    fn test_anomaly_detection() {
        let fp1 = DeviceFingerprint {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string(),
            screen_resolution: Some("1920x1080".to_string()),
            timezone: Some("GMT-5".to_string()),
            language: Some("en-US".to_string()),
            platform: Some("Win32".to_string()),
            canvas_fingerprint: Some("abc123".to_string()),
            webgl_vendor: Some("Intel Inc.".to_string()),
            webgl_renderer: Some("Intel Iris".to_string()),
            fonts: None,
            ip_subnet: "192.168.1.0/24".to_string(),
        };
        
        let mut fp2 = fp1.clone();
        fp2.user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15)".to_string();
        fp2.platform = Some("MacIntel".to_string());
        
        assert_eq!(fp1.detect_anomaly(&fp2), AnomalyLevel::Major);
    }
}