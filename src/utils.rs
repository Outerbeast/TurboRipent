/*
	TurboRipent - TUI Frontend for Ripent
	Version 2.0

Copyright (C) 2025 Outerbeast
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/
#[macro_export]
macro_rules! alloc_leaked// Leak allocator: boxes the value, leaks it forever
{
    ( $value:expr ) =>
    {
        Box::leak( Box::new( $value ) )
    };
}

#[macro_export]
macro_rules! current_dir_path
{
    () =>
    {
        env::current_dir().unwrap_or( PathBuf::from( "." ) )
    };
}

#[macro_export]
macro_rules! clear_terminal
{
    () =>
    {
        {
            use std::io::Write;
            print!( "\x1bc" );
            let _ = io::stdout().flush();
        }
    };
}
// Hide console window (Windows only)
#[cfg( target_os = "windows" )]
pub fn hide_terminal()
{
    use std::ffi;

    unsafe extern "system"
    {
        fn GetConsoleWindow() -> *mut ffi::c_void;
        fn ShowWindow(hwnd: *mut ffi::c_void, nCmdShow: i32) -> i32;
    }

    let hwnd = unsafe { GetConsoleWindow() };
    if !hwnd.is_null()
    {
        unsafe { ShowWindow(hwnd, 0); } // SW_HIDE = 0
    }
}

#[cfg( target_os = "windows" )]
pub fn show_terminal()
{
    use std::ffi;

    unsafe extern "system"
    {
        fn GetConsoleWindow() -> *mut ffi::c_void;
        fn ShowWindow(hwnd: *mut ffi::c_void, nCmdShow: i32) -> i32;
        fn SetForegroundWindow(hwnd: *mut ffi::c_void) -> i32;
    }

    let hwnd = unsafe { GetConsoleWindow() };
    if !hwnd.is_null()
    {
        unsafe { ShowWindow(hwnd, 5); } // SW_SHOW = 5
        unsafe { SetForegroundWindow(hwnd); }
    }
}

pub fn remove_files(paths: &[std::path::PathBuf], some_extension: Option<&str>)
{
    if paths.is_empty()
    {
        return;
    }

    for p in paths
    {
        if let Some( ext ) = some_extension
        {
            if let Err( e ) = std::fs::remove_file( p.with_extension( ext ) )
            {
                eprintln!( "⚠️ Couldn't delete '{p:?}' with extension '{ext}': {e}" );
            }
        }
        else
        {
            if let Err( e ) = std::fs::remove_file( p )
            {
                eprintln!( "⚠️  Couldn't delete {p:?}: {e}" );
            }
        }
    }
}
