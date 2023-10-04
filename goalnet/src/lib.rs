use anyhow::Context;
use dll_syringe_macros::remote_procedure;
use netcorehost::hostfxr::HostfxrHandle;
use netcorehost::nethost;
use netcorehost::pdcstring::PdCString;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::str::FromStr;

static mut UNLOADED: Cell<bool> = Cell::new(false);
static mut HOSTFXR_HANDLE: Cell<Option<HostfxrHandle>> = Cell::new(None);

#[remote_procedure]
fn load(config_path_vec: Vec<u8>, stderr_log: bool, msgbox_log: bool) {
    if let Err(e) = load_internal(config_path_vec) {
        if stderr_log {
            eprintln!("error loading goalnet: {}", e);
        }

        if msgbox_log {
            msgbox::create(
                "goalnet error",
                &format!("An error occured loading goalnet: {}", e),
                msgbox::IconType::Error,
            )
            .ok();
        }
    }
}

fn load_internal(config_path_vec: Vec<u8>) -> anyhow::Result<()> {
    let config_path = String::from_utf8(config_path_vec).context("failed to parse config path")?;
    let config_path = PathBuf::from_str(&config_path).context("failed to convert config path")?;
    let config = goalnet_common::parse(&config_path).context("failed to parse config file")?;

    let payload_directory = goalnet_common::relative_dir(&config_path, &config.payload.directory)
        .expect("failed to get payload directory path");

    let payload_directory = if config.payload.copy_build_artifacts {
        let temp_dir = std::env::temp_dir().join(goalnet_common::temp_filename());

        copy_dir_all(&payload_directory, &temp_dir).context("failed to copy directory")?;

        temp_dir
    } else {
        payload_directory
    };

    let runtime_config = payload_directory.join(&config.payload.runtime_config);
    let runtime_config_pdcstr = PdCString::from_os_str(runtime_config.as_os_str())
        .context("failed to convert runtime config path")?;

    let dll = payload_directory.join(&config.payload.dll);
    let dll_pdcstr =
        PdCString::from_os_str(dll.as_os_str()).context("failed to convert dll path")?;

    let hostfxr = nethost::load_hostfxr().context("failed to load hostfxr")?;
    let context = hostfxr
        .initialize_for_runtime_config(runtime_config_pdcstr)
        .context("failed to initialize hostfxr")?;
    let loader = context
        .get_delegate_loader_for_assembly(dll_pdcstr)
        .context("failed to get delegate loader")?;

    let type_name =
        PdCString::from_str(&config.entrypoint.type_name).context("failed to convert type name")?;
    let method_name = PdCString::from_str(&config.entrypoint.method_name)
        .context("failed to convert method name")?;
    let delegate_type_name = PdCString::from_str(&config.entrypoint.delegate_type_name)
        .context("failed to convert delegate type name")?;

    if config.entrypoint.unload {
        let init = loader
            .get_function::<fn(*const u8) -> ()>(&type_name, &method_name, &delegate_type_name)
            .context("failed to get unload function")?;

        init(set_unload as *const u8);
    } else {
        let init = loader
            .get_function::<fn() -> ()>(&type_name, &method_name, &delegate_type_name)
            .context("failed to get init function")?;

        init();
    }

    unsafe {
        HOSTFXR_HANDLE.set(Some(context.into_handle()));
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("failed to get file name"))?;
        let dst = dst.join(file_name);
        if path.is_dir() {
            copy_dir_all(&path, &dst)?;
        } else {
            std::fs::copy(&path, &dst)?;
        }
    }

    Ok(())
}

#[remote_procedure]
fn remove_hostfxr() {
    unsafe {
        let context = nethost::load_hostfxr().unwrap();
        context
            .lib
            .hostfxr_close(HOSTFXR_HANDLE.get().unwrap().as_raw());

        HOSTFXR_HANDLE.set(None);
        UNLOADED.set(false);
    }
}

fn set_unload() {
    unsafe {
        UNLOADED.set(true);
    }
}

#[remote_procedure]
fn is_unloaded() -> bool {
    unsafe { UNLOADED.get() }
}
