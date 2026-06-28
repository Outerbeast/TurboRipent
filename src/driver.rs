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
    io,
    path::PathBuf
};

use anyhow::Result;

use crossterm::
{
    style::Stylize,
    terminal
};

use crate::
{
    APPNAME,
    bsp::ent,
    clear_terminal,
    exec,
    cli::
    {
        self,
        Menu
    }
};

#[cfg( windows )] use crate::editor;

pub fn run() -> Result<()>
{
    crossterm::execute!( io::stdout(), terminal::SetTitle( APPNAME ) )?;
    println!( "{}\nExtract and Import BSP entity data", APPNAME.on_green().bold().underline_white() );
    let (args, paths) = crate::utils::get_args::<Menu, PathBuf>();

    if args.contains( &Menu::Help )
    {
        Menu::help();
        return Ok( () );
    }

    if args.contains( &Menu::Repair )
    {
        for p in &paths
        {
            if let Err( e ) = p.try_exists()
            {
                eprintln!( "❌ {}", format!( "Error processing {p:?}: {e}" ).red() );
                continue;
            }

            if let Err( e ) = ent::repair( p )
            {
                eprintln!( "❌ {}", format!( "Error processing {p:?}: {e}" ).red() );
                continue;
            }
        }

        return Ok( () );
    }

    if args.contains( &Menu::Stats )
    {
        for p in &paths
        {
            if let Err( e ) = p.try_exists()
            {
                eprintln!( "❌ {}", format!( "Error processing {p:?}: {e}" ).red() );
                continue;
            }

            match exec::batch_stats( p )
            {
                Ok( reports ) =>
                {
                    for ( report_path, report_txt ) in &reports
                    {
                        println!( "{report_path:?}:\n{report_txt}\n" );
                    }
                }

                Err( e ) => eprintln!( "❌ {}", format!( "Stats failed for {p:?}: {e}" ).red() ),
            }
        }

        return Ok( () );
    }

    #[cfg( windows )]
    if args.contains( &Menu::Edit )
    {
        let path = 
        match paths.first()
        {
            Some( p ) => p,
            None => anyhow::bail!( "Please provide a BSP to edit e.g. '-edit bspfile.bsp'" )
        };

        editor::launch( path )?;

        return Ok( () );
    }
    // Just act on BSPs/ENTs if no arg flags used
    if !paths.is_empty()
    {
        for p in &paths
        {
            if let Err( e ) = p.try_exists()
            {
                eprintln!( "❌ {}", format!( "Error processing {p:?}: {e}" ).red() );
                continue;
            }

            if let Err( e ) = ent::rip( p )
            {
                eprintln!( "❌ {}", format!( "Error processing {p:?}: {e}" ).red() );
                continue;
            }
        }

        return Ok( () );
    }

    while Menu::show()?
    {
        println!( "\nPress any key to return..." );
        let _ = cli::get_keystroke();
        clear_terminal!();
    }

    Ok( () )
}
