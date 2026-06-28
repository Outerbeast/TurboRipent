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
    fmt::
    {
        self,
        Display
    },
    fs,
    io,
    path::PathBuf
};

use anyhow::Result;
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
    terminal::
    {
        disable_raw_mode,
        enable_raw_mode
    }
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
    EnumMessage,
    EnumString,
    VariantArray
};

use crate::
{
    APPNAME, 
    bsp::ent::
    {
        EXT_BRUSH_ENT,
        EXT_BSP,
        EXT_ENT,
        EXT_POINT_ENT
    },
    exec,
    utils
};

#[cfg( windows )] use crate::editor;
/// Handles TUI menu and CLI args
#[derive( Debug, EnumMessage, EnumString, PartialEq, VariantArray )]
pub enum Menu
{
    #[strum( message = "Exit TurboRipent" )] Close,

    #[strum(
        serialize = "-help",
        serialize = "-usage",
        serialize = "-h",
        message = "Usage Information"
    )]
    Help,

    #[strum(
        serialize = "-extract",
        serialize = "-export",
        serialize = "-e",
        message = "Extract an entity list (.ent file) from a BSP file"
    )]
    Extract,

    #[strum(
        serialize = "-import",
        serialize = "-i",
        message = "Import an entity list (.ent file) into BSP file"
    )]
    Import,

    #[strum(
        serialize = "-splitextract",
        serialize = "-splitexport",
        serialize = "-se",
        message = "Extract split .entp (point entities) + .entm (brush entities) from a BSP"
    )]
    SplitExtract,

    #[strum(
        serialize = "-splitimport",
        serialize = "-si",
        message = "Import split .entp (point entities) + .entm (brush entities) into BSP (both files required)"
    )]
    SplitImport,

    #[strum(
        serialize = "-repair",
        serialize = "-parse",
        serialize = "-r",
        message = "Repair entities in a BSP file (re-parse and re-serialise)"
    )]
    Repair,

    #[strum(
        serialize = "-stats",
        serialize = "-info",
        message = "Show BSP info such as number of point entities, brush entities, etc."
    )]
    Stats,

    #[cfg(windows)]
    #[strum(
        serialize = "-edit",
        serialize = "-editor",
        serialize = "-gui",
        message = "Opens the interactive entity editor for a BSP or ENT file"
    )]
    Edit
}

impl Menu
{
    pub fn help()
    {
        crate::clear_terminal!();
        println!( "\n{}\nThis tool allows you to extract and import BSP entity data.\nOptions:-", "❓-Help-❓".cyan() );

        for option in Self::VARIANTS
        {   // Don't need to show help for close and help options
            if matches!( option, Self::Close | Self::Help )
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
    pub fn show() -> Result<bool>
    {   // display actual menu
        let choice = 
        &Self::VARIANTS
        [
            Select::with_theme( &ColorfulTheme::default() )
                .with_prompt( "\nSelect an option\n↑↓ + Space/Enter" )
                .items( Self::VARIANTS )
                .default( Self::Extract as usize )
            .interact()?// Unsure how this would fail.
        ];

        match choice
        {
            Self::Close =>
            {
                println!( "\nThank you for using {APPNAME}!" );
                return Ok( false );
            }

            Self::Help =>
            {
                Self::help();
                return Ok( true );
            }

            _ => { }
        };
        // Get BSP input
        let chosen_bsp =
            get_prompt_input( 
                &format!( "Drag a BSP file or folder you want to {choice:?} (leave blank to use the current folder, enter 'x' to cancel)\n" ) );
        // User quits
        if chosen_bsp == "x"
        {
            return Ok( true );
        }

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
        if matches!( choice, Self::Edit ) 
        {
            editor::launch( chosen_bsp )?;
            return Ok( true );
        }

        let mut processed_bsps = vec![];
        match choice
        {
            Self::Extract | Self::Import | Self::SplitExtract | Self::SplitImport | Self::Repair =>
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

                        processed_bsps = processed.0;
                    }

                    Err( e ) => eprintln!( "❌ {}", format!( "{choice:?} failed: {e}" ).red() )
                }
            }

            Self::Stats =>
            {
                match exec::batch_stats( &chosen_bsp )
                {
                    Ok( reports ) =>
                    {
                        println!( "Save stats to file (.log)? [y/n]" );
                        let should_save = matches!( get_keystroke()?, 'y' | 'Y' | '\r' );
                        println!( "{}", "\n================= TurboRipent - Entity Report =================".cyan() );

                        for ( path, report ) in &reports
                        {
                            println!( "📊 {:?}:\n{report}\n{}\n", path.file_stem().unwrap_or_default(), "_".repeat( 100 ) );

                            if should_save && let Err( e ) = fs::write( path.with_extension( "log" ), report )
                            {
                                eprintln!( "❌ {}", format!( "Failed to save stats for {path:?}: {e}" ).red() )
                            }
                        }
                    }

                    Err( e ) => eprintln!( "❌ {}", format!( "Stats failed: {e}" ).red() )
                }
            }

            _ => unreachable!()
        };
        // Ask to delete remaining entity files if importing.
        if matches!( choice, Self::Import | Self::SplitImport ) && !processed_bsps.is_empty()
        {
            println!( "❓ Delete remaining entity files? [y/n]: " );

            if matches!( get_keystroke()?, 'y' | 'Y' | '\r' )// also Enter key
            {
                match choice
                {
                    Self::Import => utils::remove_files( &processed_bsps, Some( EXT_ENT ) ),
                    Self::SplitImport =>
                    {
                        utils::remove_files( &processed_bsps, Some( EXT_BRUSH_ENT ) );
                        utils::remove_files( &processed_bsps, Some( EXT_POINT_ENT ) );
                    }

                    _ => unreachable!()
                }
            }
        }

        Ok( true )
    }

}

impl Display for Menu
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!( f, "{self:?}" )
    }
}
// Blocks until a key is pressed
pub fn get_keystroke() -> Result<char>
{
    enable_raw_mode()?;
    loop
    {
        if let Event::Key( KeyEvent { code, kind: KeyEventKind::Press, .. } ) = read()?
        {
            let result =
            match code
            {
                KeyCode::Char( c ) if c == '\r' || c == '\n' => '\r',
                KeyCode::Char( c ) => c,
                KeyCode::Enter => '\r',
                KeyCode::Esc => '0',
                _ => continue,
            };

            disable_raw_mode()?;
            return Ok( result );
        }
    }
}

pub fn get_prompt_input(prompt: &str) -> String
{   // IO error but specifically when Input fails because terminal drops the ball for whatever reason
    // Absorb error and return empty string if it happens
    let input: String = Input::with_theme( &ColorfulTheme::default() )
        .with_prompt( prompt )
        .allow_empty( true )
        .interact()
    .unwrap_or_default();

    let input = input.trim();
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
