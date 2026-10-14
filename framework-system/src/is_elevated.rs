//! 判断当前进程是否已提权。

use anyhow::Result;

/// 判断当前进程是否以管理员（Windows）或 root（Linux、macOS）身份运行。
///
/// - Windows：查询当前进程令牌的 `TokenElevation`，为真表示已提权。
/// - Linux、macOS：判断有效用户 ID 是否为 `0`。
/// - 其他平台：始终返回假。
#[cfg(unix)]
pub fn is_elevated() -> Result<bool> {
    Ok(unsafe { libc::geteuid() } == 0)
}

/// 判断当前进程是否以管理员身份运行。
#[cfg(windows)]
pub fn is_elevated() -> Result<bool> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut return_length = 0u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );
        let last_error = if result == 0 {
            Some(std::io::Error::last_os_error())
        } else {
            None
        };
        // 令牌必须显式关闭，先取出错误再关闭，避免提前返回时泄漏句柄。
        CloseHandle(token);

        if let Some(error) = last_error {
            return Err(error.into());
        }

        Ok(elevation.TokenIsElevated != 0)
    }
}

/// 判断当前进程是否已提权；当前平台无法判断，始终返回假。
#[cfg(not(any(unix, windows)))]
pub fn is_elevated() -> Result<bool> {
    Ok(false)
}
