use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct CommandLine {
    pub debug: bool,
    pub fallback: bool,
    pub memdisk: bool,
    pub sector_size: Option<u32>,
    pub flags: BTreeMap<String, String>,
    pub positional: Vec<String>,
}

impl CommandLine {
    pub fn parse(s: &str) -> Self {
        let mut cl = CommandLine::default();
        for tok in s.split_whitespace() {
            if let Some((k, v)) = tok.split_once('=') {
                match k {
                    "debug" => cl.debug = true,
                    "fallback" => cl.fallback = true,
                    "memdisk" => cl.memdisk = true,
                    "sector_size" | "sector-size" => {
                        cl.sector_size = v.parse().ok();
                    }
                    _ => {
                        cl.flags.insert(k.to_string(), v.to_string());
                    }
                }
                // also handle bare key with value true?
            } else {
                match tok {
                    "debug" => cl.debug = true,
                    "fallback" => cl.fallback = true,
                    "memdisk" => cl.memdisk = true,
                    _ => cl.positional.push(tok.to_string()),
                }
            }
        }
        cl
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.flags.get(key).map(|s| s.as_str())
    }
}