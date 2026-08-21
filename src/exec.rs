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
    fs,
    path::
    {
        Path,
        PathBuf
    },
};

use anyhow::
{
    Result,
    bail
};

use crossterm::style::Stylize;

use crate::
{
    bsp::ent,
    cli::Menu,
    current_dir_path,
    prelude::*
};
// Collects BSP files from a directory and its subdirectories.
pub fn collect_bsps(input: impl AsRef<Path>) -> Vec<PathBuf>
{
    let path = input.as_ref();

    if !path.as_os_str().is_empty() && !path.is_dir() && path.has_extension( &[EXT_BSP] )
    {   
        return vec![path.to_path_buf()];
    }
    // Input might end with a trailing wildcard
    let is_wildcard = path.to_string_lossy().ends_with( '*' );

    if path.as_os_str().is_empty() || path.is_dir() || is_wildcard
    {
        let dir =
        {
            if path.as_os_str().is_empty()
            {
                current_dir_path!()
            } 
            else if is_wildcard
            {
                path.parent().unwrap_or( Path::new( "." ) ).to_path_buf()
            } 
            else
            {
                path.to_path_buf()
            }
        };
        // Trim the wildcard from the input to get the prefix for
        let prefix = is_wildcard.then( ||
        {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .trim_end_matches( '*' )
            .to_string()
        });

        let mut bsps = vec![];
        if let Ok( entries ) = fs::read_dir( &dir )
        {
            for entry in entries.flatten()
            {
                if !entry.file_type().is_ok_and( |ft| ft.is_file() )
                || !&entry.path().has_extension( &[EXT_BSP] )
                {
                    continue;
                }

                if let Some( ref p ) = prefix && !entry.file_name().to_string_lossy().starts_with( p )
                {
                    continue;
                }

                bsps.push( entry.path() );
            }
        }

        return bsps;
    }

    vec![]
}
// Extraction/importation of BSP files, Result: collection of success/failed BSPs
pub fn batch_ripent(bsp_fileordir_path: &Path, action: &Menu) -> Result<(Vec<PathBuf>, Vec<PathBuf>)>
{
    if !matches!( action, Menu::Extract | Menu::Import | Menu::SplitExtract | Menu::SplitImport | Menu::Repair )
    {
        bail!( "Invalid action '{:?}' for batch ripent", action );
    }

    let bsps = collect_bsps( bsp_fileordir_path );

    if bsps.is_empty()
    {
        bail!( "No BSP files were found.\nIf you entered an '.{EXT_ENT}' file, you have to use the '.{EXT_BSP}' file instead." );
    }

    let( mut success, mut failed ) = ( Vec::with_capacity( bsps.len() ), Vec::with_capacity( bsps.len() ) );

    for b in &bsps
    {
        let bsp_path =
        if bsp_fileordir_path.has_extension( &[EXT_BSP] ) || bsps.len() == 1
        {
            b.clone()
        }
        else
        {
            bsp_fileordir_path.join( b )
        };

        println!( "{}", format!( "{action:?}ing: {bsp_path:?}" ).cyan() );

        if matches!( action, Menu::Repair )
        {
            if let Err( e ) = ent::repair( &bsp_path )
            {
                eprintln!( "⚠️ {}", format!( "{action:?} failed for file {bsp_path:?}: {e}" ).yellow() );
                failed.push( bsp_path );
            }
            else
            {
                success.push( bsp_path );
            }
            
            continue;
        }

        match BspFile::load( &bsp_path )
        {
            Ok( bsp_file ) =>
            {
                let ent_path = bsp_path.with_extension( EXT_ENT );
                let result =
                match action
                {
                    Menu::Extract => ExtractTarget::Single.with( &bsp_file, Some( &ent_path ) ).map( |_| () ),
                    Menu::Import => Ok( ImportSource::Single( ent_path ).with( bsp_file )?.save()? ),
                    Menu::SplitExtract => ExtractTarget::Split.with( &bsp_file, Some( &bsp_path.with_extension( "" ) ) ).map( |_| () ),
                    Menu::SplitImport => Ok( ImportSource::Split( bsp_path.with_extension( "" ) ).with( bsp_file )?.save()? ),
                    _ => unreachable!()
                };

                if let Err( e ) = result
                {
                    eprintln!( "⚠️ {}", format!( "{action:?} failed for file {bsp_path:?}: {e}" ).yellow() );
                    failed.push( bsp_path );

                    continue;
                }
                else
                {
                    success.push( bsp_path )
                }
            }

            Err( e ) =>
            {
                eprintln!( "⚠️ {}", format!( "Failed to load file {bsp_path:?}: {e}" ).yellow() );
                failed.push( bsp_path );

                continue;
            }
        }
    }

    Ok( ( success, failed ) )
}
// Stat generation for BSP files, Result: collection of BSP file reports
pub fn batch_stats(bsp_fileordir_path: &Path) -> Result<Vec<(PathBuf, String)>>
{
    let bsps = collect_bsps( bsp_fileordir_path );

    if bsps.is_empty()
    {
        bail!( "No BSP files were found." );
    }

    let mut results = Vec::with_capacity( bsps.len() );

    for b in &bsps
    {
        let report = 
        match BspFile::load( b )
        {
            Ok( bsp ) => EntityReport::generate( &bsp ).to_string(),
            Err( e ) => format!( "⚠️ {}", format!( "Failed to load {:?}: {e}", b ).yellow() ),
        };
        
        results.push( ( b.clone(), report ) );
    }

    Ok( results )
}
