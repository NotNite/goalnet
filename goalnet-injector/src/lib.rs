#![allow(clippy::missing_safety_doc)]

use anyhow::Context;
use dll_syringe::process::{OwnedProcess, Process};
use dll_syringe::Syringe;
pub use goalnet_common::Config;
use goalnet_common::ConfigProcess;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use sysinfo::{PidExt, ProcessExt, SystemExt};

#[cfg(all(feature = "embedded", release))]
static GOALNET_DLL: &[u8] = include_bytes!("../../target/release/goalnet.dll");

#[cfg(all(feature = "embedded", not(release)))]
static GOALNET_DLL: &[u8] = include_bytes!("../../target/debug/goalnet.dll");

type Callback = Arc<Mutex<dyn FnMut() + Send + Sync>>;

pub struct GoalnetProcess {
    config: Config,
    config_file: PathBuf,

    syringe: Syringe,
    dll: PathBuf,
    callback: Option<Callback>,
}

impl GoalnetProcess {
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: FnMut() + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(Mutex::new(callback)));
    }

    pub unsafe fn run(&self) -> anyhow::Result<()> {
        let payload = self
            .syringe
            .inject(&self.dll)
            .context("failed to inject goalnet DLL")?;

        let remote_load = self
            .syringe
            .get_payload_procedure::<fn(Vec<u8>, bool, bool)>(payload, "load")
            .context("couldn't get load procedure")?
            .context("couldn't get load procedure")?;

        let canonicalized = self
            .config_file
            .canonicalize()
            .context("failed to canonicalize config file path")?;

        let config_path_vec = canonicalized
            .to_str()
            .context("failed to convert config file path to string")?
            .as_bytes()
            .to_vec();

        remote_load
            .call(
                &config_path_vec,
                &self.config.goalnet.stderr_log,
                &self.config.goalnet.msgbox_log,
            )
            .context("failed to call load procedure")?;

        if let Some(callback) = &self.callback {
            let mut callback = callback.lock().unwrap();
            callback();
        }

        if self.config.entrypoint.unload {
            let remote_is_unloaded = self
                .syringe
                .get_payload_procedure::<fn() -> bool>(payload, "is_unloaded")
                .context("couldn't get is_unloaded procedure")?
                .context("couldn't get is_unloaded procedure")?;

            loop {
                let is_unloaded = remote_is_unloaded.call().unwrap_or(false);
                if is_unloaded {
                    break;
                }

                if !self.syringe.process().is_alive() {
                    anyhow::bail!("target process died while waiting for unload")
                }

                std::thread::sleep(std::time::Duration::from_millis(250));
            }

            let remote_remove_hostfxr = self
                .syringe
                .get_payload_procedure::<fn()>(payload, "remove_hostfxr")
                .context("couldn't get remove_hostfxr procedure")?
                .context("couldn't get remove_hostfxr procedure")?;
            remote_remove_hostfxr
                .call()
                .context("failed to call remove_hostfxr procedure")?;

            self.syringe
                .eject(payload)
                .context("failed to eject syringe")?;
        }

        Ok(())
    }
}

fn get_process(config: &ConfigProcess) -> anyhow::Result<u32> {
    let system = sysinfo::System::new_all();
    let processes = system.processes();

    if let Some(pid) = config.pid {
        if processes.iter().any(|x| x.0.as_u32() == pid) {
            return Ok(pid);
        }

        anyhow::bail!("no process with given pid found")
    }

    if let Some(name) = &config.name {
        for (pid, process) in processes {
            if process.name() == name {
                return Ok(pid.as_u32());
            }
        }

        anyhow::bail!("no process with given name found")
    } else {
        anyhow::bail!("specify one of process name or pid")
    }
}

pub fn inject(config_file: &Path) -> anyhow::Result<GoalnetProcess> {
    let config = goalnet_common::parse(config_file).context("failed to parse config file")?;

    let dll = if let Some(goalnet_path) = &config.goalnet.path {
        let path = goalnet_common::relative_dir(config_file, &PathBuf::from(goalnet_path))
            .context("failed to get goalnet DLL path")?;

        if config.payload.copy_build_artifacts {
            let temp =
                std::env::temp_dir().join(format!("{}.dll", goalnet_common::temp_filename()));

            std::fs::copy(path, &temp).context("failed to copy goalnet DLL")?;

            temp
        } else {
            path
        }
    } else {
        #[cfg(feature = "embedded")]
        {
            let temp =
                std::env::temp_dir().join(format!("{}.dll", goalnet_common::temp_filename()));
            std::fs::write(&temp, GOALNET_DLL).context("failed to write goalnet DLL")?;
            temp
        }

        #[cfg(not(feature = "embedded"))]
        {
            anyhow::bail!("goalnet path not specified in config file");
        }
    };

    let pid = get_process(&config.process).context("failed to get target process pid")?;
    //println!("injecting into process {}...", pid);
    let process = OwnedProcess::from_pid(pid).context("failed to get target process")?;
    let syringe = Syringe::for_process(process);

    Ok(GoalnetProcess {
        config,
        config_file: config_file.to_owned(),

        syringe,
        dll: dll.clone(),
        callback: None,
    })

    /*
    unsafe {


        //println!("injected successfully!");

        if config.entrypoint.unload {
            //println!("waiting for unload...");
            let remote_is_unloaded = syringe
                .get_payload_procedure::<fn() -> bool>(payload, "is_unloaded")
                .context("couldn't get is_unloaded procedure")?
                .context("couldn't get is_unloaded procedure")?;

            loop {
                let is_unloaded = remote_is_unloaded.call().unwrap_or(false);
                if is_unloaded {
                    break;
                }

                if !syringe.process().is_alive() {
                    anyhow::bail!("target process died while waiting for unload");
                }

                std::thread::sleep(std::time::Duration::from_millis(250));
            }

            //println!("unloading hostfxr...");
            let remote_remove_hostfxr = syringe
                .get_payload_procedure::<fn()>(payload, "remove_hostfxr")
                .context("couldn't get remove_hostfxr procedure")?
                .context("couldn't get remove_hostfxr procedure")?;
            remote_remove_hostfxr
                .call()
                .context("failed to call remove_hostfxr procedure")?;

            //println!("ejecting syringe...");
            syringe.eject(payload).context("failed to eject syringe")?;
        }
    }
    */
}
