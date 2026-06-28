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
    Split(PathBuf)
}

#[inline] pub(crate) fn is_brush_ent(ent_kvs: &Dictionary) -> bool
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

        out.push_str( "}\n" );
    }

    out
}

pub(crate) fn normalise_entities(text: &str) -> String
{   // Pass 1: Structural normalisation (quote-aware brace handling)
    let mut struct_fixed = String::new();
    let mut in_quote = false;
    let mut depth: i32 = 0;

    for c in text.chars()
    {
        match c
        {
            '\r' => continue,
            '"' =>
            { 
                in_quote = !in_quote;
                struct_fixed.push( c );
            }

            '{' if !in_quote =>
            {
                if !struct_fixed.is_empty() && !struct_fixed.ends_with( '\n' )
                {
                    struct_fixed.push( '\n' );
                }

                struct_fixed.push_str( "{\n" );
                depth = depth.max( 0 ) + 1;
            }

            '}' if !in_quote =>
            {
                if !struct_fixed.ends_with( '\n' )
                {
                    struct_fixed.push( '\n' );
                }

                depth -= 1;
                struct_fixed.push( c );
                struct_fixed.push( '\n' );
            }

            _ if depth > 0 => struct_fixed.push( c ),
            _ => { }
        }
    }

    while depth > 0
    {
        struct_fixed.push_str( "}\n" );
        depth -= 1;
    }
    // Pass 2: Per-line kvp fixing
    let mut fixed = String::new();
    let mut in_block = false;

    for line in struct_fixed.lines()
    {
        let trimmed = line.trim();

        if trimmed == "{"
        {
            in_block = true;
            fixed.push_str( line );
            fixed.push( '\n' );
        }
        else if trimmed == "}"
        { 
            in_block = false;
            fixed.push_str( line );
            fixed.push( '\n' );
        }
        else if in_block && !trimmed.is_empty()
        {
            let q: Vec<_> = trimmed.split( '"' ).collect();
            match q.len() - 1
            {
                2 =>
                {
                    let key = q[1];
                    if !q[2].trim().is_empty()
                    {
                        fixed.push_str( &format!( "\"{key}\" \"{}\"\n", q[2].trim() ) );
                    }
                }

                3 => fixed.push_str( &format!( "\"{}\" \"{}\"\n", q[1], q[3] ) ),
                4 =>
                {
                    fixed.push_str( line );
                    fixed.push( '\n' );
                }

                5 ..= 7 =>
                {
                    let clean: String = q[3..q.len()-1].concat().chars().filter( |c| *c != '"' ).collect();
                    if !clean.is_empty()
                    {
                        fixed.push_str( &format!( "\"{}\" \"{clean}\"\n", q[1] ) );
                    }
                }

                _ =>
                {
                    let mut i = 0;
                    while i + 3 < q.len() - 1
                    {
                        fixed.push_str( &format!( "\"{}\" \"{}\"\n", q[i + 1], q[i + 3] ) );
                        i += 4;
                    }
                }
            }
        }
        else
        {
            fixed.push_str( line );
            fixed.push( '\n' );
        }
    }

    fixed
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
        ExtractTarget::Single =>
        {
            fs::write( out, all_ents.as_bytes() )?;
            Ok( () )
        }
        ExtractTarget::Split =>// Split entities into point and brush entities into separate files
        {
            let( mut point_ents, mut brush_ents ) = ( String::new(), String::new() );

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
    let ent_txt = 
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

    let ent_txt = normalise_entities( &ent_txt );
    let mut entities: Vec<_> = parse_entity_blocks( &ent_txt )
        .into_iter().map( |( _, d )| d ).collect();

    let model_indices = bsp.slice_lump( LumpIdx::Models ).len() / 64;
    entities.retain( |ent|
    {
        let Some( m ) = ent.get( "model" ) else { return true };
        let Some( s ) = m.strip_prefix( '*' ) else { return true };
        let Ok( idx ) = s.parse::<usize>() else { return true };

        if idx == 0 || idx < model_indices
        {
            return true;
        }

        let cn = ent.get( "classname" ).map( |s| s.as_str() ).unwrap_or( "<unknown>" );
        eprintln!( "  [warning] entity \"{cn}\" references non-existent brush model *{idx}, discarding" );

        false
    });

    let mut out = serialize_entities( &entities );

    if !out.ends_with( '\0' )
    {
        out.push( '\0' );
    }

    bsp.replace_lump( LumpIdx::Entities, out.as_bytes() )?;

    Ok( bsp )
}

pub fn repair(bsporent_path: &Path) -> Result<()>
{
    match bsporent_path.extension().and_then( |ext| ext.to_str() )
    {
        Some( EXT_BSP ) =>
        {
            let bsp = BspFile::load( bsporent_path )?;
            let text = str::from_utf8( bsp.slice_lump( LumpIdx::Entities ) )?;
            import( bsp.clone(), ImportSource::Text( normalise_entities( text ) ) )?.save()?;
            println!( "Repaired entities → {bsporent_path:?}" );

            Ok( () )
        }

        Some( EXT_ENT ) | Some( EXT_POINT_ENT ) | Some( EXT_BRUSH_ENT ) =>
        {
            let text = fs::read_to_string( bsporent_path )?;
            fs::write( bsporent_path, normalise_entities( &text ).as_bytes() )?;
            println!( "Repaired entities → {bsporent_path:?}" );

            Ok( () )
        }

        Some( s ) => bail!( "Unsupported file type '{s}'. Requires ENT/BSP" ),
        None => bail!( "Unsupported file type. Requires ENT/BSP" )
    }
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

        Some( s ) => bail!( "Invalid file type '{s}'. Requires ENT/BSP" ),
        None => bail!( "Invalid file type. Requires ENT/BSP" )
    }
}

