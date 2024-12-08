use anyhow::Result;
use is_root::is_root;
use std::fmt::Display;
use std::{env::consts::OS, process::exit};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsA, SetupDiGetDeviceRegistryPropertyA,
    DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SPDRP_DEVICEDESC, SPDRP_FRIENDLYNAME,
    SP_DEVINFO_DATA,
};

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

fn main() -> Result<()> {
    if OS != "windows" {
        println!("OS isn't windows!");
        exit(1);
    }

    if !is_root() {
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
