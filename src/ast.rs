use std::collections::HashMap;
use std::sync::Arc;
use std::cell::RefCell;

pub struct Config {
    entries: HashMap<Arc<String>, RefCell<Arc<String>>>
}

pub struct Register {
    name: Arc<String>, 
    mappings: Vec<(Arc<String>, u32)>
}

pub enum RalEntry {
    RawRegister(Register), 
    Alias(Arc<String>), 
    RestrictedAlias(Arc<String>, Arc<String>)
}

pub struct Ral {
    config: Config, 
    registers: HashMap<Arc<String>, RalEntry>
}
