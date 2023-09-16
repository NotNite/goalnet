use std::path::PathBuf;
use clap::Parser;
use dll_syringe::process::{OwnedProcess, Process};
use dll_syringe::Syringe;
use sysinfo::{PidExt, ProcessExt, SystemExt};
use goalnet_common::ConfigProcess;

#[cfg(release)]
static GOALNET_DLL: &[u8] = include_bytes!("../../target/release/goalnet.dll");

#[cfg(not(release))]
static GOALNET_DLL: &[u8] = include_bytes!("../../target/debug/goalnet.dll");

#[derive(Parser)]
struct Args {
    config_path: PathBuf,
}

fn get_process(config: &ConfigProcess) -> Result<u32, Box<dyn std::error::Error>> {
    let system = sysinfo::System::new_all();
    let processes = system.processes();

    if let Some(pid) = config.pid {
        if processes.iter().any(|x| x.0.as_u32() == pid) {
            return Ok(pid);
        }

        panic!("no process with given pid found")
    }

    if let Some(name) = &config.name {
        for (pid, process) in processes {
            if process.name() == name {
                return Ok(pid.as_u32());
            }
        }

        panic!("no process with given name found")
    } else {
        panic!("specify one of process name or pid")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = goalnet_common::parse(&args.config_path)
        .expect("failed to parse config file");


    let dll = if let Some(goalnet_path) = config.goalnet.path {
        let path = goalnet_common::relative_dir(&args.config_path, &PathBuf::from(goalnet_path))
            .expect("failed to get goalnet DLL path");

        if config.payload.copy_build_artifacts {
            let temp = std::env::temp_dir()
                .join(format!("{}.dll", goalnet_common::temp_filename()));

            std::fs::copy(path, &temp)
                .expect("failed to copy goalnet DLL");

            temp
        } else {
            path
        }
    } else {
        let temp = std::env::temp_dir()
            .join(format!("{}.dll", goalnet_common::temp_filename()));

        std::fs::write(&temp, GOALNET_DLL)
            .expect("failed to write goalnet DLL");

        temp
    };

    let pid = get_process(&config.process).expect("failed to get target process pid");
    println!("injecting into process {}...", pid);
    let process = OwnedProcess::from_pid(pid).expect("failed to get target process");
    let syringe = Syringe::for_process(process);

    let payload = syringe.inject(dll).expect("failed to inject goalnet DLL");

    unsafe {
        let remote_load = syringe
            .get_payload_procedure::<fn(Vec<u8>, bool, bool)>(payload, "load")
            .expect("couldn't get load procedure")
            .expect("couldn't get load procedure");

        let canonicalized = args.config_path.canonicalize()
            .expect("failed to canonicalize config file path");
        let config_path_vec = canonicalized.to_str()
            .expect("failed to convert config path to string")
            .as_bytes().to_vec();

        remote_load
            .call(
                &config_path_vec,
                &config.goalnet.stderr_log,
                &config.goalnet.msgbox_log,
            )
            .expect("couldn't call load procedure");

        println!("injected successfully!");

        if config.entrypoint.unload {
            println!("waiting for unload...");
            let remote_is_unloaded = syringe
                .get_payload_procedure::<fn() -> bool>(payload, "is_unloaded")
                .expect("couldn't get is_unloaded procedure")
                .expect("couldn't get is_unloaded procedure");

            loop {
                let is_unloaded = remote_is_unloaded.call().unwrap_or(false);
                if is_unloaded {
                    break;
                }

                if !syringe.process().is_alive() {
                    panic!("process died while waiting for unload");
                }

                std::thread::sleep(std::time::Duration::from_millis(250));
            }

            println!("unloading hostfxr...");
            let remote_remove_hostfxr = syringe
                .get_payload_procedure::<fn()>(payload, "remove_hostfxr")
                .expect("couldn't get remove_hostfxr procedure")
                .expect("couldn't get remove_hostfxr procedure");
            remote_remove_hostfxr.call().expect("couldn't call remove_hostfxr procedure");

            println!("ejecting syringe...");
            syringe.eject(payload).expect("failed to eject syringe");
        }
    }

    Ok(())
}
