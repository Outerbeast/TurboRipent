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
    fs,
    io,
    path::PathBuf
};

use crossterm::
{
    event::
    {
        Event,
        KeyCode,
        KeyEvent,
        KeyEventKind,
        read
    },
    style::Stylize,
};

use dialoguer::
{
    Input,
    Select, 
    theme::ColorfulTheme
};

use strum::
{
    EnumMessage,
    VariantArray
};

use strum_macros::
{
    Display,
    EnumMessage,
    VariantArray
};

use crate::
{
    APPNAME, 
    bsp::
    {
        BspFile,
        ent::
        {
            EXT_BRUSH_ENT,
            EXT_BSP,
            EXT_ENT,
            EXT_POINT_ENT
        },
        stats::EntityReport
    },
    exec, utils
};

#[cfg( windows )] use crate::editor;

#[derive( Debug, Display, EnumMessage, VariantArray )]
pub enum Menu
{
    #[strum( message = "Exit TurboRipent" )] Close,
    #[strum( message = "Usage Information" )] Help,
    #[strum( message = "Extract an entity list (.ent file) from a BSP file" )] Extract,
    #[strum( message = "Import an entity list (.ent file) into BSP file" )] Import,
    #[strum( message = "Extract split .entp (point entities) + .entm (brush entities) from a BSP" )] SplitExtract,
    #[strum( message = "Import split .entp (point entities) + .entm (brush entities) into BSP (both files required)" )] SplitImport,
    #[cfg( windows )]
    #[strum( message = "Opens the interactive entity editor for a BSP or ENT file" )] Edit,
    #[strum( message = "Show BSP info such as number of point entities, brush entities, etc.")] Stats
}
// Blocks until a key is pressed
pub fn get_keystroke() -> Result<char, io::Error>
{
    loop
    {
        if let Event::Key( KeyEvent { code, kind: KeyEventKind::Press, .. } ) = read()?
        {
            if let KeyCode::Char( c ) = code
            {
                return Ok( c );
            }
            else if code == KeyCode::Enter
            {
                return Ok( '\r' );
            }
            else if code == KeyCode::Esc
            {
                return Ok( '0' );
            }
        }
    }
}

pub fn get_prompt_input(prompt: &str) -> String
{   // IO error but specifically when Input fails because terminal drops the ball for whatever reason
    // Absorb error and return empty string if it happens
    let buf: String = Input::new().with_prompt( prompt ).interact_text().unwrap_or_default();
    let input = buf.trim();
    // Strip surrounding matching quotes (double or single)
    let stripped = 
    if ( input.starts_with( '"' ) && input.ends_with( '"' ) )
    || ( input.starts_with( '\'' ) && input.ends_with( '\'' ) )
    {
        &input[1..input.len()-1]
    }
    else
    {
        input
    };

    cfg_select!
    {
        windows => stripped.to_string(),
        target_os = "linux" =>
        {
            let mut result = String::with_capacity( stripped.len() );
            let mut chars = stripped.chars();

            while let Some( c ) = chars.next()
            {
                if c == '\\'
                {
                    result.push( chars.next().unwrap_or( c ) );
                }
                else
                {
                    result.push( c );
                }
            }

            result
        }
    }
}

fn help()
{
    crate::clear_terminal!();
    println!( "\n{}\nThis tool allows you to extract and import BSP entity data.\nOptions:", "❓-Help-❓".cyan()  );

    for option in Menu::VARIANTS
    {   // Don't need to show help for close and help options
        if matches!( option, Menu::Close | Menu::Help )
        {
            continue;
        }

        if let Some( help ) = option.get_message()
        {
            println!( "\t{option:?}: {}", help.replace( "\"", "" ) );
        }
    }

    println!( "\nThank you for using {APPNAME}!" );
}
// Menu handler
pub fn show() -> io::Result<bool>
{   // display actual menu
    let choice = 
    &Menu::VARIANTS
    [
        Select::with_theme( &ColorfulTheme::default() )
            .with_prompt( "\nSelect an option\n↑↓ + Space/Enter" )
            .items( Menu::VARIANTS )
            .default( Menu::Extract as usize )
        .interact()?// Unsure how this would fail.
    ];

    match choice
    {
        Menu::Close =>
        {
            println!( "\nThank you for using {APPNAME}!" );
            return Ok( false );
        }

        Menu::Help =>
        {
            help();
            return Ok( true );
        }

        _ => { }
    };
    // Get BSP input
    let chosen_bsp =
        get_prompt_input( 
            &format!( "Drag a BSP file or folder you want to {choice:?} entities (leave blank to use the current folder, enter 'x' to cancel)\n" ) );
    // User quits
    if chosen_bsp == "x" || chosen_bsp == "\r"
    {
        return Ok( true );
    }

    //let chosen_bsp = PathBuf::from( &chosen_bsp );
    let chosen_bsp = // Passed in a ENT file, swap extension, want BSPs only
    if let path = PathBuf::from( &chosen_bsp ) 
    && path.extension().is_some_and( |ext| ext == EXT_ENT || ext == EXT_POINT_ENT || ext == EXT_BRUSH_ENT )
    {
        path.with_extension( EXT_BSP )
    }
    else
    {
        PathBuf::from( &chosen_bsp )
    };
    // Open the BSP in the editor
    #[cfg(windows)]
    if matches!( choice, Menu::Edit ) 
    {
        if let Err( e ) = editor::launch( chosen_bsp )
        {
            return Err( io::Error::other( e ) );
        }

        return Ok( true );
    }

    let mut cleanup_ents = vec![];
    match choice
    {
        Menu::Extract | Menu::Import | Menu::SplitExtract | Menu::SplitImport =>
        {
            match exec::batch_ripent( &chosen_bsp, choice )
            {
                Ok( processed) =>
                {
                    if !processed.0.is_empty()
                    {
                        println!( "✅ {}", format!( "{} BSP(s) processed.", processed.0.len() ).green() );
                    }

                    if !processed.1.is_empty()
                    {
                        eprintln!( "⚠️ {}", format!( "{} BSP(s) failed to process.\
                            \nIf importing, check that the .ent (or .entp/.entm) file exists for the BSP.", 
                            processed.1.len() )
                            .yellow() );
                    }

                    cleanup_ents = processed.0;
                }

                Err( e ) => eprintln!( "❌ {}", format!( "{choice:?} failed: {e}" ).red() )
            }
        }

        Menu::Stats =>
        {
            let stats =
            match &BspFile::load( &chosen_bsp )
            {
                Ok( bsp ) => EntityReport::generate( bsp ).to_string(),
                Err( e ) => format!( "⚠️ {}", format!( "Failed to load bsp file {chosen_bsp:?}: {e}" ).yellow() ),
            };

            println!( "{stats}\n\n❓ Would you like to save these stats to a .log file? [y/n]: " );

            if matches!( get_keystroke()?, 'y' | 'Y' | '\r' )// also Enter key
            {
                fs::write( chosen_bsp.with_extension( ".log" ), stats )?;
            }
        }

        _ => unreachable!()
    };
    // Ask to delete remaining entity files if importing.
    if matches!( choice, Menu::Import | Menu::SplitImport ) && !cleanup_ents.is_empty()
    {
        println!( "❓ Delete remaining entity files? [y/n]: " );

        if matches!( get_keystroke()?, 'y' | 'Y' | '\r' )// also Enter key
        {
            match choice
            {
                Menu::Import => utils::remove_files( &cleanup_ents, Some( EXT_ENT ) ),
                Menu::SplitImport =>
                {
                    utils::remove_files( &cleanup_ents, Some( EXT_BRUSH_ENT ) );
                    utils::remove_files( &cleanup_ents, Some( EXT_POINT_ENT ) );
                }
                _ => unreachable!()
            }
        }
    }

    Ok( true )
}
