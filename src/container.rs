use std::fs;
use std::io::Error;
use std::ffi::CString;
use std::process::{Stdio, Command};

enum ContainerState {
    Initted,

}

#[derive(Debug)]
enum CreateContainerError {
    CreateDir(Error),
    CreateWorkDir(Error),
    CreateTopDir(Error),
    CreateRootDir(Error),
    MountRoot(Error),
    WriteConfig(Error),
    RuncCommand(Error),
    RuncWait(Error),
    RuncCreate(Option<i32>)
}

pub struct Container {
    inst_id: String,
    id: String
}

impl Container {
    pub fn init(inst_id: String, id: String, diffs: &[String], config: String) -> Result<Self, CreateContainerError> { // TODO: errtype
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
        
        fn create_subdirs(inst_id: &str, id: &str) -> Result<(), CreateContainerError> {
            fs::create_dir(format!("/rto/conts/{}/{}/work", inst_id, id)).map_err(CreateContainerError::CreateWorkDir)?;
            fs::create_dir(format!("/rto/conts/{}/{}/top", inst_id, id)).map_err(CreateContainerError::CreateTopDir)?;
            fs::create_dir(format!("/rto/conts/{}/{}/root", inst_id, id)).map_err(CreateContainerError::CreateRootDir)?;
            
            Ok(())
        }
        
        if let Err(err) = create_subdirs(&inst_id, &id) {
            fs::remove_dir(format!("/rto/conts/{}", id)).unwrap();
    
            return Err(err);
        }
        
        let cs_overlay = CString::new("overlay").unwrap();
        let cs_root = CString::new(format!("/rto/conts/{}/root", id)).unwrap();
        let cs_options = CString::new(format!("lowerdir={diffs},upperdir=/rto/conts/{id}/top,workdir=/rto/conts/{id}/work,volatile", id = id, diffs = lowerdir_from_diffs(diffs))).unwrap();
    
        if unsafe { libc::mount(cs_overlay.as_ptr(), cs_root.as_ptr(), cs_overlay.as_ptr(), 0, cs_options.as_ptr() as *const libc::c_void) } != 0 {
            fs::remove_dir(format!("/rto/conts/{}", id)).unwrap();
            
            return Err(CreateContainerError::MountRoot(Error::last_os_error()));
        }
            
        if let Err(err) = fs::write(format!("/rto/conts/{}/config.json", id), config) {
            if unsafe { libc::umount(cs_root.as_ptr()) } != 0 { panic!() };
    
            fs::remove_dir(format!("/rto/conts/{}", id)).unwrap();
    
            return Err(CreateContainerError::WriteConfig(err));
        }
    
        match Command::new("/usr/bin/runc").args(["create", &format!("rto_{}", id)]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(CreateContainerError::RuncCommand)?.wait().map_err(CreateContainerError::RuncWait)?.code() {
            Some(0) => Ok(Self {
                inst_id,
                id
            }),
            code @ (None | Some(_)) => {
                if unsafe { libc::umount(cs_root.as_ptr()) } != 0 { panic!() };
                
                fs::remove_dir(format!("/rto/conts/{}", id)).unwrap();
                
                Err(CreateContainerError::RuncCreate(code))
            }
        }
    }

    pub fn start(&self, inputs: &[u8]) {
        
    }
}