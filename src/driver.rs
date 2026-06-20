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
use std::
{
    env,
    io,
    path::PathBuf
};

use crossterm::
{
    style::Stylize,
    terminal
};

use anyhow::Result;

use crate::
{
    APPNAME,
    bsp::ent::
    {
        self,
        EXT_BRUSH_ENT,
        EXT_BSP,
        EXT_ENT,
        EXT_POINT_ENT
    }, 
    clear_terminal,
    menu
};

#[cfg( windows )] use crate::editor;

pub fn run() -> Result<()>
{   // Set console title
    crossterm::execute!( io::stdout(), terminal::SetTitle( APPNAME ) )?;
    // Print banner
    println!( "{}\nExtract and Import BSP entity data", APPNAME.on_green().bold().underline_white() );
    // Check CLI args
    if let args = env::args().skip( 1 ).collect::<Vec<_>>() && !args.is_empty()
    {
        if matches!( args[0].as_str(), "-edit" | "-editor" | "-gui" )
        {
            #[cfg( windows )]
            {
                if args.len() > 1
                {
                    editor::launch( &args[1] )?;
                }
                else
                {
                    eprintln!( "{}", "Please provide a BSP to edit e.g. '-edit bspfile.bsp'".yellow() );
                    return Err( io::Error::new( io::ErrorKind::InvalidInput, "Missing BSP file path" ).into() );
                }
            }

            #[cfg(not(windows))]
            {
                eprintln!( "{}", "The editor is only available for Windows.".yellow() );
                return Err( io::Error::new( io::ErrorKind::Unsupported, "Editor not available on this platform" ).into() );
            }
        }
        else// Handle quick extract/import/
        {
            for a in &args
            {
                let path = PathBuf::from( a );
                
                if path.try_exists().is_err()
                {
                    eprintln!( "{}", "Error: {path:?} does not exist.".red() );
                    continue;
                }

                match path.extension().and_then( |ext| ext.to_str() )
                {
                    Some( EXT_BSP ) | 
                    Some( EXT_ENT ) | 
                    Some( EXT_POINT_ENT ) | 
                    Some( EXT_BRUSH_ENT ) => ent::rip( &path )?,
                    _ => continue
                }
            }
        }
    }
    else// No args, display menu
    {
        while menu::show()?
        {
            println!( "\nPress any key to return..." );
            let _ = menu::get_keystroke();
            clear_terminal!();
        }
    }
    
    Ok( () )
}
