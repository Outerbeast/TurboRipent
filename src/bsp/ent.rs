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
    path::
    {
        Path,
        PathBuf
    }
};

use anyhow::
{
    Result,
    bail
};

use crate::
{
    bsp::
    {
        BspFile,
        LumpIdx
    }
};

pub const EXT_ENT: &str = "ent";
pub const EXT_POINT_ENT: &str = "entp";
pub const EXT_BRUSH_ENT: &str = "entm";
pub const EXT_BSP: &str = "bsp";

pub type Dictionary = std::collections::HashMap<String, String>;

pub enum ExtractTarget
{
    Single,
    Split
}

pub enum ImportSource
{
    Text(String),
    File(PathBuf),
    Split(PathBuf),
}

#[inline] pub fn is_brush_ent(ent_kvs: &Dictionary) -> bool
{
    ent_kvs.get( "classname" ).is_some_and( |c| c == "worldspawn" )
    || ent_kvs.get( "model" ).is_some_and( |m| m.starts_with( '*' ) && m.len() > 1 )
}

fn extract_keyvalues(ent_block: &str) -> Dictionary
{
    let mut dict = Dictionary::new();
    let quoted: Vec<_> = ent_block.split( '"' ).skip( 1 ).step_by( 2 ).collect();

    for pair in quoted.chunks( 2 )
    {
        if let [key, value] = pair
        {
            dict.insert( key.to_string(), value.to_string() );
        }
    }

    dict
}

pub fn parse_entity_blocks(ent_txt: &str) -> Vec<(String, Dictionary)>
{
    ent_txt.split( '{' )
        .skip( 1 )
        .filter_map( |block|
        {
            let inner = block.split_once( '}' )?.0;
            Some( ( format!( "{{{inner}}}" ), extract_keyvalues( inner ) ) )
        })
    .collect()
}

pub fn serialize_entities(entities: &[Dictionary]) -> String
{
    let mut out = String::new();

    for kv in entities
    {
        if kv.is_empty()
        {
            continue;
        }

        out.push_str( "{\n" );

        for (key, value) in kv
        {
            out.push_str( &format!( "\"{key}\" \"{value}\"\n" ) );
        }

        out.push_str( "}\n\n" );
    }

    out
}
// Extracts entity data from a BSP file to entity file
pub fn extract(bsp: &BspFile, out: &Path, target: ExtractTarget) -> Result<()>
{
    let ent_data = bsp.slice_lump( LumpIdx::Entities );

    if ent_data.is_empty()
    {
        bail!( "Entity extraction failed: data size is 0" );
    }

    let all_ents =
    if let Some( &0 ) = ent_data.last()
    {
        str::from_utf8( &ent_data[..ent_data.len() - 1] )?
    }
    else
    {
        str::from_utf8( ent_data )?
    };

    match target
    {
        ExtractTarget::Single => fs::write( out, all_ents.as_bytes() ).map_err( Into::into ),
        ExtractTarget::Split =>// Split entities into point and brush entities into separate files
        {
            let mut point_ents = String::new();
            let mut brush_ents = String::new();

            for ( raw, dict ) in &parse_entity_blocks( all_ents )
            {
                if is_brush_ent( dict )
                {
                    brush_ents.push_str( raw );
                    brush_ents.push( '\n' );
                }
                else
                {
                    point_ents.push_str( raw );
                    point_ents.push( '\n' );
                }
            }

            fs::write( out.with_extension( EXT_POINT_ENT ), point_ents.as_bytes() )?;
            fs::write( out.with_extension( EXT_BRUSH_ENT ), brush_ents.as_bytes() )?;

            println!( "Extracted split entities → {:?} and {:?}",
                out.with_extension( EXT_POINT_ENT ), out.with_extension( EXT_BRUSH_ENT ) );

            Ok( () )
        }
    }
}
// Imports entity data from an entity file into a BSP.
// The BSP file is NOT ovewritten here - BspFile::save() must be executed after this function returns OK
// This avoids BSP corruption.
pub fn import(mut bsp: BspFile, source: ImportSource) -> Result<BspFile>
{
    let mut ent_txt = 
    match source
    {
        ImportSource::Text( t ) => t,
        ImportSource::File( p ) => fs::read_to_string( p )?,
        ImportSource::Split( base ) =>
        {
            let point_ents_file = base.with_extension( EXT_POINT_ENT );
            let brush_ents_file = base.with_extension( EXT_BRUSH_ENT );

            if !point_ents_file.try_exists().unwrap_or( false )
            {
                bail!( "Cannot import split entities: missing {point_ents_file:?}.\n
                    Both .{EXT_POINT_ENT} and .{EXT_BRUSH_ENT} are required." );
            }

            if !brush_ents_file.try_exists().unwrap_or( false )
            {
                bail!( "Cannot import split entities: missing {brush_ents_file:?}.\n
                    Both .{EXT_POINT_ENT} and .{EXT_BRUSH_ENT} are required." );
            }

            let mut combined = fs::read_to_string( &point_ents_file )?;
            combined.push( '\n' );
            combined.push_str( &fs::read_to_string( &brush_ents_file )? );

            combined
        }
    };

    if !ent_txt.ends_with( '\0' )
    {
        ent_txt.push( '\0' );
    }

    bsp.replace_lump( LumpIdx::Entities, ent_txt.as_bytes() )?;

    Ok( bsp )
}
// Decides automatically extraction/importation based on file type
pub fn rip(bsporent_path: &Path) -> Result<()>
{
    if !bsporent_path.exists() 
    {
        bail!( "BSP rip: '{:?}' does not exist.", bsporent_path );
    }

    match bsporent_path.extension().and_then( |ext| ext.to_str() )
    {
        Some( EXT_BSP ) =>// Extract from BSP to ENT
        {
            let ent_path = bsporent_path.with_extension( EXT_ENT );// "level.bsp" -> "level.ent"
            extract( &BspFile::load( bsporent_path )?, &ent_path, ExtractTarget::Single )?;
            println!( "Extracted entities → {ent_path:?}" );

            Ok( () )
        }

        Some( EXT_ENT ) =>// Import from ENT to BSP
        {
            let bsp_path = bsporent_path.with_extension( EXT_BSP );// "level.ent" -> "level.bsp"
            let bsp = BspFile::load( &bsp_path )?;
            import( bsp, ImportSource::File( bsporent_path.to_path_buf() ) )?.save()?;
            println!( "Imported entities → {bsp_path:?}" );
            fs::remove_file( bsporent_path )?;

            Ok( () )
        }

        Some( EXT_POINT_ENT ) | Some( EXT_BRUSH_ENT ) =>// Import from .entp + .entm to BSP
        {
            let bsp_path = bsporent_path.with_extension( EXT_BSP );
            let bsp = BspFile::load( &bsp_path )?;
            import( bsp, ImportSource::Split( bsp_path.with_extension( "" ) ) )?.save()?;
            println!( "Imported split entities → {bsp_path:?}" );

            fs::remove_file( bsp_path.with_extension( EXT_POINT_ENT ) )?;
            fs::remove_file( bsp_path.with_extension( EXT_BRUSH_ENT ) )?;

            Ok( () )
        }

        _ => Ok( () )
    }
}
