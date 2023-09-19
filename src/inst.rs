use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde_json::Value;
use std::io::Cursor;

use crate::io_bin::InputStream;
use crate::{Config, Mode};
use crate::container::Container;
use crate::BASE_OCI_CONFIG;

struct Inst {
    id: String,
    config: Config,
    mode: Mode,
    conts: Vec<Container>
}

pub struct InstFront {
    inner: Arc<Mutex<Inst>>
}

fn oci_config_from_config(config: &Config, inst_id: &str, id: &str) -> String {
    let mut oci_config: HashMap<String, Value> = serde_json::from_str(BASE_OCI_CONFIG).unwrap(); // stupid rust won't let me do this at compile time >:|
    
    oci_config.insert("hostname".to_owned(), Value::String([oci_config.get("hostname").unwrap().as_str().unwrap(), "-", inst_id, "-", id].concat())); // hostname can be assumed to be a string that exists

    // make changes based on config?
    //   - namespaces
    //   - mounts
    //   - resources?
    
    serde_json::to_string(&oci_config).unwrap()
}

impl InstFront {
    pub fn init(id: String, config: Config, mode: Mode) -> Result<InstFront, ()> { // TODO: errtype
        let cases = match inner.mode {
            Mode::SingleCase | Mode::Tty => 1,
            Mode::MultiCase(cases) => cases
        };

        let conts: Vec<Container> = Vec::with_capacity(cases);

        for cont_id in 0..cases {
            conts.push(Container::init(id.clone(), cont_id.to_string(), &config.diffs, oci_config_from_config(&config, &id, &cont_id.to_string())).map_err(|_| ())?);
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(Inst {
                id,
                config,
                mode,
                conts
            }))
        })
    }

    pub fn start(&self, inputs: &[u8]) {
        let inner = self.inner.lock().unwrap();

        match inner.mode {
            Mode::SingleCase | Mode::Tty => inner.conts[0].start(inputs),
            Mode::MultiCase(_) => {
                let mut cursor = Cursor::new(inputs);
                let common = cursor.input_string().unwrap();

                for cont in inner.conts {
                    cont.start(&[common, cursor.input_string().unwrap()].concat());
                }
            }
        }
    }
}