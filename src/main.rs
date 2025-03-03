use anyhow::Result;
use std::fmt::Display;
use std::mem;
use std::os::raw::c_void;
use std::{env::consts::OS, process::exit};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsA, SetupDiGetDeviceRegistryPropertyA,
    DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SPDRP_DEVICEDESC, SPDRP_FRIENDLYNAME,
    SP_DEVINFO_DATA,
};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows::Win32::Security::GetTokenInformation;
use windows::Win32::Security::TokenElevation;
use windows::Win32::Security::TOKEN_ELEVATION;
use windows::Win32::Security::TOKEN_QUERY;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

struct WinDev {
    fname: Option<String>,
    desc: Option<String>,
    guid: u128,
}

impl Display for WinDev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fname = self.fname.clone().unwrap_or("Unkown".to_string());
        let desc = self.desc.clone().unwrap_or("None".to_string());
        write!(
            f,
            "---------------------------\nDev Name: {}\nDev Desc: {}\nGUID: {}\nGUID (hex): {:#x}\n---------------------------",
            fname, desc, self.guid, self.guid
        )
    }
}

fn get_fname(
    dev_info_set: HDEVINFO,
    dev_info_data: *const SP_DEVINFO_DATA,
) -> Result<Option<String>> {
    let mut buffer: Vec<u8> = vec![0; 256];
    let mut required_size: u32 = 0;

    unsafe {
        // when no name return None
        if SetupDiGetDeviceRegistryPropertyA(
            dev_info_set,
            dev_info_data,
            SPDRP_FRIENDLYNAME,
            None,
            Some(&mut buffer),
            Some(&mut required_size),
        )
        .is_err()
        {
            return Ok(None);
        }
    }

    if let Some(null_pos) = buffer.iter().position(|&b| b == 0) {
        buffer.truncate(null_pos); // Remove trailing nulls
    }
    let friendly_name = String::from_utf8_lossy(&buffer).to_string();
    Ok(Some(friendly_name))
}

fn get_desc(
    dev_info_set: HDEVINFO,
    dev_info_data: *const SP_DEVINFO_DATA,
) -> Result<Option<String>> {
    let mut buffer: Vec<u8> = vec![0; 256];
    let mut required_size: u32 = 0;

    unsafe {
        // when no desc return None
        if SetupDiGetDeviceRegistryPropertyA(
            dev_info_set,
            dev_info_data,
            SPDRP_DEVICEDESC,
            None,
            Some(&mut buffer),
            Some(&mut required_size),
        )
        .is_err()
        {
            return Ok(None);
        }
    }

    if let Some(null_pos) = buffer.iter().position(|&b| b == 0) {
        buffer.truncate(null_pos); // Remove trailing nulls
    }
    let desc = String::from_utf8_lossy(&buffer).to_string();
    Ok(Some(desc))
}

// This code snippet is derived from "is-root" by "John Meow"
// Original repository: https://gitlab.com/caralice/is-root
fn is_root() -> Result<bool> {
    let mut token = INVALID_HANDLE_VALUE;
    let mut elevated = false;
    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_ok() {
            let mut elevation: TOKEN_ELEVATION = mem::zeroed();
            let mut size = mem::size_of::<TOKEN_ELEVATION>().try_into().unwrap();
            if GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut TOKEN_ELEVATION as *mut c_void),
                size,
                &mut size,
            )
            .is_ok()
            {
                elevated = elevation.TokenIsElevated != 0;
            }
        }
        if token != INVALID_HANDLE_VALUE {
            CloseHandle(token)?;
        }
    }
    Ok(elevated)
}

fn main() -> Result<()> {
    if OS != "windows" {
        println!("OS isn't windows!");
        exit(1);
    }

    if !is_root()? {
        println!("This program needs root priviledges");
        exit(1);
    }

    let dev_info_set =
        unsafe { SetupDiGetClassDevsA(None, None, None, DIGCF_ALLCLASSES | DIGCF_PRESENT) }?;

    if dev_info_set.is_invalid() {
        println!("Failed to get device list");
        exit(1)
    }

    let mut dev_info_data = SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };

    let mut index = 0;
    loop {
        if let Err(e) = unsafe { SetupDiEnumDeviceInfo(dev_info_set, index, &mut dev_info_data) } {
            // Exit code for no more devices
            match e.to_string().contains("0x80070103") {
                true => break Ok(()),
                false => {
                    println!("Error occurred: {}", e);
                    exit(1);
                }
            }
        };
        index += 1;

        let dev = WinDev {
            fname: get_fname(dev_info_set, &dev_info_data)?,
            desc: get_desc(dev_info_set, &dev_info_data)?,
            guid: dev_info_data.ClassGuid.to_u128(),
        };

        println!("{}", dev);
    }
}
