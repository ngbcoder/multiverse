use crate::config::Config;
use crate::discovery::ImageMeta;

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32 {
        0
    }
    fn check(&self, image: &ImageMeta, config: &Config) -> bool;
    fn apply(&self, image: &ImageMeta, config: &Config);
}

pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn register(&mut self, p: Box<dyn Plugin>) {
        self.plugins.push(p);
        self.plugins.sort_by_key(|p| std::cmp::Reverse(p.priority()));
    }

    pub fn applicable<'a>(&'a self, image: &ImageMeta, config: &Config) -> Vec<&'a dyn Plugin> {
        self.plugins
            .iter()
            .map(|b| b.as_ref())
            .filter(|p| p.check(image, config))
            .collect()
    }

    pub fn run(&self, image: &ImageMeta, config: &Config) {
        for p in self.applicable(image, config) {
            p.apply(image, config);
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}