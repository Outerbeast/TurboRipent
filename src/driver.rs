/*
	TurboRipent - TUI Frontend for Ripent
	Version 2.1.0

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

use anyhow::
{
    bail,
    Result
};

use crossterm::
{
    style::Stylize,
    terminal
};

use crate::
{
    prelude::*,
    bsp::ent,
    cli,
    editor,
    exec,
    utils
};

pub(crate) fn run() -> Result<()>
{
    crossterm::execute!( io::stdout(), terminal::SetTitle( APPNAME ) )?;
    println!( "{}\nExtract and Import BSP entity data", APPNAME.on_green().bold().underline_white() );
    let (args, paths) = utils::get_args::<Menu, PathBuf>();

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

    if args.contains( &Menu::Edit )
    {
        let Some( path ) = paths.first()
        else
        {
            bail!( "Please provide a BSP to edit e.g. '-edit bspfile.bsp'" );
        };
        
        return editor::launch( path );
    }

    const BATCH_ACTIONS: [Menu; 4] = [ Menu::Extract, Menu::Import, Menu::SplitExtract, Menu::SplitImport ];

    if BATCH_ACTIONS.iter().any( |a| args.contains( a ) )
    {
        for action in &BATCH_ACTIONS
        {
            if !args.contains( action )
            {
                continue;
            }

            for p in &paths
            {
                if let Err( e ) = p.try_exists()
                {
                    eprintln!( "❌ {}", format!( "Error processing {p:?}: {e}" ).red() );
                    continue;
                }

                match exec::batch_ripent( p, action )
                {
                    Ok( ( processed, failed ) ) =>
                    {
                        if !processed.is_empty()
                        {
                            println!( "✅ {}", format!( "{} BSP(s) processed.", processed.len() ).green() );
                        }

                        if !failed.is_empty()
                        {
                            eprintln!( "⚠️ {}", format!( "{} BSP(s) failed.", failed.len() ).yellow() );
                        }
                    }

                    Err( e ) => eprintln!( "❌ {}", format!( "{action:?} failed for {p:?}: {e}" ).red() ),
                }
            }
        }

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
        crossterm::execute!( io::stdout(), terminal::SetTitle( APPNAME ) )?;
        println!( "\nPress any key to return..." );
        let _ = cli::get_keystroke();
        clear_terminal!();
    }

    Ok( () )
}
