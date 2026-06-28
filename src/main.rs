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
mod bsp;
mod driver;
mod exec;
mod cli;
mod utils;
#[cfg( windows )] mod editor;
#[cfg( test )] mod tests;

pub const APPNAME: &str = env!( "CARGO_PKG_NAME" );

fn main() -> std::process::ExitCode
{
    match driver::run()
    {
        Ok( () ) => std::process::ExitCode::SUCCESS,
        Err( e ) =>
        {
            eprintln!( "Application error: {e}.\nPress any key to exit..." );
            std::io::Write::flush( &mut std::io::stdout() ).ok();
            let _ = std::io::Read::read_exact( &mut std::io::stdin(), &mut[0] );

            std::process::ExitCode::FAILURE
        }
    }
}
