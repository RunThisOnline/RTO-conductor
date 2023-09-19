use std::sync::{Arc, Mutex};
use std::fs::{self, File};
use std::ffi::CString;
use std::process::{Command, Stdio};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::thread;
use serde::Deserialize;
use serde_json::Value;
use std::sync::mpsc;

mod io_bin;

pub type Try<T> = Option<T>; // Don't care about the contents of errors

const BASE_OCI_CONFIG: &str = include_str!("../base_oci_config.json");

/*#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
struct OciConfig {
    oci_version: String,
    root: OciConfigRoot,
    mounts: Vec<OciConfigMount>,
    hostname: Option<String>,
    domainname: Option<String>,
    linux: OciConfigLinux
    annotations: HashMap<String, String>
}

#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
struct OciConfigRoot {
    path: String,
    readonly: Option<bool>
}

#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
struct OciConfigMount {
    destination: String,
    source: Option<String>,
    options: Option<Vec<String>>
}

#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
struct OciConfigLinux {
    namespaces: Vec<OciConfigLinuxNamespace>,
    devices: Vec<OciConfigLinuxDevice>,
    cgroupsPath: Option<String>,
    resources: Option<OciConfigLinuxResources>,
    sysctl: Option<HashMap<String, String>>,
    seccomp: Option<OciConfigLinuxSeccomp>,
    
}*/

#[derive(Deserialize)]
struct Config {
    diffs: Vec<String>
}

enum Mode {
    SingleCase,
    MultiCase(usize),
    Tty
}

#[derive(Debug)]
enum CreateContainerError {
    CreateDir(io::Error),
    CreateWorkDir(io::Error),
    CreateTopDir(io::Error),
    CreateRootDir(io::Error),
    MountRoot(io::Error),
    WriteConfig(io::Error),
    RuncCommand(io::Error),
    RuncWait(io::Error),
    RuncCreate(Option<i32>)
}

fn create_container(id: usize, diffs: &[String], config: &str) -> Result<(), CreateContainerError> {
    fn lowerdir_from_diffs(diffs: &[String]) -> String {
        let mut lowerdir = String::new();
        let mut is_first: bool = true;
        
        for diff in diffs {
            if is_first {
                is_first = false;
            } else {
                lowerdir.push(':');
            }
            
            lowerdir += "/rto/imgs/diffs/";
            lowerdir += diff;
        }
        
        lowerdir
    }
    
    fs::create_dir(format!("/rto/conts/{}", id)).map_err(CreateContainerError::CreateDir)?;
    
    fn create_subdirs(id: usize) -> Result<(), CreateContainerError> {
        fs::create_dir(format!("/rto/conts/{}/work", id)).map_err(CreateContainerError::CreateWorkDir)?;
        fs::create_dir(format!("/rto/conts/{}/top", id)).map_err(CreateContainerError::CreateTopDir)?;
        fs::create_dir(format!("/rto/conts/{}/root", id)).map_err(CreateContainerError::CreateRootDir)?;
        
        Ok(())
    }
    
    if let Err(err) = create_subdirs(id) {
        fs::remove_dir(format!("/rto/conts/{}", id)).unwrap();

        return Err(err);
    }
    
    let cs_overlay = CString::new("overlay").unwrap();
    let cs_root = CString::new(format!("/rto/conts/{}/root", id)).unwrap();
    let cs_options = CString::new(format!("lowerdir={diffs},upperdir=/rto/conts/{id}/top,workdir=/rto/conts/{id}/work,volatile", id = id, diffs = lowerdir_from_diffs(diffs))).unwrap();

    if unsafe { libc::mount(cs_overlay.as_ptr(), cs_root.as_ptr(), cs_overlay.as_ptr(), 0, cs_options.as_ptr() as *const libc::c_void) } != 0 {
        fs::remove_dir(format!("/rto/conts/{}", id)).unwrap();
        
        return Err(CreateContainerError::MountRoot(io::Error::last_os_error()));
    }
        
    if let Err(err) = fs::write(format!("/rto/conts/{}/config.json", id), config) {
        if unsafe { libc::umount(cs_root.as_ptr()) } != 0 { panic!() }; // umount2 instead?

        fs::remove_dir(format!("/rto/conts/{}", id)).unwrap();

        // TODO: have a way to mark sticky/broken container attempts instead of panicking, assuming they are clean

        return Err(CreateContainerError::WriteConfig(err));
    }

    match Command::new("/usr/bin/runc").args(["create", &format!("rto_{}", id)]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(CreateContainerError::RuncCommand)?.wait().map_err(CreateContainerError::RuncWait)?.code() {
        Some(0) => Ok(()),
        code @ (None | Some(_)) => {
            if unsafe { libc::umount(cs_root.as_ptr()) } != 0 { panic!() };
            
            fs::remove_dir(format!("/rto/conts/{}", id)).unwrap();
            
            Err(CreateContainerError::RuncCreate(code))
        }
    }
}

fn oci_config_from_config(config: &Config) -> String {
    let /*mut*/ oci_config: HashMap<String, Value> = serde_json::from_str(BASE_OCI_CONFIG).unwrap(); // stupid rust won't let me do this at compile time >:|
    
    // change hostname?
    // force root path?
    // make changes based on config?
    //   - namespaces
    //   - mounts
    //   - resources?
    
    serde_json::to_string(&oci_config).unwrap()
}

/*enum InstState {
    Init {
        conts: Vec<usize>
    },
    PostStart {
        conts: Vec<usize>
    }
}

struct InstInner {
    config: Config,
    mode: Mode,
    c_id: Arc<Mutex<usize>>,
    state: Mutex<InstState>,
    conts: Mutex<Vec<usize>>
}

#[derive(Clone)]
struct Inst(Arc<InstInner>);

impl Inst {
    fn init(config: Config, mode: Mode, c_id: Arc<Mutex<usize>>) -> Result<Inst, CreateContainerError> {
        let num_cases = match mode {
            Mode::SingleCase => 1,
            Mode::MultiCase(num_cases) => num_cases,
            Mode::Tty => 1
        };
        
        let mut conts: Vec<usize> = Vec::new();

        for _ in 0..num_cases {
            let id = *c_id.lock().unwrap();

            *c_id.lock().unwrap() += 1;

            create_container(id, &config.diffs, &oci_config_from_config(&config))?;
            conts.push(id);
        }
        
        Ok(Inst(Arc::new(InstInner {
            config,
            mode,
            c_id,
            state: Mutex::new(InstState::Init),
            conts: Mutex::new(conts)
        })))
    }
    
    fn start(&self) -> Result<(), ()> {
        let mut state = self.0.state.lock().unwrap();
        
        *state = match *state {
            InstState::Init => InstState::Start,
            _ => panic!()
        };
        
        thread::spawn(move || {
            self.kill();
        });
        
        Ok(())
    }
    
    fn input(&self, stream: usize, data: &[u8]) {
        unimplemented!();
    }
    
    fn stop(&self) {
        unimplemented!();
    }
    
    fn kill(&self) {
        unimplemented!();
    }
}*/

fn random_inst_id(insts: &HashMap<usize, Inst>) -> usize {
    loop {
        let id = rand::random::<usize>();
        
        if !insts.contains_key(&id) {
            break id;
        }
    }
}

fn main() {
    let mut stdin = io::stdin().lock();
    
    let mut insts: HashMap<usize, Inst> = HashMap::new();
    
    macro_rules! byte {
        () => {
            {
                let mut buf: [u8; 1] = [0];

                stdin.read_exact(&mut buf).unwrap();

                buf[0]
            }
        }
    }

    macro_rules! int {
        () => {
            {
                let mut int: usize = 0;

                loop {
                    let byte = byte!();

                    int = int.checked_mul(128).unwrap().checked_add((byte % 128) as usize).unwrap();

                    if byte < 128 {
                        break;
                    }
                }

                int
            }
        }
    }

    macro_rules! bytestring {
        () => {
            {
                let size: usize = int!();

                let mut string: Vec<u8> = vec![0; size];

                stdin.read_exact(&mut string).unwrap();

                string
            }
        }
    }
    
    let (output_p, output_c) = mpsc::channel::<Vec<u8>>();
    
    thread::spawn(move || {
        while let Ok(output) = output_c.recv() {
            io::stdout().lock().write_all(&output).unwrap();
        }
    });
    
    loop {
        match byte!() {
            config_src @ (0x00 | 0x01) => {
                let config = match config_src {
                    0x00 => {
                        let id_string = bytestring!();

                        let lang_id = std::str::from_utf8(&id_string).unwrap();
                        
                        serde_json::from_reader(File::open(format!("/rto/imgs/configs/{}.json", lang_id)).unwrap()).unwrap()
                    }
                    0x01 => serde_json::from_slice(&bytestring!()).unwrap(),
                    _ => unreachable!()
                };
                
                let mode = match byte!() {
                    0x00 => Mode::SingleCase,
                    0x01 => Mode::MultiCase(int!()),
                    0x02 => Mode::Tty,
                    _ => panic!()
                };
                
                let id = random_inst_id(&insts);

                match Inst::init(config, mode, id) {
                    Ok(inst) => {
                        insts.insert(id, inst);

                        io::stdout().lock().write_all(&[&[0x80u8, 0x00u8], &id.to_be_bytes()[..]].concat()).unwrap();
                    }
                    Err(err) => io::stdout().lock().write_all(&[&[0x80u8, 0x01u8], &format!("{:?}", err).as_bytes()[..]].concat()).unwrap()
                }
            }
            0x10 => {
                let inst_id = int!();
                let inst = insts.get(&inst_id).unwrap().clone();
                
                thread::spawn(move || {
                    match inst.start() {
                        Ok(()) => io::stdout().lock().write_all(&[&[0x81u8, 0x00u8], &inst_id.to_be_bytes()[..]].concat()).unwrap(),
                        Err(err) => io::stdout().lock().write_all(&[&[0x81u8, 0x01u8], &inst_id.to_be_bytes()[..], &format!("{:?}", err).as_bytes()[..]].concat()).unwrap()
                    }
                });
            }
            0x11 => {
                let inst_id = int!();
                
                insts.get_mut(&inst_id).unwrap().input(int!(), &bytestring!());
            }
            0x12 => {
                let inst_id = int!();
                
                insts.get_mut(&inst_id).unwrap().stop();
            }
            _ => panic!()
        }
    }
}