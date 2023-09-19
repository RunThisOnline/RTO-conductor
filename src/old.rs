use vsock::{VsockStream, VsockAddr};
use std::thread;
use std::fs::{File, Path};

// Conductor is a contract; will panic on any strange behavior from host

enum Mode {
    SingleCase,
    MultiCase,
    Tty
}

impl Mode {
    fn is_multi(&self) -> bool {
        match Mode {
            SingleCase => false
            MultiCase => true,
            Tty = false
        }
    }
}

#[derive(Deserialize)]
struct Config {
    diffs: Vec<String>,
    container_config: Option<ContainerConfig>,
    staging: Vec<StagingDir>
}

#[derive(Deserialize)]
struct ContainerConfig {
    
}

#[derive(Deserialize)]
#[serde(tag = "type")]
#[serde(rename = "snake_case")]
enum StagingDir {
    WriteFile {
        file: String,
        src: Input,
        #[serde(default)]
        append: bool,
        exists: Option<bool>
    }
    OutputFile {
        file: String,
        dst: Output
    }
    Run {
        bin: String,
        tty: Option<bool>,
        cwd: String,
        user: User,
        stdin: Input,
        stdout: Output,
        stderr: Output,
        args: Args,
        env: Env,
        ignore_code: Option<bool>,
        success_codes: Option<Vec<i8>>,
        fail_codes: Option<Vec<i8>>
    }
    CloseStream {
        id: String
    }
    Simul {
        blocks: Vec<Vec<StagingDir>>
    }
}

#[derive(Deserialize)]
struct User {
    uid: u16,
    gid: u16,
    umask: Option<u16>,
    additionalGids: Option<Vec<u16>>
}

#[derive(Deserialize)]
#[serde(tag = "type")]
#[serde(rename = "snake_case")]
enum Input {
    Const {
        #[serde(rename = "const")]
        const_str: String
    }
    String {
        id: String
    }
    Stream {
        id: String
    }
    Concat {
        strings: Vec<Input>
    }
}

enum Output {
    Ignore,
    
}

#[derive(Serialize)]
struct Config {
    staging: Vec<TopStagingDir>
}

#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
enum TopStagingDir {
    OutputFile {
        file: String,
        dst: Dest,
        #[serde(default)]
        close: bool
    }
    WriteFile {
        file: String,
        src: Source,
        #[serde(default)]
        append: bool,
        exists: Option<bool>
    }
    Run {
        run: String,
        tty: Option<bool>,
        cwd: Option<String>,
        stdin: Option<Source>,
        stdout: Option<Dest>,
        stderr: Option<Dest>,
        args: ArgsSource,
        env: EnvSource,
        process_config: Option<ProcessConfig>,
        #[serde(default)]
        ignore_code: Option<bool>,
        success_codes: Option<Vec<i8>>,
        fail_codes: Option<Vec<i8>>
        /*rlimits: Option<Vec<Rlimit>>,
        capabilities: Option<Capabilities>,
        stripCapabilities: Option<Capabilities>
        #[serde(default)]
        noBaseCapabilities: bool,
        noNewPrivileges: Option<bool>,
        oomScoreAdj: Option<i16>,
        scheduler: Scheduler,
        ioPriority: IoPriority,
        uid: Option<u16>,
        gid: Option<u16>,
        umask: Option<u16>,
        additionalGids: Option<Vec<u16>>,
        hostname: Option<Source>,
        domainname: Option<Source>*/
    }
    Simul {
        directives: Vec<TopStagingDir>,
        #[serde(default)]
        finish_on_first_fail: bool,
        #[serde(default)]
        kill_on_first_fail: bool,
        #[serde(default)]
        stop_file_io_on_first_fail: bool
    }
    CloseStream {
        stream: String
    }
    WaitStream {
        stream: String
    }
    Conditional {
        condition: Condition,
        directives: Vec<StagingDir>
    }
    ForkCases {
        directives: Vec<PostForkStagingDir>,
        simul: bool
    }
    SpawnContainer {
        directives: Vec<PostSpawnStagingDir>,
        diffs: Vec<String>,
        container_config: Option<ContainerConfig>
    }
}

#[derive(Serialize)]
enum PostForkStagingDir {
    Dirs(Vec<PostForkStagingDir>),
    Dir(PostForkStagingDir)
}

#[derive(Serialize)]
enum PostSpawnStagingDir {
    OutputFile {
        file: String,
        dst: Dest,
        #[serde(default)]
        close: bool
    }
    WriteFile {
        file: String,
        src: Source,
        #[serde(default)]
        append: bool,
        exists: Option<bool>
    }
    Run {
        run: String,
        tty: Option<bool>,
        cwd: Option<String>,
        stdin: Option<Source>,
        stdout: Option<Dest>,
        stderr: Option<Dest>,
        args: ArgsSource,
        env: EnvSource,
        process_config: Option<ProcessConfig>,
        #[serde(default)]
        ignore_code: Option<bool>,
        success_codes: Option<Vec<i8>>,
        fail_codes: Option<Vec<i8>>
    }
}

#[derive(Serialize)]
enum PostForkPostSpawnStagingDir {
    Dirs(Vec<PostForkPostSpawnStagingDir>),
    Dir(PostForkPostSpawnStagingDir)
}

fn init_conductor() {
    let stream = VsockStream::connect_with_cid_port(2, -1).unwrap();
    
    macro_rules! byte {
        () => {
            let mut buf: [u8; 1] = [0];
            
            /*loop {
                match stream.read(&mut buf) {
                    Ok(0) => panic!("stream.read: unexpected EOF"),
                    Ok(1) => break buf[0],
                    Ok(bytes) => panic!("'buf' is size 1 but stream.read read {} bytes", bytes),
                    Err(err) => match err.raw_os_error().expect("raw_os_error() is None on read err") {
                        libc::EINTR => {}
                        errno => panic!("stream.read errored with errno {}", errno)
                    }
                }
            }*/
            
            stream.read_exact(&mut buf).unwrap();
            buf[0]
        }
    }
    
    macro_rules! int {
        () => {
            let size = 0;
            
            loop {
                let byte = byte!();
                
                size = size.checked_mul(128).unwrap().checked_add(byte % 128);
                
                if byte < 128 {
                    break;
                }
            }
            
            size
        }
    }
    
    macro_rules! string {
        () => {
            let size: usize = int!();
            
            let string: Vec<u8> = vec![0; size];
            
            match stream.read_exact(&mut string) {
                Ok(()) => string,
                Err(err) => match err {
                    std::io::ErrorKind::UnexpectedEof => panic!("stream.read_exact: unexpected EOF"),
                    _ => panic!("stream.read_exact: {}", err)
                }
            }
        }
    }
    
    loop {
        match byte!() {
            0x00 => { // init container with config ID
                let lang_id = str::from_utf8(string!()).unwrap();
                
                let config = serde_json::from_reader(File::open(Path::new(format!("/rto/imgs/configs/{}.json", lang_id))).unwrap()).unwrap(); // Trusts that lang_id is valid
                
                let mode = match byte!() {
                    0x00 => Mode::SingleCase,
                    0x01 => Mode::MultiCase,
                    0x02 => Mode::Tty,
                    _ => panic!()
                };
                
                if mode.is_multi() {
                    let cases = int!();
                    let simul = match byte!() {
                        0x00 => true,
                        0x01 => false
                    };
                }
                
                let num_cases: u32 = if mode.is_multi() { int!() } else { 1 };
                
                let containers: Vec<Container> = Vec::with_capacity(num_cases);
                
                for _ in 0..num_cases {
                    containers.push(provision_container());
                }
            }
            0x01 => { // init container with custom config
                let config = serde_json::from_slice(string!()).unwrap();
                
                
            }
            0x10 => { // start running, provide code + strings
                
            }
            0x11 => { // stream input
                
            }
            0x12 => { // stop
                
            }
            command => panic!("received unknown command type {}", command)
        }
    }
}

fn kill_soon() {
    
}

fn main() {
     // 2: host CID, -1: any port
    
    
}
